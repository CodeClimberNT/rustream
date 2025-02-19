use super::{CaptureArea, CapturedFrame};
use crate::config::Config;
use scrap::{Capturer, Display};
use std::sync::{Arc, Mutex};

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

    pub fn next_frame(&mut self, capture_area: Option<CaptureArea>) -> Option<CapturedFrame> {
        if self.capturer.is_none() {
            let monitor = match get_monitor_from_index(
                self.config.lock().unwrap().capture.selected_monitor,
            ) {
                Ok(m) => m,
                Err(_) => return None,
            };

            self.height = monitor.height();
            self.width = monitor.width();
            log::debug!("Monitor dimensions: {}x{}", self.width, self.height);

            self.capturer = match Capturer::new(monitor) {
                Ok(cap) => Some(cap),
                Err(e) => {
                    log::error!("{}", CaptureError::InitError(e.to_string()));
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
                    log::debug!("Frame not ready; skipping this frame.");
                    None
                }
                _ => {
                    log::error!(
                        r"Strange Error: {e}.
                        Resetting capturer."
                    );
                    self.reset_capture();
                    None
                }
            },
        }
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
