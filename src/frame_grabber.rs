use crate::audio_capture::AudioCapture;
use crate::config::Config;
use image::{GenericImageView, ImageBuffer, RgbaImage};
//use scrap::dxgi::Capturer;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::process::{Command, Stdio};
use std::io::Write;
use scrap::{Capturer, Display};
use std::sync::mpsc;
use std::{env, thread};
use std::collections::VecDeque;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub rgba_data: Vec<u8>,
}

impl CapturedFrame {
    fn from_bgra(width: u32, height: u32, mut bgra_buffer: RgbaImage) -> Self {
        // Convert BGRA to RGBA immediately
        for chunk in bgra_buffer.chunks_exact_mut(4) {
            chunk.swap(0, 2); // Swap B and R
        }
        Self {
            width,
            height,
            rgba_data: bgra_buffer.to_vec(),
        }
    }

    pub fn view(self, x: u32, y: u32, view_width: u32, view_height: u32) -> Option<Self> {
        let image_view: RgbaImage = ImageBuffer::from_vec(self.width, self.height, self.rgba_data)
            .expect("Couldn't create image buffer from raw frame");

        let cropped_image: Vec<u8> = image_view
            .view(x, y, view_width, view_height)
            .to_image()
            .to_vec();

        Some(Self {
            width: view_width,
            height: view_height,
            rgba_data: cropped_image,
        })
    }

    pub fn encode_to_h265(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {

        let mut ffmpeg = Command::new("ffmpeg")
            .args([
                //gpu_acceleration[0], gpu_acceleration[1],
                "-f", "rawvideo", // input is raw video
                "-pixel_format", "rgba",
                "-video_size", &format!("{}x{}", self.width, self.height),
                "-i", "-", // input from stdin
                "-c:v", "libx265", // Codec H.265
                //encoder[0], encoder[1],
                "-preset", "ultrafast",
                "-f", "rawvideo", // output raw
                "-", // output to stdout
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()) // Stdio::null() Ignora errori di ffmpeg per semplicità
            .spawn()?;

        // write RGBA data in stdin
        ffmpeg.stdin.as_mut().unwrap().write_all(&self.rgba_data)?;

        // read H.264 encoded data from stdout
        let output = ffmpeg.wait_with_output()?;
        if !output.status.success() {
            return Err("FFmpeg encoding failed".into());
        }

        Ok(output.stdout)
    }
}

pub struct FrameGrabber {
    config: Arc<Mutex<Config>>, // Reference to shared config
    monitors: Vec<String>,
    capturer: Option<Capturer>,
    //audio_capture: Option<AudioCapture>,
    audio_receiver: Option<mpsc::Receiver<Vec<f32>>>,
    width: u32,
    height: u32,
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
        let (audio_capture, audio_receiver) = AudioCapture::new();

        Self {
            config,
            monitors: monitors_list,
            capturer: None,
            width: 0,
            height: 0,
            // stride: 0,
            //audio_capture: Some(audio_capture),
            audio_receiver: Some(audio_receiver),
        }
    }

    pub fn reset_capture(&mut self) {
        //self.capturer = None;
        self.width = 0;
        self.height = 0;
    }

    pub fn get_monitors(&self) -> &Vec<String> {
        &self.monitors
    }

    /*pub fn capture_frame_with_audio(&mut self) -> Option<(Option<CapturedFrame>, Vec<f32>)> {
        let frame = self.capture_frame();
        let audio = self
            .audio_receiver
            .as_ref()
            .and_then(|rx| rx.try_recv().ok())
            .unwrap_or_default();

        Some((frame, audio))
    }*/

    pub fn capture_frame(&self, captured_frames: Arc<Mutex<VecDeque<CapturedFrame>>>) { //-> Option<CapturedFrame>  tx: mpsc::SyncSender<CapturedFrame>
        
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

                    log::debug!(
                        "Monitor dimensions: {}x{}",
                        width,
                        height,
                    );

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
                            let img_buffer: RgbaImage =
                                ImageBuffer::from_raw(current_dimensions.0, current_dimensions.1, raw_frame.to_vec())
                                    .expect("Couldn't create image buffer from raw frame");

                            let rgba_img = CapturedFrame::from_bgra(
                                current_dimensions.0, 
                                current_dimensions.1, 
                                img_buffer
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
                thread::sleep(std::time::Duration::from_millis(1000/40));
            }
        });
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
