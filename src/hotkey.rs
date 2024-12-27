use egui::Key;
use std::collections::HashMap;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum HotkeyAction {
    ToggleStreaming,
    TogglePreview,
    StartRecording,
    Quit,
    ClosePopup,
}

impl HotkeyAction {
    pub fn is_visible(&self) -> bool {
        !matches!(self, HotkeyAction::ClosePopup)
    }
}

impl std::fmt::Display for HotkeyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = format!("{:?}", self);
        let mut result = String::new();
        let mut chars = s.chars().peekable();

        while let Some(current) = chars.next() {
            result.push(current);
            if let Some(next) = chars.peek() {
                if current.is_lowercase() && next.is_uppercase() {
                    result.push(' ');
                }
            }
        }

        write!(f, "{}", result)
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct KeyCombination {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: Key,
}

impl KeyCombination {
    pub const NULL: Self = Self {
        ctrl: false,
        shift: false,
        alt: false,
        // Any key, will be ignored
        key: Key::Enter,
    };
}

// Hotkey manager to handle all shortcuts
#[derive(Debug, Default)]
pub struct HotkeyManager {
    pub shortcuts: HashMap<KeyCombination, HotkeyAction>,
    pub default_shortcuts: HashMap<KeyCombination, HotkeyAction>,
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
                ctrl: false,
                shift: false,
                alt: false,
                key: Key::R,
            },
            HotkeyAction::StartRecording,
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
        self.default_shortcuts.insert(
            KeyCombination {
                ctrl: false,
                shift: false,
                alt: false,
                key: Key::Escape,
            },
            HotkeyAction::ClosePopup,
        );

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

            if self.current_combination.as_ref() != Some(&new_combination)
                && new_combination != KeyCombination::NULL
            {
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
        // Remove existing binding for this action
        self.shortcuts.retain(|_, a| a != &action);

        // Handle conflict: remove existing action at this combination
        if let Some(existing_action) = self.shortcuts.remove(&combination) {
            // Find default combination for displaced action
            if let Some((default_combo, _)) = self
                .default_shortcuts
                .iter()
                .find(|(_, a)| **a == existing_action)
            {
                // Restore displaced action to its default
                self.shortcuts
                    .insert(default_combo.clone(), existing_action);
            }
        }

        // Add new binding
        self.shortcuts.insert(combination, action);
    }

    pub fn get_unassigned_actions(&self) -> Vec<HotkeyAction> {
        self.default_shortcuts
            .values()
            .filter(|action| !self.shortcuts.values().any(|a| a == *action))
            .cloned()
            .collect()
    }

    pub fn reset_to_defaults(&mut self) {
        self.shortcuts = self.default_shortcuts.clone();
    }
}
