use ffmpeg_next::{
    self, codec,
    format::Pixel,
    frame, init,
    software::scaling::{self, flag::Flags},
    Packet,
};
use log::{debug, info, warn};

// Ensure FFmpeg is initialized only once
// static INIT: Once = Once::new();

// /// Initialize FFmpeg safely
// fn init() -> Result<(), FFmpegError> {
//     let mut result = Ok(());
//     INIT.call_once(|| {
//         result = ffmpeg_next::init();
//     });
//     result
// }

/// Encode RGBA data to H265
pub fn encode_to_h265(
    width: usize,
    height: usize,
    rgba_data: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Initialize FFmpeg
    init()?;

    // Find a suitable H265 encoder
    let encoder_name = find_available_hevc_encoder()?;
    let encoder = match encoder_name.as_str() {
        "hevc" => codec::encoder::find(codec::Id::HEVC),
        "h264" => codec::encoder::find(codec::Id::H264),
        _ => codec::encoder::find_by_name(&encoder_name),
    }
    .ok_or_else(|| format!("Failed to find encoder: {}", encoder_name))?;

    // Create codec context for the encoder
    let context = codec::Context::new();

    // Create the video encoder
    let mut video_encoder = context.encoder().video()?;

    // Determine target pixel format.
    // If the encoder supports YUV420P, use it;
    // otherwise, fall back to the first supported pixel format.
    let target_pix_fmt = Pixel::YUV420P;

    // Configure encoder parameters
    video_encoder.set_width(width as u32);
    video_encoder.set_height(height as u32);
    video_encoder.set_format(target_pix_fmt);
    video_encoder.set_time_base((1, 30)); // 30 fps

    // Set quality options for streaming
    let mut opts = ffmpeg_next::Dictionary::new();
    opts.set("preset", "ultrafast");
    opts.set("tune", "zerolatency");

    // Open the encoder
    let mut video_encoder = video_encoder.open_as_with(encoder, opts)?;

    // Create source RGBA frame
    let mut rgba_frame = frame::Video::new(Pixel::RGBA, width as u32, height as u32);

    // Copy RGBA data into frame
    let stride = rgba_frame.stride(0);
    for y in 0..height {
        let src_idx = y * width * 4;
        let dst_idx = y * stride;

        // Check bounds
        if (src_idx + width * 4 <= rgba_data.len())
            && (dst_idx + width * 4 <= rgba_frame.data(0).len())
        {
            rgba_frame.data_mut(0)[dst_idx..dst_idx + width * 4]
                .copy_from_slice(&rgba_data[src_idx..src_idx + width * 4]);
        } else {
            return Err("Buffer bounds error when copying RGBA data".into());
        }
    }

    // Create destination frame for encoding using the target pixel format
    let mut dst_frame = frame::Video::new(target_pix_fmt, width as u32, height as u32);

    // Create scaler for RGBA to target pixel format conversion
    let mut scaler = scaling::Context::get(
        Pixel::RGBA,
        width as u32,
        height as u32,
        target_pix_fmt,
        width as u32,
        height as u32,
        Flags::BILINEAR,
    )?;

    // Convert RGBA to target pixel format
    scaler.run(&rgba_frame, &mut dst_frame)?;

    // Send frame to encoder
    video_encoder.send_frame(&dst_frame)?;
    video_encoder.send_eof()?;

    // Receive encoded packets
    let mut encoded_data = Vec::new();
    let mut packet = Packet::empty();

    while video_encoder.receive_packet(&mut packet).is_ok() {
        if let Some(data) = packet.data() {
            encoded_data.extend_from_slice(data);
        }
    }

    Ok(encoded_data)
}

