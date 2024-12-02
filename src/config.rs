use crate::common::CaptureArea;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Config {
    pub video: VideoConfig,
    pub capture: CaptureConfig,
    pub audio: AudioConfig,
}

impl Config {
    pub fn update(&mut self, new_config: Config) {
        self.video = new_config.video;
        self.capture = new_config.capture;
        self.audio = new_config.audio;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoConfig {
    pub output_path: PathBuf,
    pub fps: u32,
    pub temp_dir: PathBuf,
}

impl Default for VideoConfig {
    fn default() -> Self {
        let output_path = dirs::video_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("output.mp4");

        Self {
            output_path,
            fps: 30,
            temp_dir: std::env::temp_dir().join("temp"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CaptureConfig {
    pub selected_monitor: usize,
    pub capture_area: Option<CaptureArea>, // Changed from tuple to CaptureArea
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioConfig {
    pub enabled: bool,
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_size: usize,
    pub volume: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sample_rate: 48000,
            channels: 1,
            buffer_size: 1024,
            volume: 1.0,
        }
    }
}
