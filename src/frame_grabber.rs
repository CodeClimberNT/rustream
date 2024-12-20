use crate::{common::CaptureArea, config::Config};
use image::{ImageBuffer, RgbaImage};

use std::sync::{Arc, Mutex};

use scrap::{Capturer, Display};

type RgbaBuffer = Vec<u8>;
type BgraBuffer = Vec<u8>;

#[derive(Debug, Default, Clone)]
pub struct CapturedFrame {
    pub width: usize,
    pub height: usize,
    pub rgba_data: Arc<RgbaImage>,
}

impl CapturedFrame {
    fn bgra_to_rgba(buffer_bgra: BgraBuffer, width: usize, height: usize) -> RgbaBuffer {
        // Calculate the stride (bytes per row)
        let stride = buffer_bgra.len() / height;

        // Preallocate the entire vector to avoid reallocations
        let mut rgba: Vec<u8> = vec![0u8; width * height * 4];

        // Process the buffer and write directly to `rgba_data`
        for y in 0..height {
            let row_start = y * stride;
            for x in 0..width {
                let i = row_start + x * 4;
                let target = (y * width + x) * 4;
                rgba[target] = buffer_bgra[i + 2]; // B to R
                rgba[target + 1] = buffer_bgra[i + 1]; // G remains the same
                rgba[target + 2] = buffer_bgra[i]; // R to B
                rgba[target + 3] = 255; // Alpha
            }
        }
        rgba
    }

    fn bgra_to_rgba_with_crop(
        buffer_bgra: BgraBuffer,
        buffer_width: usize,
        buffer_height: usize,
        crop_area: CaptureArea,
    ) -> (RgbaBuffer, usize, usize) {
        // Clamp and adjust the provided crop area
        let crop = CaptureArea::new_with_safeguards(
            crop_area.x,
            crop_area.y,
            crop_area.width,
            crop_area.height,
            buffer_width,
            buffer_height,
        );

        // Calculate the stride (bytes per row) of the source buffer
        let stride = buffer_width * 4;

        // Preallocate the cropped RGBA data (buffer for the final image)
        let mut rgba_data: RgbaBuffer = vec![0u8; crop.width * crop.height * 4];

        // Process the buffer and extract the cropped region
        for row in 0..crop.height {
            let src_start = ((crop.y + row) * stride) + (crop.x * 4);
            let dst_start = row * crop.width * 4;

            // Iterate over each pixel in the row
            for col in 0..crop.width {
                let src_idx = src_start + col * 4;
                let dst_idx = dst_start + col * 4;

                // Set the RGBA values in the destination buffer
                rgba_data[dst_idx] = buffer_bgra[src_idx + 2]; // B to R
                rgba_data[dst_idx + 1] = buffer_bgra[src_idx + 1]; // G remains the same
                rgba_data[dst_idx + 2] = buffer_bgra[src_idx]; // R to B
                rgba_data[dst_idx + 3] = 255; // Alpha
            }
        }
        (rgba_data, crop.width, crop.height)
    }

    fn from_bgra_buffer(
        buffer_bgra: BgraBuffer,
        buffer_width: usize,
        buffer_height: usize,
        capture_area: Option<CaptureArea>,
    ) -> Self {
        // Default to full buffer if crop_area is None
        let (rgba_data, final_width, final_height) = match capture_area {
            Some(crop) => {
                Self::bgra_to_rgba_with_crop(buffer_bgra, buffer_width, buffer_height, crop)
            }
            None => {
                let rgba_image = Self::bgra_to_rgba(buffer_bgra, buffer_width, buffer_height);
                (rgba_image, buffer_width, buffer_height)
            }
        };

        // Create an ImageBuffer from the processed data
        let image: RgbaImage =
            ImageBuffer::from_vec(final_width as u32, final_height as u32, rgba_data)
                .expect("Failed to create image from buffer");
        Self {
            width: final_width,
            height: final_height,
            rgba_data: Arc::new(image),
        }
    }
}

pub struct FrameGrabber {
    config: Arc<Mutex<Config>>, // Reference to shared config
    monitors: Vec<String>,
    capturer: Option<Capturer>,
    width: usize,
    height: usize,
    // stride: usize,
}

impl Default for FrameGrabber {
    fn default() -> Self {
        Self::new(Arc::new(Mutex::new(Config::default())))
    }
}

impl FrameGrabber {
    pub fn new(config: Arc<Mutex<Config>>) -> Self {
        let mut monitors_list: Vec<String> = Vec::new();
        if let Ok(displays) = get_monitors() {
            for (i, _monitor) in displays.iter().enumerate() {
                monitors_list.push(format!("Monitor {}", i));
            }
        }

        Self {
            config,
            monitors: monitors_list,
            capturer: None,
            width: 0,
            height: 0,
            // stride: 0,
        }
    }

    pub fn reset_capture(&mut self) {
        self.capturer = None;
        self.width = 0;
        self.height = 0;
    }

    pub fn get_monitors(&self) -> &Vec<String> {
        &self.monitors
    }

    pub fn capture_frame(
        &mut self,
        capture_area: Option<CaptureArea>,
    ) -> Result<CapturedFrame, std::io::Error> {
        if self.capturer.is_none() {
            let monitor: Display =
                get_monitor_from_index(self.config.lock().unwrap().capture.selected_monitor)
                    .unwrap();
            self.height = monitor.height();
            self.width = monitor.width();

            log::debug!("Monitor dimensions: {}x{}", self.width, self.height);
            self.capturer = Some(Capturer::new(monitor)?);
        }

        let capturer: &mut Capturer = self.capturer.as_mut().unwrap();
        match capturer.frame() {
            Ok(raw_frame) => Ok(CapturedFrame::from_bgra_buffer(
                raw_frame.to_vec(),
                self.width,
                self.height,
                capture_area,
            )),
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock => {
                    log::debug!("Frame not ready; skipping this frame.");
                    Err(e)
                }
                _ => {
                    log::error!(
                        r"Strange Error: {e}.
                        Resetting capturer."
                    );
                    self.reset_capture();
                    Err(e)
                }
            },
        }
    }
}

fn get_monitors() -> Result<Vec<Display>, ()> {
    let monitors: Vec<Display> = Display::all().expect("Couldn't find any display.");
    if monitors.is_empty() {
        return Err(());
    }

    Ok(monitors)
}

pub fn get_monitor_from_index(index: usize) -> Result<Display, ()> {
    let mut monitors = get_monitors().unwrap();

    if index >= monitors.len() {
        return Err(());
    }

    let monitor: Display = monitors.remove(index);
    Ok(monitor)
}
