use ffmpeg::codec;
use ffmpeg::format::output;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;
use ffmpeg_next as ffmpeg;
use ffmpeg::util::format;
use ffmpeg::util::media;

// ...existing code...

pub struct Recorder {
    context: ffmpeg::format::context::Output,
    stream: ffmpeg::format::stream::StreamMut<'static>,
    scaler: Context,
    frame: Video,
}

impl Recorder {
    pub fn start_recording(output_path: &str) -> Self {
        ffmpeg::init().unwrap();
        let mut context = output(&std::path::Path::new(output_path)).unwrap();
        let global = context
            .format()
            .flags()
            .contains(ffmpeg::format::flag::Flags::GLOBAL_HEADER);

        let mut stream = context.add_stream(codec::Id::H264).unwrap();
        let encoder = stream.codec_mut().encoder().video().unwrap();

        // Configure encoder settings
        // ...encoder configuration code...

        let scaler = Context::get(
            encoder.format(),
            encoder.width(),
            encoder.height(),
            encoder.format(),
            encoder.width(),
            encoder.height(),
            Flags::BILINEAR,
        )
        .unwrap();

        let frame = Video::empty();

        context.set_metadata(ffmpeg::Dictionary::new());
        context.write_header().unwrap();

        Self {
            context,
            stream,
            scaler,
            frame,
        }
    }

    pub fn record_frame(&mut self, data: &[u8], width: u32, height: u32) {
        // Create a frame from raw data
        let mut frame = Video::empty();
        frame.set_format(format::Pixel::RGBA);
        frame.set_width(width as i32);
        frame.set_height(height as i32);
        frame.fill_with(data);

        // Scale frame if necessary
        let mut scaled_frame = Video::empty();
        scaled_frame.set_format(self.stream.codec().format());
        scaled_frame.set_width(self.stream.codec().width());
        scaled_frame.set_height(self.stream.codec().height());
        self.scaler.run(&frame, &mut scaled_frame).unwrap();

        // Encode the frame
        let mut packet = ffmpeg::Packet::empty();
        let encoder = self.stream.codec_mut().encoder();
        if encoder.send_frame(&scaled_frame).is_ok() {
            while encoder.receive_packet(&mut packet).is_ok() {
                packet.set_stream_index(self.stream.index());
                self.context.write_packet(&packet).unwrap();
            }
        }
    }

    pub fn stop_recording(&mut self) {
        self.context.write_trailer().unwrap();
    }
}

pub fn start_recording(output_path: &str) -> Recorder {
    Recorder::start_recording(output_path)
}

pub fn stop_recording(recorder: &mut Recorder) {
    recorder.stop_recording();
}

// ...existing code...
