use crate::{annotations, capture, hotkeys, multimonitor, network, recording};
use eframe::egui;
use std::sync::Arc;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::Mutex;
use winit::keyboard::KeyCode;
use winit::platform::windows::EventLoopBuilderExtWindows as _;

fn key_to_keycode(key: egui::Key) -> KeyCode {
    match key {
        egui::Key::Space => KeyCode::Space,
        egui::Key::Enter => KeyCode::Enter,
        egui::Key::Escape => KeyCode::Escape,
        _ => KeyCode::Space, // Default mapping, expand as needed
    }
}

pub fn initialize_ui() {
    let options = eframe::NativeOptions {
        event_loop_builder: Some(Box::new(|builder| {
            builder.with_any_thread(true);
        })),
        ..Default::default()
    };

    _ = eframe::run_native(
        "Rustream",
        options,
        Box::new(|_cc: &eframe::CreationContext<'_>| Ok(Box::new(RustreamApp::default()))),
    );
}

struct RustreamApp {
    mode: Option<AppMode>,
    selected_monitor: Option<multimonitor::MonitorInfo>,
    custom_area: Option<capture::ScreenArea>,
    caster_address: String,
    runtime: Arc<Runtime>,
    annotation_state: Arc<Mutex<annotations::AnnotationState>>,
    hotkey_config: Arc<hotkeys::HotkeyConfig>,
    recording_state: Option<recording::Recorder>,
    configuring_hotkey: Option<&'static str>,
    selecting_area: bool,
    area_selection_start: Option<egui::Pos2>,
    area_selection_current: Option<egui::Pos2>,
    selecting_screen_area_active: bool,
    monitor_selection_window_open: bool,
    hotkey_config_window_open: bool,
    annotation_window_open: bool,
}

enum AppMode {
    Caster,
    Receiver,
}

impl Default for RustreamApp {
    fn default() -> Self {
        Self {
            mode: None,
            selected_monitor: None,
            custom_area: None,
            caster_address: String::new(),
            runtime: Arc::new(Builder::new_multi_thread().enable_all().build().unwrap()),
            annotation_state: Arc::new(Mutex::new(annotations::AnnotationState::default())),
            hotkey_config: Arc::new(hotkeys::HotkeyConfig::default()),
            recording_state: None,
            configuring_hotkey: None,
            selecting_area: false,
            area_selection_start: None,
            area_selection_current: None,
            selecting_screen_area_active: false,
            monitor_selection_window_open: false,
            hotkey_config_window_open: false,
            annotation_window_open: false,
        }
    }
}

impl eframe::App for RustreamApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle global hotkeys through egui events
        ctx.input(|i| {
            for event in &i.events {
                if let egui::Event::Key {
                    key, pressed: true, ..
                } = event
                {
                    let keycode = key_to_keycode(*key);
                    hotkeys::handle_key(keycode, &self.hotkey_config);
                }
            }
        });

        // Handle hotkey configuration
        if let Some(hotkey_name) = self.configuring_hotkey {
            ctx.input(|i| {
                for event in &i.events {
                    if let egui::Event::Key {
                        key, pressed: true, ..
                    } = event
                    {
                        let mut new_config = (*self.hotkey_config).clone();

                        match hotkey_name {
                            "pause" => new_config.pause = key_to_keycode(*key),
                            "blank" => new_config.blank = key_to_keycode(*key),
                            "terminate" => new_config.terminate = key_to_keycode(*key),
                            _ => {}
                        }
                        self.hotkey_config = Arc::new(new_config);
                        self.configuring_hotkey = None;
                        break;
                    }
                }
            });
        }

        // Handle area selection
        if self.selecting_area {
            if self.area_selection_start.is_none() {
                if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
                    if ctx.input(|i| i.pointer.primary_clicked()) {
                        self.area_selection_start = Some(pos);
                    }
                }
            } else {
                self.area_selection_current = ctx.input(|i| i.pointer.hover_pos());
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Foreground,
                    egui::Id::new("area_selection"),
                ));
                if let (Some(start), Some(current)) =
                    (self.area_selection_start, self.area_selection_current)
                {
                    let rect = egui::Rect::from_two_pos(start, current);
                    painter.rect_stroke(rect, 0.0, (2.0, egui::Color32::GREEN));
                    if ctx.input(|i| i.pointer.primary_released()) {
                        self.selecting_area = false;
                        self.custom_area = Some(capture::ScreenArea {
                            x: rect.min.x.min(rect.max.x) as u32,
                            y: rect.min.y.min(rect.max.y) as u32,
                            width: rect.width() as u32,
                            height: rect.height() as u32,
                        });
                    }
                }
            }
        }

        match self.mode {
            None => self.show_mode_selection(ctx),
            Some(AppMode::Caster) => self.caster_ui(ctx),
            Some(AppMode::Receiver) => self.receiver_ui(ctx),
        }
    }
}

