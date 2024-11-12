use ffmpeg_next as ffmpeg;
use ffmpeg::{
    codec,
    format::{self, output},
    software::scaling::{context::Context, flag::Flags},
    util::{
        frame::video::Video,
        format::Pixel,
        rational::Rational,
        log::Level,
    },
};

pub struct Recorder {
    context: format::context::Output,
    stream_index: usize,
    encoder: codec::encoder::video::Video,
    scaler: Context,
}

impl Recorder {
    pub fn start_recording(output_path: &str) -> Self {
        ffmpeg::init().unwrap();

        let mut context = output(&std::path::Path::new(output_path)).unwrap();
        let mut stream = context.add_stream(codec::Id::H264).unwrap();
        let global_header = context
            .format()
            .flags()
            .contains(format::flag::Flags::GLOBAL_HEADER);

        let mut encoder = stream.codec_mut().encoder().video().unwrap();
        encoder.set_width(1280); // Set appropriate width
        encoder.set_height(720); // Set appropriate height
        encoder.set_format(Pixel::YUV420P);
        encoder.set_time_base(Rational::new(1, 30));
        if global_header {
            encoder.set_flags(codec::Flags::GLOBAL_HEADER);
        }
        encoder.open_as(codec::Id::H264).unwrap();

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

        context.write_header().unwrap();

        Self {
            context,
            stream_index: stream.index(),
            encoder,
            scaler,
        }
    }

    pub fn record_frame(&mut self, data: &[u8], width: u32, height: u32) {
        let mut input = Video::empty();
        input.set_width(width as i32);
        input.set_height(height as i32);
        input.set_format(Pixel::RGBA);
        input.plane_mut(0).unwrap().copy_from_slice(data);

        let mut output = Video::empty();
        output.set_width(self.encoder.width());
        output.set_height(self.encoder.height());
        output.set_format(self.encoder.format());

        self.scaler.run(&input, &mut output).unwrap();

        self.encoder.send_frame(&output).unwrap();

        let mut packet = ffmpeg::util::packet::Packet::empty();
        while self.encoder.receive_packet(&mut packet).is_ok() {
            packet.set_stream_index(self.stream_index);
            self.context.write_packet(&mut packet).unwrap();
        }
    }

    pub fn stop_recording(&mut self) {
        self.encoder.send_eof().unwrap();

        let mut packet = ffmpeg::util::packet::Packet::empty();
        while self.encoder.receive_packet(&mut packet).is_ok() {
            packet.set_stream_index(self.stream_index);
            self.context.write_packet(&mut packet).unwrap();
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
