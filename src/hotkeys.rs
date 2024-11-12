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

    // Map hotkeys using the updated inputbot API
    if let Some(key) = egui_key_to_inputbot(config.pause) {
        key.bind(move || {
            let mut paused = pause_config.paused.lock().unwrap();
            *paused = !*paused;
        });
    }

    if let Some(key) = egui_key_to_inputbot(config.blank) {
        key.bind(move || {
            // Implement blank screen functionality if needed
        });
    }

    if let Some(key) = egui_key_to_inputbot(config.terminate) {
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

fn egui_key_to_inputbot(key: egui::Key) -> Option<inputbot::KeybdKey> {
    // Map egui::Key to inputbot::KeybdKey as per inputbot 0.6.0 API
    match key {
        egui::Key::A => Some(AKey),
        egui::Key::B => Some(BKey),
        egui::Key::C => Some(CKey),
        egui::Key::D => Some(DKey),
        egui::Key::E => Some(EKey),
        egui::Key::F => Some(FKey),
        egui::Key::G => Some(GKey),
        egui::Key::H => Some(HKey),
        egui::Key::I => Some(IKey),
        egui::Key::J => Some(JKey),
        egui::Key::K => Some(KKey),
        egui::Key::L => Some(LKey),
        egui::Key::M => Some(MKey),
        egui::Key::N => Some(NKey),
        egui::Key::O => Some(OKey),
        egui::Key::P => Some(PKey),
        egui::Key::Q => Some(QKey),
        egui::Key::R => Some(RKey),
        egui::Key::S => Some(SKey),
        egui::Key::T => Some(TKey),
        egui::Key::U => Some(UKey),
        egui::Key::V => Some(VKey),
        egui::Key::W => Some(WKey),
        egui::Key::X => Some(XKey),
        egui::Key::Y => Some(YKey),
        egui::Key::Z => Some(ZKey),
        egui::Key::Escape => Some(EscapeKey),
        egui::Key::Space => Some(SpaceKey),
        egui::Key::Enter => Some(ReturnKey),
        egui::Key::Tab => Some(TabKey),
        // Map additional keys as needed
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
