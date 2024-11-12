use ffmpeg::{
    codec::{self},
    format::{self, output},
    software::scaling::{context::Context, flag::Flags},
    util::{format::Pixel, frame::video::Video},
};
use ffmpeg_next as ffmpeg;

pub struct Recorder {
    context: format::context::Output,
    stream_index: usize,
    encoder: ffmpeg::encoder::Video,
    scaler: Context,
}

impl Recorder {
    pub fn start_recording(output_path: &str) -> Self {
        ffmpeg::init().expect("Failed to initialize FFmpeg.");

        let mut context = output(&std::path::Path::new(output_path)).unwrap();
        let global_header = context
            .format()
            .flags()
            .contains(format::flag::Flags::GLOBAL_HEADER);
        let stream = context.add_stream(codec::Id::H264).unwrap();
        let stream_index = stream.index();

        let codec = ffmpeg::encoder::find(codec::Id::H264).unwrap();
        let mut encoder = codec::context::Context::from_parameters(stream.parameters())
            .unwrap()
            .encoder()
            .video()
            .unwrap();
        encoder.set_width(1280); // Set appropriate width
        encoder.set_height(720); // Set appropriate height
        encoder.set_format(Pixel::YUV420P);
        encoder.set_time_base((1, 30));
        if global_header {
            encoder.set_flags(codec::Flags::GLOBAL_HEADER);
        }
        let encoder = encoder.open_as(codec).unwrap();

        let scaler = Context::get(
            Pixel::RGBA,
            encoder.width(),
            encoder.height(),
            encoder.format(),
            encoder.width(),
            encoder.height(),
            Flags::BILINEAR,
        )
        .unwrap();

        Self {
            context,
            stream_index,
            encoder,
            scaler,
        }
    }

    pub fn record_frame(&mut self, data: &[u8], width: u32, height: u32) {
        let mut input = Video::empty();
        input.set_width(width as u32);
        input.set_height(height as u32);
        input.set_format(Pixel::RGBA);
        input
            .plane_mut::<[u8; 4]>(0)
            .copy_from_slice(bytemuck::cast_slice(data));

        let mut output = Video::empty();
        output.set_width(self.encoder.width());
        output.set_height(self.encoder.height());
        output.set_format(self.encoder.format());

        if let Err(e) = self.scaler.run(&input, &mut output) {
            eprintln!("Failed to scale frame: {}", e);
            return;
        }

        if let Err(e) = self.encoder.send_frame(&output) {
            eprintln!("Failed to send frame to encoder: {}", e);
            return;
        }

        let mut packet = ffmpeg::packet::Packet::empty();
        while self.encoder.receive_packet(&mut packet).is_ok() {
            packet.set_stream(self.stream_index);
            packet.write(&mut self.context).unwrap();
        }
    }

    pub fn stop_recording(&mut self) {
        if let Err(e) = self.encoder.send_eof() {
            eprintln!("Failed to send EOF to encoder: {}", e);
        }

        let mut packet = ffmpeg::packet::Packet::empty();
        while self.encoder.receive_packet(&mut packet).is_ok() {
            packet.set_stream(self.stream_index);
            packet.write(&mut self.context).unwrap();
        }

        self.context.write_trailer().unwrap();
    }
}

pub fn start_recording(output_path: &str) -> Recorder {
    Recorder::start_recording(output_path)
}

pub fn stop_recording(recorder: &mut Recorder) {
    recorder.stop_recording();
}
