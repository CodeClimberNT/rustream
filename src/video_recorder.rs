use crate::config::{Config, VideoConfig};
use crate::frame_grabber::CapturedFrame;

use std::time::{Duration, Instant};
use std::{
    path::PathBuf,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

use hound::{WavSpec, WavWriter};
use image::RgbaImage;

pub struct VideoRecorder {
    config: Arc<Mutex<Config>>,
    is_recording: Arc<AtomicBool>,
    is_finalizing: Arc<AtomicBool>,
    frame_counter: u32,
    audio_buffer: Vec<f32>,
    audio_file: Option<PathBuf>,
    frame_writer_handle: Option<JoinHandle<()>>,
    start_time: Option<Instant>,
    frame_sender: Option<mpsc::Sender<(Arc<RgbaImage>, PathBuf)>>,
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
            is_finalizing: Arc::new(AtomicBool::new(false)),
            frame_writer_handle: None,
            frame_counter: 0,
            audio_buffer: Vec::new(),
            audio_file: None,
            frame_sender: None,
            start_time: None,
        }
    }

    fn cleanup(&mut self) {
        self.audio_buffer.clear();
        self.audio_file = None;
        self.frame_counter = 0;
        self.start_time = Some(Instant::now());
        self.is_finalizing.store(false, Ordering::SeqCst);
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

        // Create channel for frame writing
        let (tx, rx) = mpsc::channel::<(Arc<RgbaImage>, PathBuf)>();
        self.frame_sender = Some(tx);

        // Spawn single background thread for writing frames
        let writer_handle = thread::spawn(move || {
            while let Ok((image, path)) = rx.recv() {
                if let Err(e) = image.save(&path) {
                    log::error!("Failed to save frame: {}", e);
                }
            }
        });

        self.cleanup();
        self.frame_writer_handle = Some(writer_handle);
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

        // Send frame to background thread through channel
        if let Some(sender) = &self.frame_sender {
            if let Err(e) = sender.send((frame.rgba_data.clone(), frame_path)) {
                log::error!("Failed to send frame to writer thread: {}", e);
            }
        }
    }

    pub fn record_audio(&mut self, audio_data: &[f32]) {
        if !self.is_recording.load(Ordering::SeqCst) {
            return;
        }
        self.audio_buffer.extend_from_slice(audio_data);
    }

    pub fn stop(&mut self) -> bool {
        if !self.is_recording.load(Ordering::SeqCst) {
            return false;
        }

        self.is_recording.store(false, Ordering::SeqCst);
        self.is_finalizing.store(true, Ordering::SeqCst);

        self.frame_sender.take();
        log::info!("Recording stopped, waiting for pending frames...");

        let elapsed_time: Duration = self.start_time.unwrap_or_else(Instant::now).elapsed();
        let total_seconds: f64 = elapsed_time.as_secs_f64();
        let actual_fps: u32 = if total_seconds > 0.0 {
            (self.frame_counter as f64 / total_seconds).round() as u32
        } else {
            self.config.lock().unwrap().video.fps
        };

        log::info!(
            "Recording stopped. Duration: {:.2} seconds, Frames: {}, Calculated FPS: {}",
            total_seconds,
            self.frame_counter,
            actual_fps
        );

        let writer_handle = self.frame_writer_handle.take();
        let config = self.config.clone();
        let is_finalizing = self.is_finalizing.clone();
        let audio_file = self.audio_file.clone();

        std::thread::spawn(move || {
            // Wait for frame writer to finish
            if let Some(handle) = writer_handle {
                let _ = handle.join();
            }
            let config = config.lock().unwrap().video.clone();

            VideoRecorder::run_ffmpeg_command(&config, audio_file, &actual_fps);
            std::fs::remove_dir_all(&config.temp_dir).expect("Failed to clean temp directory");
            is_finalizing.store(false, Ordering::SeqCst);
        });

        true
    }

    fn run_ffmpeg_command(config: &VideoConfig, audio_file: Option<PathBuf>, fps: &u32) {
        log::info!("Generating video...");
        let mut command = Command::new("ffmpeg");
        log::info!("Temp dir: {:?}", config.temp_dir);
        command
            .arg("-y")
            .arg("-hwaccel")
            .arg("auto")
            .arg("-framerate")
            .arg(fps.to_string())
            .arg("-i")
            .arg(config.temp_dir.join("frame_%06d.png"))
            .arg("-vf")
            .arg(format!("fps={}", fps))
            .arg("-c:v") // Video encoder
            .arg("libx264") // Try NVIDIA encoder first
            .arg("-movflags")
            .arg("+faststart") // Enable fast start
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg("-preset")
            .arg("medium") // Encoding speed
            .arg("-tune")
            .arg("zerolatency")
            .arg("-crf")
            .arg("23");

        // Add audio if available
        if let Some(audio_file) = &audio_file {
            log::debug!("Audio was Available for the video");
            command
                .arg("-i")
                .arg(audio_file)
                .arg("-ac") // Number of audio channels
                .arg("1")
                .arg("-acodec")
                .arg("aac")
                .arg("-b:a")
                .arg("192k");
        } else {
            log::warn!("Audio was not available for the video");
        }

        // Set output path
        command.arg(&config.output_path);

        match command.output() {
            Ok(_) => log::info!("Video generated successfully"),

            Err(e) => log::error!("FFmpeg execution failed: {}", e),
        }
    }

    pub fn is_finalizing(&self) -> bool {
        self.is_finalizing.load(Ordering::SeqCst)
    }

    fn save_audio_to_wav(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let spec = WavSpec {
            channels: 1,
            sample_rate: 48000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let mut writer = WavWriter::create(path, spec)?;
        for sample in &self.audio_buffer {
            writer.write_sample(*sample)?;
        }
        writer.finalize()?;
        Ok(())
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