/// Decode H265 data to RGBA
pub fn decode_from_h265(
    encoded_data: &[u8],
) -> Result<(Vec<u8>, usize, usize), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize FFmpeg
    init()?;

    // Find H265 decoder
    let decoder = codec::decoder::find(codec::Id::HEVC).ok_or("No HEVC decoder found")?;

    // Create decoder context
    let context = codec::Context::new();

    // Get the decoder context first
    let decoder_ctx = context.decoder();

    // Open the decoder with our specific codec
    let opened_decoder = decoder_ctx.open_as(decoder)?;

    // Now get the video decoder from the opened decoder
    let mut video_decoder = opened_decoder.video()?;

    // Create a packet for the encoded data
    let data_copy = encoded_data.to_vec();
    let mut packet = Packet::new(data_copy.len());

    // Copy our data into the packet's buffer
    if let Some(packet_data) = packet.data_mut() {
        if packet_data.len() >= data_copy.len() {
            packet_data[..data_copy.len()].copy_from_slice(&data_copy);
        } else {
            return Err("Packet buffer too small for encoded data".into());
        }
    } else {
        return Err("Failed to get mutable packet data buffer".into());
    }

    // Send packet to decoder
    video_decoder.send_packet(&packet)?;

    // Receive decoded frame
    let mut frame = frame::Video::empty();
    video_decoder.receive_frame(&mut frame)?;

    // Extract dimensions
    let width = frame.width() as usize;
    let height = frame.height() as usize;

    // Convert to RGBA if needed
    let rgba_data = if frame.format() == Pixel::RGBA {
        // Frame is already RGBA
        frame.data(0).to_vec()
    } else {
        // Create an RGBA destination frame
        let mut rgba_frame = frame::Video::new(Pixel::RGBA, width as u32, height as u32);

        // Create a scaler to convert from source format to RGBA
        let mut scaler = scaling::Context::get(
            frame.format(),
            width as u32,
            height as u32,
            Pixel::RGBA,
            width as u32,
            height as u32,
            Flags::BILINEAR,
        )?;

        // Convert the frame
        scaler.run(&frame, &mut rgba_frame)?;

        // Extract the RGBA data
        rgba_frame.data(0).to_vec()
    };

    Ok((rgba_data, width, height))
}

/// Helper function to find an available HEVC encoder with improved detection and fallbacks
fn find_available_hevc_encoder() -> Result<String, Box<dyn std::error::Error>> {
    // Try hardware encoders first, then software
    let hevc_encoders = [
        "hevc_nvenc",        // NVIDIA
        "hevc_qsv",          // Intel QuickSync
        "hevc_amf",          // AMD
        "hevc_vaapi",        // VA-API
        "hevc_videotoolbox", // macOS
        "libx265",           // Software (fallback)
    ];

    // H264 encoders as fallback (most systems support these)
    let h264_encoders = [
        "h264_nvenc",        // NVIDIA
        "h264_qsv",          // Intel QuickSync
        "h264_amf",          // AMD
        "h264_vaapi",        // VA-API
        "h264_videotoolbox", // macOS
        "libx264",           // Software (common)
    ];

    // Try HEVC encoders first
    debug!("Looking for HEVC encoders...");
    for name in hevc_encoders.iter() {
        if let Some(_enc) = codec::encoder::find_by_name(name) {
            info!("Found HEVC encoder: {}", name);
            return Ok(name.to_string());
        } else {
            warn!("Encoder {} not available", name);
        }
    }

    // Fall back to H264 encoders if no HEVC encoders are found
    debug!("No HEVC encoders found, trying H264 encoders...");
    for name in h264_encoders.iter() {
        if let Some(_enc) = codec::encoder::find_by_name(name) {
            info!("Found H264 encoder: {}", name);
            return Ok(name.to_string());
        } else {
            warn!("Encoder {} not available", name);
        }
    }

    // As a last resort, try the software encoder "libx265" explicitly instead of the generic "hevc",
    // to avoid accidentally selecting a hardware encoder (like hevc_d3d12va) that requires a hardware frames context.
    debug!("Trying default software encoder 'libx265'...");
    if let Some(_enc) = codec::encoder::find_by_name("libx265") {
        info!("Found software encoder libx265");
        return Ok("libx265".to_string());
    }

    Err(
        "No suitable video encoder found. Please ensure FFmpeg is installed with encoding support."
            .into(),
    )
}

// #[test]
// fn test_find_hevc_encoder() {
//     // Initialize FFmpeg
//     init().expect("Failed to initialize FFmpeg");

//     // Call the function to find an encoder
//     let encoder_result = find_available_hevc_encoder();

//     // Check that an encoder was found
//     assert!(encoder_result.is_ok(), "Should find at least one encoder");

//     // Print the selected encoder
//     let encoder = encoder_result.unwrap();
//     println!("Selected encoder: {}", encoder);

//     // Verify the encoder exists
//     let encoder_exists = match encoder.as_str() {
//         "hevc" => codec::encoder::find(codec::Id::HEVC).is_some(),
//         "h264" => codec::encoder::find(codec::Id::H264).is_some(),
//         _ => codec::encoder::find_by_name(&encoder).is_some(),
//     };

//     assert!(encoder_exists, "Selected encoder should exist");
// }
