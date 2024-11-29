use std::{
    path::PathBuf,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

use crate::config::Config;
use crate::screen_capture::CapturedFrame;
use image::{ImageBuffer, RgbaImage};

pub struct VideoRecorder {
    config: Arc<Mutex<Config>>,
    is_recording: Arc<AtomicBool>,
    frame_writer_handle: Option<JoinHandle<()>>,
    frame_counter: usize,
}

impl Default for VideoRecorder {
    fn default() -> Self {
        Self::new(Arc::new(Mutex::new(Config::default())))
    }
}

impl VideoRecorder {
    pub fn new(config: Arc<Mutex<Config>>) -> Self {
        let temp_dir = config.lock().unwrap().video.temp_dir.clone();
        std::fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");

        Self {
            config,
            is_recording: Arc::new(AtomicBool::new(false)),
            frame_writer_handle: None,
            frame_counter: 0,
        }
    }

    pub fn set_config(&mut self, config: Arc<Mutex<Config>>) {
        self.config = config;
    }

    pub fn start(&mut self) {
        if self.is_recording.load(Ordering::SeqCst) {
            return;
        }

        let temp_dir: PathBuf = self.config.lock().unwrap().video.temp_dir.clone();

        // Clear any existing frames
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).expect("Failed to clean temp directory");
        }
        std::fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");

        self.frame_counter = 0;
        self.is_recording.store(true, Ordering::SeqCst);
        log::info!("Recording started");
    }

    pub fn record_frame(&mut self, frame: &CapturedFrame) {
        if !self.is_recording.load(Ordering::SeqCst) {
            return;
        }

        let temp_dir: PathBuf = self.config.lock().unwrap().video.temp_dir.clone();
        let frame_path: PathBuf = temp_dir.join(format!("frame_{:06}.png", self.frame_counter));
        self.frame_counter += 1;

        // Convert the frame data to an image::ImageBuffer
        let img: RgbaImage =
            ImageBuffer::from_raw(frame.width, frame.height, frame.rgba_data.clone())
                .expect("Failed to create image buffer");

        // Spawn a thread to save the frame asynchronously
        let handle: JoinHandle<()> = thread::spawn(move || {
            if let Err(e) = img.save(&frame_path) {
                log::error!("Failed to save frame: {}", e);
            }
        });

        // We don't wait for the thread to finish - fire and forget
        handle.join().ok();
    }

    pub fn stop(&mut self) -> bool {
        if !self.is_recording.load(Ordering::SeqCst) {
            return false;
        }

        self.is_recording.store(false, Ordering::SeqCst);
        log::info!("Recording stopped, generating video...");

        // Wait for any remaining frame writes to complete
        if let Some(handle) = self.frame_writer_handle.take() {
            handle.join().ok();
        }

        // Generate the video using ffmpeg
        self.generate_video()
    }

    fn generate_video(&self) -> bool {
        let config = self.config.lock().unwrap().video.clone();
        let status = Command::new("ffmpeg")
            .arg("-y") // Overwrite output file if it exists
            .arg("-framerate")
            .arg(self.get_fps().to_string())
            .arg("-i")
            .arg(config.temp_dir.join("frame_%06d.png"))
            .arg("-c:v")
            .arg("libx264")
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg("-preset")
            .arg("medium")
            .arg("-crf")
            .arg("23")
            .arg(self.get_output_path())
            .status();

        match status {
            Ok(exit_status) if exit_status.success() => {
                let config = self.config.lock().unwrap().video.clone();

                log::info!(
                    "Video generated successfully at {:?}",
                    self.get_output_path()
                );
                // Clean up temp directory
                if let Err(e) = std::fs::remove_dir_all(&config.temp_dir) {
                    log::error!("Failed to clean up temp directory: {}", e);
                }
                true
            }
            Ok(_) => {
                log::error!("ffmpeg failed to generate video");
                false
            }
            Err(e) => {
                log::error!("Failed to run ffmpeg: {}", e);
                false
            }
        }
    }

    fn get_output_path(&self) -> PathBuf {
        self.config.lock().unwrap().video.output_path.clone()
    }

    fn get_fps(&self) -> u32 {
        self.config.lock().unwrap().video.fps
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }
}

impl Drop for VideoRecorder {
    fn drop(&mut self) {
        if self.is_recording() {
            self.stop();
        }
        let temp_dir = self.config.lock().unwrap().video.temp_dir.clone();
        // Clean up temp directory if it exists
        if temp_dir.exists() {
            if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
                log::error!("Failed to clean up temp directory on drop: {}", e);
            }
        }
    }
}
