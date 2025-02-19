use super::{CaptureArea, CapturedFrame};
use crate::config::Config;
use image::{ImageBuffer, RgbaImage};
use scrap::{Capturer, Display};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
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

    pub fn capture_frame(
        &self,
        captured_frames: Arc<Mutex<VecDeque<CapturedFrame>>>,
        capture_area: Option<CaptureArea>,
    ) {
        //-> Option<CapturedFrame>  tx: mpsc::SyncSender<CapturedFrame>

        //if self.capturer.is_none() {

        //}
        //let capturer = self.capturer.as_ref().unwrap();
        /*let mut no_capturer;
        if self.capturer.is_none() {
            no_capturer = true;
        } else {
            no_capturer = false;
        }*/

        let config = self.config.clone();

        thread::spawn(move || {
            let mut current_monitor_index: Option<usize> = None;
            let mut capturer: Option<Capturer> = None;
            let mut current_dimensions = (0, 0);

            loop {
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
                            log::error!("Failed to get monitor with index {}", new_monitor_index);
                            thread::sleep(std::time::Duration::from_secs(1));
                            return;
                        }
                    };

                    let width = monitor.width() as u32;
                    let height = monitor.height() as u32;

                    log::debug!("Monitor dimensions: {}x{}", width, height,);

                    match Capturer::new(monitor) {
                        Ok(c) => {
                            capturer = Some(c);
                            current_monitor_index = Some(new_monitor_index);
                            current_dimensions = (width, height);
                        }
                        Err(e) => {
                            log::error!("Failed to initialize Capturer: {:?}", e);
                            thread::sleep(std::time::Duration::from_millis(100));
                            return;
                        }
                    };
                }

                // Capture frame using the current capturer
                if let Some(ref mut cap) = capturer {
                    match cap.frame() {
                        Ok(raw_frame) => {
                            // let img_buffer: RgbaImage = ImageBuffer::from_raw(
                            //     current_dimensions.0,
                            //     current_dimensions.1,
                            //     raw_frame.to_vec(),
                            // )
                            // .expect("Couldn't create image buffer from raw frame");

                            let rgba_img = CapturedFrame::from_bgra_buffer(
                                raw_frame.to_vec(),
                                current_dimensions.0 as usize,
                                current_dimensions.1 as usize,
                                capture_area,
                            );

                            let mut cap_frames = captured_frames.lock().unwrap();
                            cap_frames.push_back(rgba_img);
                            /*if let Err(e) = tx.send(rgba_img) {
                                println!("Failed to send captured frame: {}", e);
                            }*/
                        }
                        Err(e) => match e.kind() {
                            std::io::ErrorKind::WouldBlock => {
                                log::debug!("Frame not ready; skipping this frame.");
                            }
                            std::io::ErrorKind::ConnectionReset => {
                                log::error!(
                                    r"Strange Error: {e}.
                                    Resetting capturer.
                                    Make sure that if you changed your screen size, keep it at 16:9 ratio."
                                );
                                capturer = None; // Force reinitialization on next iteration
                            }
                            _ => {
                                log::error!("What did just happen? {e:?}");
                                capturer = None; // Force reinitialization on next iteration
                            }
                        },
                    }
                }

                // versione di prima
                /*if no_capturer{
                    let conf_lock = config.lock().unwrap();
                    let monitor =
                    get_monitor_from_index(conf_lock.capture.selected_monitor).unwrap();
                    drop(conf_lock);
                    let height = monitor.height() as u32;
                    let width = monitor.width() as u32;
                    //self.height = monitor.height() as u32;
                    //self.width = monitor.width() as u32;
                    // self.stride = monitor.width() as usize * 4; // Basic stride calculation
                    // if self.stride % 16 != 0 {
                    //     // Align to 16 bytes
                    //     self.stride = (self.stride + 15) & !15;
                    // }
                    log::debug!(
                        "Monitor dimensions: {}x{}",
                        width,
                        height,
                        // self.stride
                    );

                    let mut capturer = match Capturer::new(monitor) {
                        Ok(c) => c,
                        Err(e) => {
                            log::error!("Failed to initialize Capturer: {:?}, Error details: {:?}", e, e.to_string());
                            return;
                        }
                    };
                }
                    match capturer.frame() {
                        Ok(raw_frame) => {
                            // Create new buffer with correct size
                            // let mut img_buffer: ImageBuffer<_, Vec<_>> =
                            //     ImageBuffer::new(self.width, self.height);

                            // Copy each row, skipping the stride padding
                            // for y in 0..self.height as usize {
                            //     let start = y * self.stride;
                            //     let end = start + (self.width as usize * 4);
                            //     img_buffer.extend_from_slice(&raw_frame[start..end]);
                            // }

                            let img_buffer: RgbaImage =
                                ImageBuffer::from_raw(width, height, raw_frame.to_vec())
                                    .expect("Couldn't create image buffer from raw frame");

                            let rgba_img = CapturedFrame::from_bgra(width, height, img_buffer);

                            let mut cap_frames = captured_frames.lock().unwrap();
                            cap_frames.push_back(rgba_img);
                            //Some(rgba_img)
                        }
                        Err(e) => match e.kind() {
                            std::io::ErrorKind::WouldBlock => {
                                log::debug!("Frame not ready; skipping this frame.");
                                //None
                            }
                            std::io::ErrorKind::ConnectionReset => {
                                log::error!(
                                    r"Strange Error: {e}.
                                    Resetting capturer.
                                    Make sure that if you changed your screen size, keep it at 16:9 ratio."
                                );
                                //self.reset_capture(); //come lo implemento senza usare cose nel self?
                                //None
                            }
                            _ => {
                                log::error!("What did just happen? {e:?}");
                                //None
                            }
                        },
                    }*/
                thread::sleep(std::time::Duration::from_millis(1000 / 40));
            }
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
