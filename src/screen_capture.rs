use crate::audio_capture::AudioCapture;
use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use scrap::{Capturer, Display};
use std::sync::mpsc;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub rgba_data: Vec<u8>,
}

impl CapturedFrame {
    fn from_bgra(width: u32, height: u32, mut bgra_data: Vec<u8>) -> Self {
        // Convert BGRA to RGBA immediately
        for chunk in bgra_data.chunks_exact_mut(4) {
            chunk.swap(0, 2); // Swap B and R
        }
        Self {
            width,
            height,
            rgba_data: bgra_data,
        }
    }

    pub fn view(&self, x: u32, y: u32, view_width: u32, view_height: u32) -> Option<Self> {
        if x + view_width > self.width || y + view_height > self.height {
            log::error!("View dimensions out of bounds.");
            return None;
        }

        let mut cropped_data = Vec::with_capacity((view_width * view_height * 4) as usize);
        let stride = self.width as usize * 4;
        let start_y = y as usize * stride;
        let start_x = x as usize * 4;

        for row in 0..view_height as usize {
            let row_start = start_y + row * stride + start_x;
            let row_end = row_start + (view_width as usize * 4);
            cropped_data.extend_from_slice(&self.rgba_data[row_start..row_end]);
        }

        Some(Self {
            width: view_width,
            height: view_height,
            rgba_data: cropped_data,
        })
    }
}

pub struct FrameGrabber {
    config: Arc<Mutex<Config>>, // Reference to shared config
    monitors: Vec<String>,
    capturer: Option<Capturer>,
    audio_capture: Option<AudioCapture>,
    audio_receiver: Option<mpsc::Receiver<Vec<f32>>>,
    width: u32,
    height: u32,
    stride: usize,
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
        let (audio_capture, audio_receiver) = AudioCapture::new();

        Self {
            config,
            monitors: monitors_list,
            capturer: None,
            width: 0,
            height: 0,
            stride: 0,
            audio_capture: Some(audio_capture),
            audio_receiver: Some(audio_receiver),
        }
    }

    pub fn get_monitor_index(&self) -> usize {
        self.config.lock().unwrap().capture.selected_monitor
    }

    pub fn set_monitor_index(&mut self, index: usize) {
        {
            let mut config = self.config.lock().unwrap();
            config.capture.selected_monitor = index;
        }
        self.reset_capture();
    }

    pub fn get_config(&self) -> Arc<Mutex<Config>> {
        Arc::clone(&self.config)
    }

    pub fn reset_capture(&mut self) {
        self.capturer = None;
        self.width = 0;
        self.height = 0;
    }

    pub fn get_monitors(&self) -> &Vec<String> {
        &self.monitors
    }

    pub fn capture_frame_with_audio(&mut self) -> Option<(Option<CapturedFrame>, Vec<f32>)> {
        let frame = self.capture_frame();
        let audio = self
            .audio_receiver
            .as_ref()
            .and_then(|rx| rx.try_recv().ok())
            .unwrap_or_default();

        Some((frame, audio))
    }

    pub fn capture_frame(&mut self) -> Option<CapturedFrame> {
        if self.capturer.is_none() {
            let monitor =
                get_monitor_from_index(self.config.lock().unwrap().capture.selected_monitor).unwrap();
            self.height = monitor.height() as u32;
            self.width = monitor.width() as u32;
            self.stride = monitor.width() as usize * 4; // Basic stride calculation
            if self.stride % 16 != 0 {
                // Align to 16 bytes
                self.stride = (self.stride + 15) & !15;
            }
            log::debug!(
                "Monitor dimensions: {}x{}, stride: {}",
                self.width,
                self.height,
                self.stride
            );
            self.capturer = Some(Capturer::new(monitor).expect("Couldn't begin capture."));
        }

        let capturer: &mut Capturer = self.capturer.as_mut().unwrap();
        match capturer.frame() {
            Ok(raw_frame) => {
                // Create new buffer with correct size
                let mut proper_frame = Vec::with_capacity((self.width * self.height * 4) as usize);

                // Copy each row, skipping the stride padding
                for y in 0..self.height as usize {
                    let start = y * self.stride;
                    let end = start + (self.width as usize * 4);
                    proper_frame.extend_from_slice(&raw_frame[start..end]);
                }

                Some(CapturedFrame::from_bgra(
                    self.width,
                    self.height,
                    proper_frame,
                ))
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock => {
                    log::debug!("Frame not ready; skipping this frame.");
                    None
                }
                std::io::ErrorKind::ConnectionReset => {
                    log::error!(
                        r"Strange Error: {e}.
                        Resetting capturer.
                        Make sure that if you changed your screen size, keep it at 16:9 ratio."
                    );
                    self.reset_capture();
                    None
                }
                _ => {
                    log::error!("{e:?}");
                    None
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
