use crate::{common::CaptureArea, config::Config};
use image::{ImageBuffer, RgbaImage};
use scap::{
    capturer::{Area, Capturer, Options, Point, Resolution, Size},
    frame::Frame,
};
use std::sync::{mpsc::Receiver, Arc, Mutex};

#[derive(Debug)]
pub enum RecorderCommand {
    Capture,
    Stop,
}

#[derive(Debug, Default, Clone)]
pub struct CapturedFrame {
    pub width: usize,
    pub height: usize,
    pub rgba_data: Arc<RgbaImage>,
}

impl CapturedFrame {
    fn from_scap_frame(frame: Frame) -> Option<Self> {
        let (width, height, rgba_data) = match frame {
            Frame::BGRA(bgra) => {
                let width = bgra.width as usize;
                let height = bgra.height as usize;
                let mut rgba_data = Vec::with_capacity(width * height * 4);

                // Convert BGRA to RGBA
                for chunk in bgra.data.chunks_exact(4) {
                    rgba_data.push(chunk[2]); // R (from B)
                    rgba_data.push(chunk[1]); // G (same)
                    rgba_data.push(chunk[0]); // B (from R)
                    rgba_data.push(chunk[3]); // A (same)
                }
                (width, height, rgba_data)
            }
            Frame::RGB(rgb) => {
                let width = rgb.width as usize;
                let height = rgb.height as usize;
                let mut rgba_data = Vec::with_capacity(width * height * 4);

                // Convert RGB to RGBA
                for chunk in rgb.data.chunks_exact(3) {
                    rgba_data.extend_from_slice(chunk); // Copy RGB as-is
                    rgba_data.push(255); // Add alpha channel
                }
                (width, height, rgba_data)
            }
            _ => {
                eprintln!("Unsupported frame format: {:?}", frame);
                return None;
            }
        };

        // Create image buffer outside the match for both cases
        let image = ImageBuffer::from_vec(width as u32, height as u32, rgba_data)
            .expect("Failed to create image buffer");

        Some(Self {
            width,
            height,
            rgba_data: Arc::new(image),
        })
    }
}

pub struct FrameGrabber {
    config: Arc<Mutex<Config>>,
    capturer: Option<Capturer>,
    frame_rx: Option<Receiver<Frame>>,
}

impl Default for FrameGrabber {
    fn default() -> Self {
        Self::new(Arc::new(Mutex::new(Config::default())))
    }
}

impl FrameGrabber {
    pub fn new(config: Arc<Mutex<Config>>) -> Self {
        Self {
            config,
            capturer: None,
            frame_rx: None,
        }
    }

    pub fn reset_capture(&mut self) {
        if let Some(mut capturer) = self.capturer.take() {
            capturer.stop_capture();
        }
        self.frame_rx = None;
    }

    fn create_capturer(
        &self,
        options: Options,
        crop_area: Option<CaptureArea>,
    ) -> Result<Capturer, Box<dyn std::error::Error>> {
        let config = self.config.lock().unwrap();

        let options = Options {
            fps: config.video.fps,
            show_cursor: true,
            show_highlight: false,
            excluded_targets: None,
            output_resolution: Resolution::_720p,
            crop_area: crop_area.map(|area| Area {
                origin: Point {
                    x: area.x as f64,
                    y: area.y as f64,
                },
                size: Size {
                    width: area.width as f64,
                    height: area.height as f64,
                },
            }),
            ..Default::default()
        };

        Capturer::build(options).map_err(Into::into)
    }

    pub fn capture_frame(
        &mut self,
        crop_area: Option<CaptureArea>,
    ) -> Result<CapturedFrame, Box<dyn std::error::Error>> {
        // Initialize capturer if needed
        if self.capturer.is_none() {
            if !scap::is_supported() {
                return Err("Platform not supported".into());
            }

            if !scap::has_permission() && !scap::request_permission() {
                return Err("Screen capture permission denied".into());
            }

            let mut capturer = self.create_capturer(crop_area)?;
            capturer.start_capture();
            self.capturer = Some(capturer);
        }

        // Capture frame
        if let Some(capturer) = &mut self.capturer {
            match capturer.get_next_frame() {
                Ok(frame) => CapturedFrame::from_scap_frame(frame)
                    .ok_or_else(|| "Failed to convert frame".into()),
                Err(e) => {
                    self.reset_capture();
                    Err(e.into())
                }
            }
        } else {
            Err("Capturer not initialized".into())
        }
    }
}
