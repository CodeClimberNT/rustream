use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub video: VideoConfig,
    pub capture: CaptureConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CaptureConfig {
    pub selected_monitor: usize,
    pub capture_area: Option<(u32, u32, u32, u32)>, // (x, y, width, height)
}
