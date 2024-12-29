use crate::config::{AudioConfig, Config, VideoConfig};
use crate::screen_capture::CapturedFrame;

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
    recording_start_time: Option<Instant>,
    audio_start_time: Option<Instant>,
    audio_file: Option<PathBuf>,
    frame_writer_handle: Option<JoinHandle<()>>,
    start_time: Option<Instant>,
    last_frame_time: Option<Instant>,
    target_frame_duration: Duration,
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

        let fps = config.lock().unwrap().video.fps;

        Self {
            config,
            is_recording: Arc::new(AtomicBool::new(false)),
            is_finalizing: Arc::new(AtomicBool::new(false)),
            frame_writer_handle: None,
            frame_counter: 0,
            recording_start_time: None,
            audio_start_time: None,
            audio_file: None,
            frame_sender: None,
            start_time: None,
            last_frame_time: None,
            target_frame_duration: Duration::from_secs_f64(1.0 / fps as f64),
        }
    }

    fn cleanup(&mut self) {
        self.audio_file = None;
        self.frame_counter = 0;
        self.start_time = Some(Instant::now());
        self.is_finalizing.store(false, Ordering::SeqCst);
    }

    pub fn start(&mut self) {
        if self.is_recording.load(Ordering::SeqCst) {
            return;
        }

        self.recording_start_time = Some(Instant::now());
        self.audio_start_time = Some(Instant::now());

        // Create temp directory if it doesn't exist
        let temp_dir = {
            let config = self.config.lock().unwrap();
            let temp_dir = config.video.temp_dir.clone();
            if !temp_dir.exists() {
                if let Err(e) = std::fs::create_dir_all(&temp_dir) {
                    log::error!("Failed to create temp directory: {}", e);
                    return;
                }
            }
            temp_dir
        };

        // Setup frame writer thread
        let (sender, receiver) = mpsc::channel();
        self.frame_sender = Some(sender);

        let handle = thread::spawn(move || {
            while let Ok((image_data, frame_path)) = receiver.recv() {
                match image_data.save(&frame_path) {
                    Ok(_) => (),
                    Err(e) => log::error!("Failed to save frame: {}", e),
                }
            }
        });
        self.frame_writer_handle = Some(handle);
        self.cleanup();
        self.is_recording.store(true, Ordering::SeqCst);
        log::info!("Started recording to {:?}", temp_dir);
    }

    pub fn record_frame(&mut self, frame: &CapturedFrame) {
        if !self.is_recording.load(Ordering::SeqCst) {
            return;
        }

        let now = Instant::now();

        // Initialize start time if this is the first frame
        if self.start_time.is_none() {
            self.start_time = Some(now);
            self.last_frame_time = Some(now);
        }

        // Check if enough time has passed since last frame
        if let Some(last_time) = self.last_frame_time {
            let elapsed = now.duration_since(last_time);
            if elapsed < self.target_frame_duration {
                return; // Skip frame if too soon
            }
            // Update last frame time based on target duration multiples
            let frames_to_skip =
                (elapsed.as_secs_f64() / self.target_frame_duration.as_secs_f64()).floor() as u32;
            self.last_frame_time = Some(last_time + self.target_frame_duration * frames_to_skip);
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
    pub fn stop(&mut self) -> bool {
        if !self.is_recording.load(Ordering::SeqCst) {
            return false;
        }

        let start_time = self.start_time;

        self.is_recording.store(false, Ordering::SeqCst);
        self.is_finalizing.store(true, Ordering::SeqCst);

        self.frame_sender.take();
        log::info!("Recording stopped, waiting for pending frames...");

        // Get necessary data before spawning thread
        let writer_handle = self.frame_writer_handle.take();
        let config = self.config.clone();
        let is_finalizing = self.is_finalizing.clone();
        let audio_file = self.audio_file.clone();
        let frame_counter = self.frame_counter;

        std::thread::spawn(move || {
            // Wait for frame writer to finish
            if let Some(handle) = writer_handle {
                let _ = handle.join();
            }

            let config_guard = config.lock().unwrap();
            let (video_config, audio_config) =
                (config_guard.video.clone(), config_guard.audio.clone());
            drop(config_guard); // Release lock early

            let fps = Self::calculate_fps(frame_counter, &video_config, &audio_file, start_time);
            log::info!(
                "Recording metrics - Frames: {}, Duration: {:.2}s, Calculated FPS: {}",
                frame_counter,
                start_time.map_or(0.0, |t| t.elapsed().as_secs_f64()),
                fps
            );

            VideoRecorder::run_ffmpeg_command(&video_config, &audio_config, audio_file, &fps);
            std::fs::remove_dir_all(&video_config.temp_dir)
                .expect("Failed to clean temp directory");
            is_finalizing.store(false, Ordering::SeqCst);
        });

        true
    }

    fn run_ffmpeg_command(
        video_config: &VideoConfig,
        audio_config: &AudioConfig,
        audio_file: Option<PathBuf>,
        fps: &u32,
    ) {
        let output_path = Self::generate_unique_path(video_config.output_path.clone());
        log::info!("Generating video...");

        let mut command = Command::new("ffmpeg");

        // Add verbose logging
        command.arg("-v").arg("debug").arg("-stats");

        // Input frames
        command
            .arg("-y")
            .arg("-hwaccel")
            .arg("auto")
            .arg("-f")
            .arg("image2")
            .arg("-framerate")
            .arg(fps.to_string())
            .arg("-i")
            .arg(video_config.temp_dir.join("frame_%06d.png"));

        // Add audio input BEFORE video encoding params
        if let Some(audio_path) = &audio_file {
            if audio_path.exists() {
                log::info!("Adding audio from: {:?}", audio_path);
                command
                    .arg("-i")
                    .arg(audio_path)
                    .arg("-map")
                    .arg("0:v") // First input video
                    .arg("-map")
                    .arg("1:a") // Second input audio
                    .arg("-async") // Force audio sync
                    .arg("1")
                    .arg("-af")
                    .arg("aresample=async=1000") // Ensure audio starts at 0
                    .arg("-vsync")
                    .arg("cfr"); // Set precise timestamps
            } else {
                log::warn!("Audio file not found at: {:?}", audio_path);
            }
        }

        // Video encoding parameters
        command
            .arg("-vf")
            .arg("scale=trunc(iw/2)*2:trunc(ih/2)*2") // Ensure even dimensions
            .arg("-c:v")
            .arg("libx264")
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg("-preset")
            .arg("medium")
            .arg("-crf")
            .arg("23")
            .arg("-r")
            .arg(fps.to_string()) // Output FPS
            .arg("-max_muxing_queue_size")
            .arg("1024");

        // Audio encoding parameters (when audio present)
        if audio_file.is_some() {
            command
                .arg("-c:a") // Changed from -acodec
                .arg("aac")
                .arg("-b:a")
                .arg("192k")
                .arg("-ac")
                .arg(format!("{}", audio_config.channels))
                .arg("-ar")
                .arg(format!("{}", audio_config.sample_rate));
        }

        // Set output path
        command.arg(output_path.clone());
        log::debug!("FFmpeg command: {:?}", command);

        match command.output() {
            Ok(output) => {
                // Log FFmpeg output for debugging
                log::debug!("FFmpeg stdout: {}", String::from_utf8_lossy(&output.stdout));
                log::debug!("FFmpeg stderr: {}", String::from_utf8_lossy(&output.stderr));

                if output.status.success() {
                    log::info!(
                        "Video generated successfully: {}",
                        output_path.to_string_lossy()
                    );
                } else {
                    log::error!("FFmpeg failed with status: {}", output.status);
                }
            }
            Err(e) => log::error!("FFmpeg execution failed: {}", e),
        }
    }

    pub fn is_finalizing(&self) -> bool {
        self.is_finalizing.load(Ordering::SeqCst)
    }

    pub fn process_audio(
        &mut self,
        audio_data: Vec<f32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if audio_data.is_empty() {
            return Ok(());
        }

        let (audio_config, temp_dir) = {
            let config = self.config.lock().unwrap();
            (config.audio.clone(), config.video.temp_dir.clone())
        };

        let audio_path = temp_dir.join("audio.wav");
        log::debug!("Audio path: {:?}", audio_path);

        let spec = WavSpec {
            channels: audio_config.channels,
            sample_rate: audio_config.sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let mut writer = WavWriter::create(&audio_path, spec)?;

        if let (Some(video_start), Some(audio_start)) =
            (self.recording_start_time, self.audio_start_time)
        {
            let offset_samples = (audio_start.duration_since(video_start).as_secs_f64()
                * audio_config.sample_rate as f64) as usize;

            // Add silence padding if audio started after video
            for _ in 0..offset_samples {
                writer.write_sample(0.0f32)?;
            }
        }

        for sample in audio_data {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;

        self.audio_file = Some(audio_path);
        log::info!("Audio saved succesfully");
        Ok(())
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }

    fn calculate_fps(
        frame_counter: u32,
        config: &VideoConfig,
        audio_file: &Option<PathBuf>,
        start_time: Option<Instant>,
    ) -> u32 {
        return config.fps;

        // Get actual recording duration
        let duration_secs = if let Some(start) = start_time {
            start.elapsed().as_secs_f64()
        } else {
            log::warn!("No start time available, using config fps");
            return config.fps;
        };

        // Calculate actual FPS based on frame count and duration
        let actual_fps = if duration_secs > 0.1 {
            let calculated_fps = (frame_counter as f64 / duration_secs).round() as u32;
            log::info!(
                "FPS calculation: {} frames / {:.2}s = {} fps (config: {} fps)",
                frame_counter,
                duration_secs,
                calculated_fps,
                config.fps
            );
            calculated_fps
        } else {
            log::warn!("Duration too short, using config fps");
            config.fps
        };

        // Validate against config
        if (actual_fps as i32 - config.fps as i32).abs() > 5 {
            log::warn!(
                "Large FPS deviation - Actual: {}, Config: {}",
                actual_fps,
                config.fps
            );
        }

        // Use actual FPS for consistent playback
        actual_fps
    }

    fn generate_unique_path(base_path: PathBuf) -> PathBuf {
        let stem = base_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");

        let ext = base_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("mkv");

        let parent = base_path.parent().unwrap_or(std::path::Path::new("."));

        let mut counter = 0;
        loop {
            let filename = if counter == 0 {
                format!("{}.{}", stem, ext)
            } else {
                format!("{}_{}.{}", stem, counter, ext)
            };

            let candidate = parent.join(filename);
            if !candidate.exists() {
                return candidate;
            }
            counter += 1;
        }
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
