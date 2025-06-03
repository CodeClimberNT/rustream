use crate::common::CaptureArea;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Config {
    pub video: VideoConfig,
    pub capture: CaptureConfig,
}

impl Config {
    pub fn update(&mut self, new_config: Config) {
        self.video = new_config.video;
        self.capture = new_config.capture;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoConfig {
    pub output_path: PathBuf,
    pub fps: u32,
    pub temp_dir: PathBuf,
}

impl Default for VideoConfig {
    fn default() -> Self {
        let base_path = if cfg!(debug_assertions) {
            // Development build - use project root
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        } else {
            // Release build - use user's video directory
            dirs::video_dir().unwrap_or_else(|| PathBuf::from("."))
        };

        let temp_path = if cfg!(debug_assertions) {
            base_path.join("temp")
        } else {
            std::env::temp_dir().join("rustream_temp")
        };

        Self {
            output_path: base_path.join("output.mkv"),
            // TODO: change to actual networking stream fps
            fps: 6,
            temp_dir: temp_path,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CaptureConfig {
    pub selected_monitor: usize,
    pub capture_area: Option<CaptureArea>,
}
