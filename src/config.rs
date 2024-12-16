use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
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
            output_path: base_path.join("output.mp4"),
            // TODO: Right now the recording is of the app that runs at 60fps, when
            // changing to
            fps: 60,
            temp_dir: temp_path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub selected_monitor: usize,
    pub options: scap::capturer::Options,
    // pub capture_area: Option<CaptureArea>,
    pub show_cursor: bool, // New field
}

impl Default for CaptureConfig {
    fn default() -> Self {
        let options = scap::capturer::Options {
            fps: 60,
            show_cursor: true,
            show_highlight: true,
            excluded_targets: None,
            output_type: scap::frame::FrameType::BGRAFrame,
            output_resolution: scap::capturer::Resolution::_720p,
            // crop_area: Some(Area {
            //     origin: Point { x: 0.0, y: 0.0 },
            //     size: Size {
            //         width: 500.0,
            //         height: 500.0,
            //     },
            // }),
            ..Default::default()
        };
        Self {
            selected_monitor: 0,
            options,
            show_cursor: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
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
