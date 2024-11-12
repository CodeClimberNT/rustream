use std::sync::{Arc, Mutex};
use std::thread;
use winit::event::Event;
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
            pause: KeyCode::Escape, // Provide a default value
            blank: KeyCode::Space,  // Provide a default value
            terminate: KeyCode::Enter, // Provide a default value
            paused: Arc::new(Mutex::new(false)),
            terminate_flag: Arc::new(Mutex::new(false)),
        }
    }
}
pub fn initialize_hotkeys(config: Arc<HotkeyConfig>) {
    let pause_config = Arc::clone(&config);
    // let blank_config = Arc::clone(&config);
    let terminate_config = Arc::clone(&config);

    // Map hotkeys using the updated mapping function
    if let Some(key) = winit_key_to_inputbot(config.pause) {
        key.bind(move || {
            let mut paused = pause_config.paused.lock().unwrap();
            *paused = !*paused;
        });
    }

    if let Some(key) = winit_key_to_inputbot(config.blank) {
        key.bind(move || {
            // Implement blank screen functionality if needed
        });
    }

    if let Some(key) = winit_key_to_inputbot(config.terminate) {
        key.bind(move || {
            let mut terminate_flag = terminate_config.terminate_flag.lock().unwrap();
            *terminate_flag = true;
        });
    }

    // Start inputbot in a separate thread
    thread::spawn(|| {
        inputbot::handle_input_events();
    });
}

fn winit_key_to_inputbot(key: KeyCode) -> Option<inputbot::KeybdKey> {
    match key {
        KeyCode::KeyA => Some(inputbot::KeybdKey::AKey),
        KeyCode::KeyB => Some(inputbot::KeybdKey::BKey),
        KeyCode::KeyC => Some(inputbot::KeybdKey::CKey),
        KeyCode::KeyD => Some(inputbot::KeybdKey::DKey),
        KeyCode::KeyE => Some(inputbot::KeybdKey::EKey),
        KeyCode::KeyF => Some(inputbot::KeybdKey::FKey),
        KeyCode::KeyG => Some(inputbot::KeybdKey::GKey),
        KeyCode::KeyH => Some(inputbot::KeybdKey::HKey),
        KeyCode::KeyI => Some(inputbot::KeybdKey::IKey),
        KeyCode::KeyJ => Some(inputbot::KeybdKey::JKey),
        KeyCode::KeyK => Some(inputbot::KeybdKey::KKey),
        KeyCode::KeyL => Some(inputbot::KeybdKey::LKey),
        KeyCode::KeyM => Some(inputbot::KeybdKey::MKey),
        KeyCode::KeyN => Some(inputbot::KeybdKey::NKey),
        KeyCode::KeyO => Some(inputbot::KeybdKey::OKey),
        KeyCode::KeyP => Some(inputbot::KeybdKey::PKey),
        KeyCode::KeyQ => Some(inputbot::KeybdKey::QKey),
        KeyCode::KeyR => Some(inputbot::KeybdKey::RKey),
        KeyCode::KeyS => Some(inputbot::KeybdKey::SKey),
        KeyCode::KeyT => Some(inputbot::KeybdKey::TKey),
        KeyCode::KeyU => Some(inputbot::KeybdKey::UKey),
        KeyCode::KeyV => Some(inputbot::KeybdKey::VKey),
        KeyCode::KeyW => Some(inputbot::KeybdKey::WKey),
        KeyCode::KeyX => Some(inputbot::KeybdKey::XKey),
        KeyCode::KeyY => Some(inputbot::KeybdKey::YKey),
        KeyCode::KeyZ => Some(inputbot::KeybdKey::ZKey),
        KeyCode::Escape => Some(inputbot::KeybdKey::EscapeKey),
        KeyCode::Space => Some(inputbot::KeybdKey::SpaceKey),
        KeyCode::Enter => Some(inputbot::KeybdKey::EnterKey),
        KeyCode::Tab => Some(inputbot::KeybdKey::TabKey),
        // Add mappings for additional keys as needed
        _ => None,
    }
}

pub fn is_paused(config: &Arc<HotkeyConfig>) -> bool {
    *config.paused.lock().unwrap()
}

pub fn should_terminate(config: &Arc<HotkeyConfig>) -> bool {
    *config.terminate_flag.lock().unwrap()
}

pub fn handle_event(event: &Event<()>, config: &Arc<HotkeyConfig>) {
    return;
    // if let Event::WindowEvent {
    //     event: WindowEvent::KeyboardInput { event, .. },
    //     ..
    // } = event
    // {
    //     if let Some(virtual_keycode) = event.logical_key.clone() {
    //         let state = event.state;
    //         match (virtual_keycode, state) {
    //             (Some(k), ElementState::Pressed) if k == config.pause => {
    //                 let mut paused = config.paused.lock().unwrap();
    //                 *paused = !*paused;
    //             }
    //             (Some(k), ElementState::Pressed) if k == config.blank => {
    //                 // Implement blank screen functionality
    //             }
    //             (Some(k), ElementState::Pressed) if k == config.terminate => {
    //                 let mut terminate = config.terminate_flag.lock().unwrap();
    //                 *terminate = true;
    //             }
    //             _ => {}
    //         }
    //     }
    // }
}
