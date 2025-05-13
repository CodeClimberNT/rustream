use ffmpeg_next::{
    self, codec,
    format::Pixel,
    frame, init,
    software::scaling::{self, flag::Flags},
    Dictionary, Packet,
};
use log::debug;

/// Encode RGBA data (raw video) to H265 using libx265 like the CLI command.
/// Adds extra sanity checks to help debug memory or indexing issues.
pub fn encode_to_h265(
    width: usize,
    height: usize,
    rgba_data: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Check that input buffer has enough data.
    let expected_len = width * height * 4;

    assert_eq!(
        rgba_data.len(),
        expected_len,
        "Input buffer length mismatch"
    );

    init()?;

    // Force use of the software encoder "libx265" to match the CLI.
    let encoder_name = "libx265";
    let encoder = codec::encoder::find_by_name(encoder_name)
        .ok_or_else(|| format!("Failed to find encoder: {}", encoder_name))?;

    let context = codec::Context::new();
    let mut video_encoder = context.encoder().video()?;

    // libx265 accepts YUV420P so we convert RGBA to YUV420P.
    let target_pix_fmt = Pixel::YUV420P;

    video_encoder.set_width(width as u32);
    video_encoder.set_height(height as u32);
    video_encoder.set_format(target_pix_fmt);
    video_encoder.set_time_base((1, 30));

    video_encoder.set_bit_rate(128_000);
    video_encoder.set_gop(12);

    let mut opts = Dictionary::new();
    opts.set("preset", "ultrafast");

    let mut video_encoder = video_encoder.open_as_with(encoder, opts)?;

    // Allocate and initialize the source frame
    let mut rgba_frame = frame::Video::new(Pixel::RGBA, width as u32, height as u32);

    let stride = rgba_frame.stride(0);

    let frame_data_len = rgba_frame.data(0).len();

    if stride < width * 4 {
        return Err(format!(
            "Allocated stride ({}) is less than expected row size ({}).",
            stride,
            width * 4
        )
        .into());
    }

    for y in 0..height {
        let src_idx = y * width * 4;
        let dst_idx = y * stride;
        // Check that we do not go out-of-bound on the destination buffer.
        if (dst_idx + width * 4) > frame_data_len {
            return Err(format!(
                "Row {}: destination slice out-of-bound: {} + {} > {}",
                y,
                dst_idx,
                width * 4,
                frame_data_len
            )
            .into());
        }
        rgba_frame.data_mut(0)[dst_idx..dst_idx + width * 4]
            .copy_from_slice(&rgba_data[src_idx..src_idx + width * 4]);
    }

    // Allocate the destination frame for YUV420P.
    let mut dst_frame = frame::Video::new(target_pix_fmt, width as u32, height as u32);
    // unsafe {
    //     // Pass the pixel format, width, and height as arguments.
    //     dst_frame.alloc(target_pix_fmt, width as u32, height as u32);
    // }


    let mut scaler = scaling::Context::get(
        Pixel::RGBA,
        width as u32,
        height as u32,
        target_pix_fmt,
        width as u32,
        height as u32,
        Flags::BILINEAR,
    )?;

    scaler.run(&rgba_frame, &mut dst_frame)?;

    video_encoder.send_frame(&dst_frame)?;
    debug!("before send");
    video_encoder.send_eof()?;
    debug!("after send eof, before receive packet");

    let mut encoded_data = Vec::new();
    let mut packet = Packet::empty();
    while video_encoder.receive_packet(&mut packet).is_ok() {
        if let Some(data) = packet.data() {
            encoded_data.extend_from_slice(data);
        }
    }

    Ok(encoded_data)
}

/// Decode H265 data (from pipe input in CLI) to raw RGBA as in CLI decoder:
///   - Input: -f hevc
///   - Decoder: -c:v rawvideo, -preset ultrafast, -pix_fmt rgba
///   - Output: rawvideo (RGBA)
pub fn decode_from_h265(
    encoded_data: &[u8],
) -> Result<(Vec<u8>, usize, usize), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize FFmpeg
    init()?;

    // Use the HEVC decoder.
    let decoder = codec::decoder::find(codec::Id::HEVC).ok_or("No HEVC decoder found")?;
    let context = codec::Context::new();
    let decoder_ctx = context.decoder();
    let opened_decoder = decoder_ctx.open_as(decoder)?;
    let mut video_decoder = opened_decoder.video()?;

    // Create a packet and fill it with the encoded H265 data.
    let data_copy = encoded_data.to_vec();
    let mut packet = Packet::new(data_copy.len());
    if let Some(packet_data) = packet.data_mut() {
        if packet_data.len() >= data_copy.len() {
            packet_data[..data_copy.len()].copy_from_slice(&data_copy);
        } else {
            return Err("Packet buffer too small for encoded data".into());
        }
    } else {
        return Err("Failed to get mutable packet data buffer".into());
    }

    // Send the packet to the decoder and then receive a decoded frame.
    video_decoder.send_packet(&packet)?;
    let mut frame = frame::Video::empty();
    video_decoder.receive_frame(&mut frame)?;

    let width = frame.width() as usize;
    let height = frame.height() as usize;

    // If the decoded frame is not already in RGBA format, convert it.
    let rgba_data = if frame.format() == Pixel::RGBA {
        frame.data(0).to_vec()
    } else {
        let mut rgba_frame = frame::Video::new(Pixel::RGBA, width as u32, height as u32);
        let mut scaler = scaling::Context::get(
            frame.format(),
            width as u32,
            height as u32,
            Pixel::RGBA,
            width as u32,
            height as u32,
            Flags::BILINEAR,
        )?;
        scaler.run(&frame, &mut rgba_frame)?;
        rgba_frame.data(0).to_vec()
    };

    Ok((rgba_data, width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Check if a specific encoder is available
    pub fn is_encoder_available(name: &str) -> bool {
        codec::encoder::find_by_name(name).is_some()
    }

    #[test]
    fn test_encoder_availability() {
        init().unwrap();
        // Common encoders to check - some may not be available depending on build
        let encoder_names = vec![
            "libx264",
            "libx265",
            "h264_nvenc",
            "hevc_nvenc",
            "h264_amf",
            "hevc_amf",
            "h264_vaapi",
            "hevc_vaapi",
            "libvpx",
            "libvpx-vp9",
            "libaom-av1",
        ];

        println!("Encoder availability:");
        for name in encoder_names {
            let available = is_encoder_available(name);
            println!(
                "  {} - {}",
                name,
                if available {
                    "AVAILABLE"
                } else {
                    "NOT AVAILABLE"
                }
            );
        }
    }
}
