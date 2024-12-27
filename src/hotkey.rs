use egui::Key;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum HotkeyAction {
    ToggleStreaming,
    TogglePreview,
    Quit,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct KeyCombination {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: Key,
}

// Hotkey manager to handle all shortcuts
#[derive(Debug, Default)]
pub struct HotkeyManager {
    shortcuts: HashMap<KeyCombination, HotkeyAction>,
    default_shortcuts: HashMap<KeyCombination, HotkeyAction>,
    last_trigger: Option<Instant>,
    cooldown: Duration,
}

impl HotkeyManager {
    pub fn new() -> Self {
        let mut manager = Self {
            shortcuts: HashMap::new(),
            default_shortcuts: HashMap::new(),
            last_trigger: None,
            cooldown: Duration::from_millis(200),
        };
        manager.setup_default_shortcuts();
        manager
    }

    fn setup_default_shortcuts(&mut self) {
        // Define default shortcuts
        self.default_shortcuts.insert(
            KeyCombination {
                ctrl: true,
                shift: false,
                alt: false,
                key: Key::S,
            },
            HotkeyAction::ToggleStreaming,
        );
        self.default_shortcuts.insert(
            KeyCombination {
                ctrl: false,
                shift: false,
                alt: false,
                key: Key::P,
            },
            HotkeyAction::TogglePreview,
        );
        self.default_shortcuts.insert(
            KeyCombination {
                ctrl: true,
                shift: false,
                alt: false,
                key: Key::Q,
            },
            HotkeyAction::Quit,
        );
        // Add more defaults...

        // Copy defaults to active shortcuts
        self.shortcuts = self.default_shortcuts.clone();
    }

    pub fn handle_input(&self, ui: &egui::Context) -> Option<HotkeyAction> {
        let now = Instant::now();
        if let Some(last) = self.last_trigger {
            if now - last < self.cooldown {
                return None;
            }
        }

        let input = ui.input(|i| {
            (
                i.modifiers.ctrl,
                i.modifiers.shift,
                i.modifiers.alt,
                i.keys_down.iter().next().copied(),
            )
        });

        if let (ctrl, shift, alt, Some(key)) = input {
            let combination = KeyCombination {
                ctrl,
                shift,
                alt,
                key,
            };
            self.shortcuts.get(&combination).cloned()
        } else {
            None
        }
    }

    pub fn register_shortcut(&mut self, combination: KeyCombination, action: HotkeyAction) {
        self.shortcuts.insert(combination, action);
    }

    pub fn reset_to_defaults(&mut self) {
        self.shortcuts = self.default_shortcuts.clone();
    }
}
