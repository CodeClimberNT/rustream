use super::CapturedFrame;
use crate::config::Config;
use image::{ImageBuffer, RgbaImage};
use log::{debug, error};
use scrap::{Capturer, Display};
use std::{
    collections::VecDeque,
    sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex},
    thread,
};

#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("No monitors found")]
    NoMonitors,
    #[error("Invalid monitor index {0}")]
    InvalidIndex(usize),
    #[error("Failed to initialize capture: {0}")]
    InitError(String),
}

pub struct ScreenCapture {
    config: Arc<Mutex<Config>>,
    monitors: Vec<String>,
    capturer: Option<Capturer>,
    width: usize,
    height: usize,
    stop_capture: Arc<AtomicBool>,
}

impl Default for ScreenCapture {
    fn default() -> Self {
        Self::new(Arc::new(Mutex::new(Config::default())))
    }
}

impl ScreenCapture {
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
            stop_capture: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn reset_capture(&mut self) {
        self.capturer = None;
        self.width = 0;
        self.height = 0;
        self.stop_capture.store(true, Ordering::SeqCst); // Stop the capture thread
    }

    pub fn get_monitors(&self) -> &Vec<String> {
        &self.monitors
    }

    /*pub fn next_frame(&mut self, capture_area: Option<CaptureArea>) -> Option<CapturedFrame> {
        if self.capturer.is_none() {
            let monitor = match get_monitor_from_index(
                self.config.lock().unwrap().capture.selected_monitor,
            ) {
                Ok(m) => m,
                Err(_) => return None,
            };

            self.height = monitor.height();
            self.width = monitor.width();
            debug!("Monitor dimensions: {}x{}", self.width, self.height);
            debug!("Monitor dimensions: {}x{}", self.width, self.height);

            self.capturer = match Capturer::new(monitor) {
                Ok(cap) => Some(cap),
                Err(e) => {
                    error!("{}", CaptureError::InitError(e.to_string()));
                    return None;
                }
            };
        }

        let capturer = self.capturer.as_mut()?;

        match capturer.frame() {
            Ok(raw_frame) => Some(CapturedFrame::from_bgra_buffer(
                raw_frame.to_vec(),
                self.width,
                self.height,
                capture_area,
            )),
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock => {
                    debug!("Frame not ready; skipping this frame.");
                    debug!("Frame not ready; skipping this frame.");
                    None
                }
                _ => {
                    error!(
                        r"Strange Error: {e}.
                        Resetting capturer."
                    );
                    self.reset_capture();
                    None
                }
            },
        }
    }*/

    pub fn capture_frame(&self, captured_frames: Arc<Mutex<VecDeque<CapturedFrame>>>) {

        let config = self.config.clone();
        let stop_capture = self.stop_capture.clone();

        thread::spawn(move || {
            let mut current_monitor_index: Option<usize> = None;
            let mut capturer: Option<Capturer> = None;
            let mut current_dimensions = (0, 0);

            while !stop_capture.load(Ordering::SeqCst) {
                // Check if monitor selection changed
                let new_monitor_index = {
                    let conf_lock = config.lock().unwrap();
                    conf_lock.capture.selected_monitor
                };

                // Reinitialize capturer if monitor changed or not initialized
                if current_monitor_index != Some(new_monitor_index) || capturer.is_none() {
                    let monitor = match get_monitor_from_index(new_monitor_index) {
                        Ok(m) => m,
                        Err(_) => {
                            error!("Failed to get monitor with index {}", new_monitor_index);
                            thread::sleep(std::time::Duration::from_secs(1));
                            return;
                        }
                    };

                    let width = monitor.width() as u32;
                    let height = monitor.height() as u32;

                    debug!("Monitor dimensions: {}x{}", width, height,);

                    match Capturer::new(monitor) {
                        Ok(c) => {
                            capturer = Some(c);
                            current_monitor_index = Some(new_monitor_index);
                            current_dimensions = (width, height);
                        }
                        Err(e) => {
                            error!("{}", CaptureError::InitError(e.to_string()));
                            return;
                        }
                    };
                }

                // Capture frame using the current capturer
                if let Some(ref mut cap) = capturer {
                    match cap.frame() {
                        Ok(raw_frame) => {
                            let img_buffer: RgbaImage = ImageBuffer::from_raw(
                                current_dimensions.0,
                                current_dimensions.1,
                                raw_frame.to_vec(),
                            )
                            .expect("Couldn't create image buffer from raw frame");

                            let rgba_img = CapturedFrame::from_bgra(
                                current_dimensions.0,
                                current_dimensions.1,
                                img_buffer,
                            );

                            let mut frames = captured_frames.lock().unwrap();

                            frames.push_back(rgba_img);
                        }

                        Err(e) => match e.kind() {
                            std::io::ErrorKind::WouldBlock => {
                                debug!("Frame not ready; skipping this frame.");
                            }
                            std::io::ErrorKind::ConnectionReset => {
                                error!(
                                    r"Strange Error: {e}.
                                    Resetting capturer.
                                    Make sure that if you changed your screen size, keep it at 16:9 ratio."
                                );
                                capturer = None; // Force reinitialization on next iteration
                            }
                            _ => {
                                error!("What did just happen? {e:?}");
                                capturer = None; // Force reinitialization on next iteration
                            }
                        },
                    }
                }
                thread::sleep(std::time::Duration::from_millis(1000 / 6));
            }
            println!("Capture thread stopped");
        });
    }
}

fn get_monitors() -> Result<Vec<Display>, CaptureError> {
    let monitors: Vec<Display> = Display::all().map_err(|_| CaptureError::NoMonitors)?;
    if monitors.is_empty() {
        return Err(CaptureError::NoMonitors);
    }

    Ok(monitors)
}

pub fn get_monitor_from_index(index: usize) -> Result<Display, CaptureError> {
    let mut monitors: Vec<Display> = get_monitors().map_err(|_| CaptureError::NoMonitors)?;

    if index >= monitors.len() {
        return Err(CaptureError::InvalidIndex(index));
    }

    let monitor: Display = monitors.remove(index);
    Ok(monitor)
}
