use crate::{annotations, capture, hotkeys, multimonitor, network, recording};
use eframe::egui;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

pub fn initialize_ui() {
    let options: eframe::NativeOptions = eframe::NativeOptions::default();
    eframe::run_native(
        "Rustream",
        options,
        Box::new(|_cc: &eframe::CreationContext<'_>| Box::new(RustreamApp::default())),
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
            runtime: Arc::new(Runtime::new().unwrap()),
            annotation_state: Arc::new(Mutex::new(annotations::AnnotationState::default())),
            hotkey_config: Arc::new(hotkeys::HotkeyConfig::default()),
            recording_state: None,
            configuring_hotkey: None,
            selecting_area: false,
            area_selection_start: None,
            area_selection_current: None,
        }
    }
}

impl eframe::App for RustreamApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle hotkey configuration
        if let Some(hotkey_name) = self.configuring_hotkey {
            ctx.input(|i| {
                for event in &i.events {
                    if let egui::Event::Key {
                        key,
                        physical_key: _,
                        repeat: _,
                        pressed: true,
                        modifiers: _,
                    } = event
                    {
                        match hotkey_name {
                            "pause" => self.hotkey_config.pause = *key,
                            "blank" => self.hotkey_config.blank = *key,
                            "terminate" => self.hotkey_config.terminate = *key,
                            _ => {}
                        }
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
    }

    fn initialize_receiver(&mut self) {
        // Initialize receiver-specific settings
    }

    fn caster_ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Caster Mode");
            // Monitor Selection
            ui.label("Select Monitor:");
            if ui.button("Choose Monitor").clicked() {
                self.selected_monitor = self.choose_monitor(ui);
            }
            // Custom Area Selection
            if ui.button("Select Screen Area").clicked() {
                self.custom_area = self.select_screen_area(ui);
            }
            // Start Streaming
            if ui.button("Start Streaming").clicked() {
                self.start_streaming();
            }
            // Annotation Tools
            if ui.button("Toggle Annotations").clicked() {
                let mut state = self.annotation_state.lock().unwrap();
                annotations::toggle_annotations(&mut state, !state.active);
            }
            // Hotkey Configuration
            if ui.button("Configure Hotkeys").clicked() {
                self.configure_hotkeys(ui);
            }
            // Draw Annotations
            annotations::draw_annotations(ui, &mut self.annotation_state.lock().unwrap());
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

    fn choose_monitor(&mut self, ui: &mut egui::Ui) -> Option<multimonitor::MonitorInfo> {
        // Provide UI for selecting a monitor from the list
        let monitors = multimonitor::list_monitors();
        egui::Window::new("Select Monitor").show(ui.ctx(), |ui| {
            for monitor in monitors {
                if ui.button(&monitor.name).clicked() {
                    self.selected_monitor = Some(monitor.clone());
                }
            }
        });
        self.selected_monitor.clone()
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
        runtime.spawn(async move {
            network::start_receiving(&address, enable_recording).await;
        });
    }

    fn start_recording(&mut self) {
        self.recording_state = Some(recording::Recorder::start_recording("output.mp4"));
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
}