impl RustreamApp {
    fn show_mode_selection(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Select Mode");
            ui.horizontal(|ui| {
                if ui.button("Caster").clicked() {
                    self.mode = Some(AppMode::Caster);
                    self.initialize_caster();
                }
                if ui.button("Receiver").clicked() {
                    self.mode = Some(AppMode::Receiver);
                    self.initialize_receiver();
                }
            });
        });
    }

    fn initialize_caster(&mut self) {
        // Initialize caster-specific settings
        self.selected_monitor = Some(multimonitor::select_monitor(0));
        self.hotkey_config = Arc::new(hotkeys::HotkeyConfig::default());
        hotkeys::initialize_hotkeys(Arc::clone(&self.hotkey_config));
        // Added initialization for recording_state
        self.recording_state = None;
    }

    fn initialize_receiver(&mut self) {
        // Initialize receiver-specific settings
        self.caster_address = String::new();
    }

    fn caster_ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Caster Mode");

            // Monitor Selection
            ui.label("Select Monitor:");
            if ui.button("Choose Monitor").clicked() {
                self.monitor_selection_window_open = true;
            }
            if self.monitor_selection_window_open {
                self.show_monitor_selection_window(ctx);
            }

            // Custom Area Selection
            if ui.button("Select Screen Area").clicked() {
                self.selecting_screen_area_active = true;
            }
            if self.selecting_screen_area_active {
                self.handle_screen_area_selection(ctx);
            }

            // Start Streaming
            if ui.button("Start Streaming").clicked() {
                self.start_streaming();
            }

            // Hotkey Configuration
            if ui.button("Configure Hotkeys").clicked() {
                self.hotkey_config_window_open = true;
            }
            if self.hotkey_config_window_open {
                self.show_hotkey_config_window(ctx);
            }

            // Annotation Tools
            if ui.button("Toggle Annotations").clicked() {
                self.annotation_window_open = !self.annotation_window_open;
                let mut state = futures::executor::block_on(self.annotation_state.lock());
                annotations::toggle_annotations(&mut state, self.annotation_window_open);
            }
            if self.annotation_window_open {
                annotations::draw_annotations(
                    ui,
                    &mut futures::executor::block_on(self.annotation_state.lock()),
                );
            }
        });
    }

    fn receiver_ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Receiver Mode");
            // Input Caster Address
            ui.horizontal(|ui| {
                ui.label("Caster Address:");
                ui.text_edit_singleline(&mut self.caster_address);
            });
            // Start Receiving
            if ui.button("Connect").clicked() && !self.caster_address.is_empty() {
                self.start_receiving();
            }
            // Recording Options
            if ui.button("Start Recording").clicked() {
                self.start_recording();
            }
            if ui.button("Stop Recording").clicked() {
                self.stop_recording();
            }
        });
    }

    fn choose_monitor(ui: &mut egui::Ui) -> Option<multimonitor::MonitorInfo> {
        // Provide UI for selecting a monitor from the list
        let monitors = multimonitor::list_monitors();
        let mut selected = None;
        egui::Window::new("Select Monitor").show(ui.ctx(), |ui| {
            for monitor in monitors {
                if ui.button(&monitor.name).clicked() {
                    selected = Some(monitor.clone());
                }
            }
        });
        selected
    }

    fn select_screen_area(&mut self, ui: &mut egui::Ui) -> Option<capture::ScreenArea> {
        ui.label("Click 'Start Selection' and drag to select the screen area.");
        if ui.button("Start Selection").clicked() {
            self.selecting_area = true;
            self.area_selection_start = None;
            self.area_selection_current = None;
        }
        if let Some(area) = &self.custom_area {
            ui.label(format!(
                "Selected Area: x={}, y={}, width={}, height={}",
                area.x, area.y, area.width, area.height
            ));
        }
        self.custom_area.clone()
    }

    fn start_streaming(&self) {
        let runtime = Arc::clone(&self.runtime);
        let area = self.custom_area.clone();
        let hotkey_config = Arc::clone(&self.hotkey_config);
        let annotation_state = Arc::clone(&self.annotation_state);
        runtime.spawn(async move {
            network::start_streaming(area, hotkey_config, annotation_state).await;
        });
    }

    fn start_receiving(&self) {
        let runtime = Arc::clone(&self.runtime);
        let address = self.caster_address.clone();
        let enable_recording = self.recording_state.is_some();
        tokio::task::spawn_blocking(move || {
            let runtime = Arc::clone(&runtime);
            runtime.block_on(async {
                network::start_receiving(&address, enable_recording).await;
            });
        });
    }

    fn start_recording(&mut self) {
        let width;
        let height;
        if let Some(ref area) = self.custom_area {
            width = area.width;
            height = area.height;
        } else if let Some(ref monitor) = self.selected_monitor {
            width = monitor.width;
            height = monitor.height;
        } else {
            width = 1280; // fallback
            height = 720;
        }
        self.recording_state = Some(recording::Recorder::start_recording(
            "output.mp4",
            width,
            height,
        ));
    }

    fn stop_recording(&mut self) {
        if let Some(ref mut recorder) = self.recording_state {
            recorder.stop_recording();
            self.recording_state = None;
        }
    }

    fn configure_hotkeys(&mut self, ui: &mut egui::Ui) {
        egui::Window::new("Hotkey Configuration").show(ui.ctx(), |ui| {
            ui.label("Press a key when 'Change' is clicked.");

            ui.horizontal(|ui| {
                ui.label("Pause/Resume Transmission:");
                ui.label(format!("{:?}", self.hotkey_config.pause));
                if ui.button("Change").clicked() {
                    self.configuring_hotkey = Some("pause");
                }
            });

            ui.horizontal(|ui| {
                ui.label("Blank Screen:");
                ui.label(format!("{:?}", self.hotkey_config.blank));
                if ui.button("Change").clicked() {
                    self.configuring_hotkey = Some("blank");
                }
            });

            ui.horizontal(|ui| {
                ui.label("Terminate Session:");
                ui.label(format!("{:?}", self.hotkey_config.terminate));
                if ui.button("Change").clicked() {
                    self.configuring_hotkey = Some("terminate");
                }
            });
        });
    }
    fn show_monitor_selection_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Select Monitor")
            .open(&mut self.monitor_selection_window_open)
            .show(ctx, |ui| {
                if let Some(monitor) = Self::choose_monitor(ui) {
                    self.selected_monitor = Some(monitor);
                }
            });
    }
    fn handle_screen_area_selection(&mut self, ctx: &egui::Context) {
        let mut is_open = self.selecting_screen_area_active;
        egui::Window::new("Screen Area Selection")
            .open(&mut is_open)
            .show(ctx, |ui| {
                self.custom_area = self.select_screen_area(ui);
                if ui.button("Done").clicked() {
                    self.selecting_screen_area_active = false;
                }
                if ui.button("Cancel").clicked() {
                    self.selecting_screen_area_active = false;
                    self.custom_area = None;
                }
            });
        self.selecting_screen_area_active = is_open;
    }

    fn show_hotkey_config_window(&mut self, ctx: &egui::Context) {
        let mut is_open = self.hotkey_config_window_open;
        egui::Window::new("Hotkey Configuration")
            .open(&mut is_open)
            .show(ctx, |ui| {
                self.configure_hotkeys(ui);
            });
        self.hotkey_config_window_open = is_open;
    }
}
