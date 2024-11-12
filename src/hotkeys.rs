use std::sync::{Arc, Mutex};
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use inputbot::KeybdKey::*;
use std::thread;

#[derive(Default)]
pub struct HotkeyConfig {
    pub pause: VirtualKeyCode,
    pub blank: VirtualKeyCode,
    pub terminate: VirtualKeyCode,
    pub paused: Arc<Mutex<bool>>,
    pub terminate_flag: Arc<Mutex<bool>>,
}

pub fn initialize_hotkeys(config: Arc<HotkeyConfig>) {
    let pause_config = Arc::clone(&config);
    let blank_config = Arc::clone(&config);
    let terminate_config = Arc::clone(&config);

    map_hotkey(config.pause, move || {
        let mut paused = pause_config.paused.lock().unwrap();
        *paused = !*paused;
    });

    map_hotkey(config.blank, move || {
        // Implement blank screen functionality if needed
    });

    map_hotkey(config.terminate, move || {
        let mut terminate_flag = terminate_config.terminate_flag.lock().unwrap();
        *terminate_flag = true;
    });

    thread::spawn(|| {
        inputbot::handle_input_events();
    });
}

fn map_hotkey(key: egui::Key, action: impl Fn() + 'static + Send) {
    if let Some(ib_key) = egui_key_to_inputbot(key) {
        ib_key.bind(action);
    }
}

fn egui_key_to_inputbot(key: egui::Key) -> Option<inputbot::KeybdKey> {
    match key {
        egui::Key::A => Some(AKey),
        egui::Key::B => Some(BKey),
        egui::Key::C => Some(CKey),
        // Map other keys as needed
        egui::Key::Escape => Some(Escape),
        egui::Key::Space => Some(Space),
        egui::Key::Enter => Some(Enter),
        // ... map additional keys ...
        _ => None,
    }
}

pub fn is_paused(config: &Arc<HotkeyConfig>) -> bool {
    *config.paused.lock().unwrap()
}

pub fn should_terminate(config: &Arc<HotkeyConfig>) -> bool {
    *config.terminate_flag.lock().unwrap()
}

fn handle_keyboard_input(input: KeyboardInput, config: &Arc<HotkeyConfig>) {
    if let Some(keycode) = input.virtual_keycode {
        if input.state == ElementState::Pressed {
            if keycode == config.pause {
                let mut paused = config.paused.lock().unwrap();
                *paused = !*paused;
            } else if keycode == config.blank {
                // Implement blank screen functionality
            } else if keycode == config.terminate {
                let mut terminate_flag = config.terminate_flag.lock().unwrap();
                *terminate_flag = true;
            }
        }
    }
}

pub fn handle_event(event: &Event<()>, config: &Arc<HotkeyConfig>) {
    if let Event::WindowEvent {
        event: WindowEvent::KeyboardInput { event, .. },
        ..
    } = event
    {
        if let Some(virtual_keycode) = event.logical_key.decode() {
            let state = event.state;
            match (virtual_keycode, state) {
                (k, ElementState::Pressed) if k == config.pause => {
                    let mut paused = config.paused.lock().unwrap();
                    *paused = !*paused;
                }
                (k, ElementState::Pressed) if k == config.blank => {
                    // Implement blank screen functionality
                }
                (k, ElementState::Pressed) if k == config.terminate => {
                    let mut terminate = config.terminate_flag.lock().unwrap();
                    *terminate = true;
                }
                _ => {}
            }
        }
    }
}
