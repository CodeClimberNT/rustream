use std::sync::{Arc, Mutex};

use winit::keyboard::KeyCode;

pub struct HotkeyConfig {
    pub pause: KeyCode,
    pub blank: KeyCode,
    pub terminate: KeyCode,
    pub paused: Arc<Mutex<bool>>,
    pub terminate_flag: Arc<Mutex<bool>>,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        HotkeyConfig {
            pause: KeyCode::Escape,    // Hardcoded default value
            blank: KeyCode::Space,     // Hardcoded default value
            terminate: KeyCode::Enter, // Hardcoded default value
            paused: Arc::new(Mutex::new(false)),
            terminate_flag: Arc::new(Mutex::new(false)),
        }
    }
}

impl Clone for HotkeyConfig {
    fn clone(&self) -> Self {
        Self {
            pause: self.pause,
            blank: self.blank,
            terminate: self.terminate,
            paused: Arc::clone(&self.paused),
            terminate_flag: Arc::clone(&self.terminate_flag),
        }
    }
}

pub fn initialize_hotkeys(_config: Arc<HotkeyConfig>) {
    // Empty function since we're not using inputbot anymore
}

// Remove winit_key_to_inputbot function since we're not using inputbot anymore

pub fn is_paused(config: &Arc<HotkeyConfig>) -> bool {
    *config.paused.lock().unwrap()
}

pub fn should_terminate(config: &Arc<HotkeyConfig>) -> bool {
    *config.terminate_flag.lock().unwrap()
}

pub fn handle_key(key: KeyCode, config: &Arc<HotkeyConfig>) {
    if key == config.pause {
        let mut paused = config.paused.lock().unwrap();
        *paused = !*paused;
    } else if key == config.blank {
        // Implement blank screen functionality
    } else if key == config.terminate {
        let mut terminate = config.terminate_flag.lock().unwrap();
        *terminate = true;
    }
}
