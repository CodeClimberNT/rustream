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
    current_combination: Option<KeyCombination>,
}

impl HotkeyManager {
    pub fn new() -> Self {
        let mut manager = Self {
            shortcuts: HashMap::new(),
            default_shortcuts: HashMap::new(),
            current_combination: None,
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

    pub fn handle_input(&mut self, ui: &egui::Context) -> Option<HotkeyAction> {
        let input = ui.input(|i| {
            (
                i.modifiers.ctrl,
                i.modifiers.shift,
                i.modifiers.alt,
                i.keys_down.iter().next().copied(),
            )
        });

        if let (ctrl, shift, alt, Some(key)) = input {
            let new_combination = KeyCombination {
                ctrl,
                shift,
                alt,
                key,
            };

            if self.current_combination.as_ref() != Some(&new_combination) {
                self.current_combination = Some(new_combination.clone());
                return self.shortcuts.get(&new_combination).cloned();
            }
        } else {
            // Reset when no keys are pressed
            self.current_combination = None;
        }
        None
    }

    pub fn register_shortcut(&mut self, combination: KeyCombination, action: HotkeyAction) {
        self.shortcuts.insert(combination, action);
    }

    pub fn reset_to_defaults(&mut self) {
        self.shortcuts = self.default_shortcuts.clone();
    }
}
