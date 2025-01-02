use crate::audio_capture::AudioCapturer;
use crate::common::CaptureArea;
use crate::config::Config;
use crate::hotkey::{HotkeyAction, HotkeyManager, KeyCombination};
use crate::screen_capture::{CapturedFrame, ScreenCapture};
use crate::video_recorder::VideoRecorder;
use crate::secondaryapp::SecondaryApp;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;
use serde_json::json;

use eframe::egui;
use egui::{
    CentralPanel, Color32, ColorImage, ComboBox, Context, Pos2, Rect, RichText, TextureHandle, TopBottomPanel, Ui, Window
};

use std::env;
use std::io::Write;
use std::process::Command;

use display_info::DisplayInfo;
use winit::event_loop::EventLoop;

lazy_static! {
    pub static ref GLOBAL_CAPTURE_AREA: Arc<Mutex<CaptureArea>> =
        Arc::new(Mutex::new(CaptureArea::default()));
}


pub struct RustreamApp {
    pub config: Arc<Mutex<Config>>,
    frame_grabber: ScreenCapture,
    audio_capturer: AudioCapturer,
    video_recorder: VideoRecorder,
    page: PageView,                              // Enum to track modes
    display_texture: Option<TextureHandle>,      // Texture for the screen capture
    textures: HashMap<TextureId, TextureHandle>, // List of textures
    cropped_frame: Option<CapturedFrame>,        // Cropped image to send
    address_text: String,                        // Text input for the receiver mode
    preview_active: bool,
    is_selecting: bool,
    capture_area: Option<CaptureArea>,
    show_config: bool,
    hotkey_manager: HotkeyManager,
    editing_hotkey: Option<HotkeyAction>,
    triggered_actions: Vec<HotkeyAction>,
    previous_monitor: usize,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextureId {
    #[default]
    Error,
    HomeIcon,
    QuitIcon,
}

impl std::fmt::Display for TextureId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Use enum name as string for egui texture identification
        write!(f, "{:?}", self)
    }
}

struct TextureResource {
    id: TextureId,
    path: &'static [u8],
}

const TEXTURE_LIST: &[TextureResource] = &[
    TextureResource {
        id: TextureId::Error,
        path: include_bytes!("../assets/icons/error.svg"),
    },
    TextureResource {
        id: TextureId::HomeIcon,
        path: include_bytes!("../assets/icons/home_icon.svg"),
    },
    TextureResource {
        id: TextureId::QuitIcon,
        path: include_bytes!("../assets/icons/quit_icon.svg"),
    },
];

const NUM_TEXTURES: usize = TEXTURE_LIST.len();

#[derive(Default, Debug, Copy, Clone)]
pub enum PageView {
    #[default]
    HomePage,
    Caster,
    Receiver,
}

#[derive(Debug, Clone)]
struct MonitorInfo {
    id: usize,
    name: String,
    position: (i32, i32),
    
}

impl RustreamApp {

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx: &Context = &cc.egui_ctx;
        egui_extras::install_image_loaders(ctx);

        let mut textures: HashMap<TextureId, TextureHandle> =
            HashMap::<TextureId, TextureHandle>::with_capacity(NUM_TEXTURES);

        TEXTURE_LIST.iter().for_each(|texture| {
            RustreamApp::add_texture_to_map(&mut textures, ctx, texture, None);
        });

        assert_eq!(
            textures.len(),
            NUM_TEXTURES,
            r"Numbers of Textures Declared: {} | Actual number of textures: {},
            Check:
                1. If the texture is loaded correctly
                2. If the texture name is unique
                3. Try again and pray to the Rust gods",
            NUM_TEXTURES,
            textures.len()
        );

        let config: Arc<Mutex<Config>> = Arc::new(Mutex::new(Config::default()));
        let frame_grabber: ScreenCapture = ScreenCapture::new(config.clone());
        let video_recorder = VideoRecorder::new(config.clone());

        let audio_capturer = AudioCapturer::new(
            config.clone()
        );

        RustreamApp {
            config,
            frame_grabber,
            video_recorder,
            audio_capturer,
            textures,
            hotkey_manager: HotkeyManager::new(),
            page: PageView::HomePage,
            display_texture: None,
            cropped_frame: None,
            address_text: String::new(),
            preview_active: false,
            is_selecting: false,
            capture_area: None,
            show_config: false,
            editing_hotkey: None,
            triggered_actions: Vec::new(),
            previous_monitor: 0,
        }
    }

    fn get_preview_screen_rect(&self, ui: &egui::Ui) -> Rect {
        // Adjust this based on how your preview is laid out
        // For example, occupy the full available space
        ui.available_rect_before_wrap()
    }

    fn reset_ui(&mut self) {
        // Reset the application
        self.frame_grabber.reset_capture();
        self.page = PageView::default();
        self.address_text.clear();
    }

    fn set_page(&mut self, page: PageView) {
        self.page = page;
    }

    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Home button on the left
            let home_text = RichText::new("🏠").size(24.0);
            if self.clickable_element(ui, home_text, HotkeyAction::Home, false) {
                self.reset_ui();
            }

            // ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Add any other Left to right elements here
            // });
        });
    }

    fn home_page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);

                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("CAST NEW STREAMING")
                                .size(32.0)
                                .strong(),
                        )
                        .min_size(egui::vec2(300.0, 60.0)),
                    )
                    .clicked()
                {
                    self.set_page(PageView::Caster);
                }

                ui.add_space(30.0);
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("VIEW STREAMING").size(32.0).strong(),
                        )
                        .min_size(egui::vec2(300.0, 60.0)),
                    )
                    .clicked()
                {
                    self.set_page(PageView::Receiver);
                }
            });
        });
    }

    fn render_config_window(&mut self, ctx: &Context) {
        let mut show_config = self.show_config;

        Window::new("Configuration")
            .open(&mut show_config)
            .resizable(true)
            .movable(true)
            .frame(
                egui::Frame::window(&ctx.style())
                    .outer_margin(0.0)
                    .inner_margin(10.0),
            )
            .show(ctx, |ui| {
                let mut config = self.config.lock().unwrap().clone();

                // Manual capture area input
                ui.heading("Capture Area");
                ui.horizontal(|ui| {
                    let mut area = GLOBAL_CAPTURE_AREA.lock().unwrap();
                    //println!("this is the global variable: {:?}", *area);

                    ui.vertical(|ui| {
                        // FIXME: X=0 value error
                        ui.label("X:");
                        let mut x_str = area.x.to_string();
                        if ui.text_edit_singleline(&mut x_str).changed() {
                            if let Ok(x) = x_str.parse() {
                                area.x = x;
                                self.capture_area = Some(*area);
                            }
                        }

                        ui.label("Y:");
                        let mut y_str = area.y.to_string();
                        if ui.text_edit_singleline(&mut y_str).changed() {
                            if let Ok(y) = y_str.parse() {
                                area.y = y;
                                self.capture_area = Some(*area);
                            }
                        }
                    });

                    ui.vertical(|ui| {
                        ui.label("Width:");
                        let mut width_str = area.width.to_string();
                        if ui.text_edit_singleline(&mut width_str).changed() {
                            if let Ok(width) = width_str.parse() {
                                area.width = width;
                                self.capture_area = Some(*area);
                            }
                        }

                        ui.label("Height:");
                        let mut height_str = area.height.to_string();
                        if ui.text_edit_singleline(&mut height_str).changed() {
                            if let Ok(height) = height_str.parse() {
                                area.height = height;
                                self.capture_area = Some(*area);
                            }
                        }
                    });
                });

                // Update config when capture area changes
                if let Some(area) = self.capture_area {
                    config.capture.capture_area = Some(area);
                }

                ui.heading("Streaming Settings");
                ui.separator();
                
                // Monitor selection
                let selected_monitor = &mut config.capture.selected_monitor;
                let current_monitor= *selected_monitor;

                ComboBox::from_label("Monitor")
                    .selected_text(format!("Monitor {}", selected_monitor))
                    .show_ui(ui, |ui| {
                        self.frame_grabber
                            .get_monitors()
                            .iter()
                            .enumerate()
                            .for_each(|(i, m)| {
                            if ui.selectable_value(selected_monitor, i, m).clicked() {
                            if current_monitor != self.previous_monitor {
                            self.capture_area = None;
                            self.previous_monitor = current_monitor;
                    }
                }
            });
                    });

                ui.horizontal(|ui| {
                    self.is_selecting ^= ui.button("Select Capture Area").clicked();

                    if self.is_selecting {
                        //println!("{:?}", self.capture_area);
                        //creating a transparent window with egui-overlay

                        // Open a new full-size window for selecting capture area
                        //check if the process with arg --secondary is opened yet
                        //shows in console value of selected_monitor
                        //log::info!("Selected Monitor: {}", selected_monitor);
                        let displays = DisplayInfo::all().unwrap_or_default();
                        let display = displays.get(*selected_monitor).unwrap_or_else(|| {
                            log::error!("Monitor not found: {}", selected_monitor);
                            std::process::exit(1);
                        });
                        //display name + x and y
                        log::info!("Display: {} ({}x{}) ({}x{}) {}", display.name, display.x, display.y,display.width,display.height, display.scale_factor);
                        let output = Command::new(env::current_exe().unwrap())
                        .arg("--secondary")
                        .arg(display.x.to_string())
                        .arg(display.y.to_string())
                        .arg(display.width.to_string())
                        .arg(display.height.to_string())
                        .arg(display.scale_factor.to_string())
                        .output()
                        .expect("failed to execute process");

                            if output.status.success() {
                                // Parse stdout with error handling
                                let stdout = std::str::from_utf8(&output.stdout).unwrap_or_else(|e| {
                                    log::error!("Failed to read stdout: {}", e);
                                    ""
                                });
                                log::debug!("Main process received: {}", stdout);
                            
                                // Parse JSON with detailed error handling
                                let json_response: serde_json::Value = serde_json::from_str(stdout).unwrap_or_else(|e| {
                                    log::error!("Failed to parse JSON response: {}", e);
                                    serde_json::json!({ "status": "error" })
                                });
                                
                                match json_response["status"].as_str() {
                                    Some("success") => {
                                        if let Some(data) = json_response.get("data") {
                                            // Detailed error handling for struct mismatch
                                            let capture_area = serde_json::from_value(data.clone()).unwrap_or_else(|e| {
                                                log::error!("Failed to parse capture area data: {}", e);
                                                log::error!("Possible struct mismatch between SecondaryApp and main process");
                                                log::error!("Expected format: {{x: usize, y: usize, width: usize, height: usize}}");
                                                None
                                            });
                                            self.capture_area = capture_area;
                                        }
                                    }
                                    Some("cancelled") => {
                                        println!("User cancelled the capture operation");
                                    }
                                    _ => {
                                        log::error!("Unknown status in response");
                                    }
                                }
                            } else {
                                // Handle process errors
                                match std::str::from_utf8(&output.stderr) {
                                    Ok(stderr) if !stderr.is_empty() => {
                                        log::error!("Secondary process error: {}", stderr);
                                    }
                                    Err(e) => {
                                        log::error!("Failed to read stderr: {}", e);
                                    }
                                    _ => {
                                        log::error!("Secondary process failed with no error output");
                                    }
                                }
                            }

                        self.is_selecting = false;
                    }

                    if self.capture_area.is_some() && ui.button("Reset Capture Area").clicked() {
                        self.capture_area = None;
                    }
                });

                // Update capture area in config when it changes
                if let Some(area) = self.capture_area {
                    let mut config = self.config.lock().unwrap();
                    config.capture.capture_area =
                        Some(CaptureArea::new(area.x, area.y, area.width, area.height));
                }

                // In render_config_window after the "Streaming Settings" section:
                ui.heading("Hotkey Settings");
                ui.separator();

                // Show current hotkeys in a table
                ui.label("Current Hotkeys:");
                egui::Grid::new("hotkeys_grid")
                    .num_columns(3)
                    .spacing([40.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        // Header
                        ui.label("Action");
                        ui.label("Shortcut");
                        ui.label("Controls");
                        ui.end_row();

                        // Display each hotkey
                        let actions: Vec<_> = self
                            .hotkey_manager
                            .default_shortcuts
                            .values()
                            .filter(|action| action.is_visible())
                            .cloned()
                            .collect();

                        actions.iter().for_each(|action| {
                            ui.label(action.to_string());

                            // Find current combination and check if default
                            let (combo_text, is_default) = self
                                .hotkey_manager
                                .shortcuts
                                .iter()
                                .find(|(_, a)| *a == action)
                                .map(|(k, _)| {
                                    let text = k.to_string();
                                    let is_default = self.hotkey_manager.is_default(k, action);
                                    (
                                        if is_default {
                                            RichText::new(text)
                                        } else {
                                            RichText::new(text).strong()
                                        },
                                        is_default,
                                    )
                                })
                                .unwrap_or_else(|| (RichText::new("Unassigned"), true));

                            ui.label(combo_text);

                            ui.horizontal(|ui| {
                                if ui.button("🖊").clicked() {
                                    self.editing_hotkey = Some(action.clone());
                                }
                                ui.add_enabled(!is_default, egui::Button::new("↺"))
                                    .clicked()
                                    .then(|| self.hotkey_manager.reset_action(action));
                            });
                            ui.end_row();
                        });
                    });

                if let Some(editing_action) = self.editing_hotkey.clone() {
                    Window::new("Configure Hotkey")
                        .collapsible(false)
                        .resizable(false)
                        .show(ctx, |ui| {
                            ui.label(format!(
                                "Press new key combination for {:?}",
                                editing_action
                            ));
                            ui.label("Press Esc to cancel");

                            // Capture key input
                            let input = ui.input(|i| {
                                (
                                    i.modifiers.ctrl,
                                    i.modifiers.shift,
                                    i.modifiers.alt,
                                    i.keys_down.iter().next().copied(),
                                )
                            });

                            if let (ctrl, shift, alt, Some(key)) = input {
                                if key == egui::Key::Escape {
                                    self.editing_hotkey = None;
                                } else {
                                    let new_combination = KeyCombination {
                                        ctrl,
                                        shift,
                                        alt,
                                        key,
                                    };
                                    self.hotkey_manager
                                        .register_shortcut(new_combination, editing_action);
                                    self.editing_hotkey = None;
                                }
                            }
                        });
                }

                // Reset button
                if ui.button("Reset to Default Hotkeys").clicked() {
                    self.hotkey_manager.reset_to_defaults();
                }

                ui.heading("Recording Settings");
                ui.separator();

                // Output path configuration
                ui.horizontal(|ui| {
                    ui.label("Output path:");
                    let mut recording_path =
                        config.video.output_path.to_string_lossy().into_owned();
                    ui.text_edit_singleline(&mut recording_path)
                        .on_hover_text(recording_path.clone());
                    if ui.button("📂").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title("Save recording as...")
                            .set_file_name("output.mkv")
                            .add_filter("Matroska Video", &["mkv"])
                            .save_file()
                        {
                            config.video.output_path = path;
                        }
                    }
                });

                // FPS settings
                ui.horizontal(|ui| {
                    ui.label("Target FPS:");
                    // let mut config = self.config.lock().unwrap();
                    ComboBox::from_label("")
                        .selected_text(format!("{} FPS", config.video.fps))
                        .show_ui(ui, |ui| {
                            for &fps in &[24, 25, 30, 50, 60] {
                                ui.selectable_value(
                                    &mut config.video.fps,
                                    fps,
                                    format!("{} FPS", fps),
                                );
                            }
                        });
                });

                // Audio settings
                ui.heading("Audio Settings");
                ui.separator();

                // let mut config = self.config.lock().unwrap();
                ui.checkbox(&mut config.audio.enabled, "Enable Audio");
                if config.audio.enabled {
                    ui.add(
                        egui::Slider::new(&mut config.audio.volume, 0.0..=1.0)
                            .text("Volume")
                            .step_by(0.1),
                    );
                }

                // Apply changes if the config has changed
                let has_config_changed: bool = self.config.lock().unwrap().clone() != config;
                if has_config_changed {
                    log::debug!("Config changed: {:?}", config);
                    self.config.lock().unwrap().update(config);
                    self.frame_grabber.reset_capture();
                }
            });
        self.show_config = show_config;
    }

    fn render_recording_controls(&mut self, ui: &mut Ui) {
        let recording = self.video_recorder.is_recording();
        let finalizing = self.video_recorder.is_finalizing();

        if finalizing {
            ui.spinner();
            ui.label("Finalizing video...");
            return;
        }

        if self.action_button(
            ui,
            if recording {
                "⏹ Stop Recording"
            } else {
                "⏺ Start Recording"
            },
            HotkeyAction::StartRecording,
        ) {
            if recording {
                self.stop_recording();
            } else {
                self.start_recording();
            }
        }
        
    }
    fn caster_page(&mut self, ui: &mut egui::Ui, ctx: &Context, _frame: &mut eframe::Frame) {
        ui.heading("Monitor Feedback");
        ui.separator();
        ui.vertical_centered(|ui| {
            ui.horizontal(|ui| {
                if self.action_button(
                    ui,
                    if self.preview_active {
                        "Stop Preview Screen"
                    } else {
                        "Start Preview Screen"
                    },
                    HotkeyAction::TogglePreview,
                ) {
                    self.preview_active = !self.preview_active;
                }

                if self.action_button(ui, "⚙ Settings", HotkeyAction::ClosePopup) {
                    self.show_config = true;
                }
                self.render_recording_controls(ui);
            });
        });

        // Render the config window if it's open
        self.render_config_window(ctx);

        ui.vertical_centered(|ui| {
            if self.preview_active {
                if let Some(display_frame) = self.frame_grabber.next_frame(self.capture_area) {
                    // Record if active
                    if self.video_recorder.is_recording() {
                        self.video_recorder.record_frame(&display_frame);
                    }

                    // Convert to ColorImage for display
                    let image: ColorImage = egui::ColorImage::from_rgba_premultiplied(
                        [display_frame.width, display_frame.height],
                        &display_frame.rgba_data,
                    );

                    // Update texture in memory
                    match self.display_texture {
                        Some(ref mut texture) => {
                            texture.set(image, egui::TextureOptions::default());
                        }
                        None => {
                            self.display_texture = Some(ctx.load_texture(
                                "display_texture",
                                image,
                                egui::TextureOptions::default(),
                            ));
                        }
                    }
                }

                // Update texture in UI
                let texture = self
                    .display_texture
                    .as_ref()
                    .unwrap_or(self.textures.get(&TextureId::Error).unwrap());
                ui.add(egui::Image::new(texture).max_size(self.get_preview_screen_rect(ui).size()));
            } else {
                self.display_texture = None;
                self.cropped_frame = None;
            }
        });
    }

    pub fn receiver_page(&mut self, ui: &mut egui::Ui) {
        ui.disable();
        ui.heading("Receiver Mode");
        ui.vertical_centered(|ui| {
            ui.label("Enter the Sender's IP Address");
            if ui
                .text_edit_singleline(&mut self.address_text)
                .on_disabled_hover_text("NOT IMPLEMENTED")
                .lost_focus()
            {
                ui.label(format!("Address:{}", self.address_text));
                log::debug!("Address: {}", self.address_text);
            }
        });
        ui.button("Connect")
            .on_disabled_hover_text("NOT IMPLEMENTED")
            .on_hover_cursor(egui::CursorIcon::NotAllowed);
    }

    fn start_recording(&mut self) {
        // Start audio capture first
        if let Err(e) = self.audio_capturer.start() {
            log::error!("Failed to start audio capture: {}", e);
            return;
        }

        // Then start video recording
        self.video_recorder.start();
    }

    fn stop_recording(&mut self) {
        // Stop audio capture first and get the buffer
        self.audio_capturer.stop();
        let audio_buffer = self.audio_capturer.take_audio_buffer();

        // Pass audio buffer to video recorder before stopping
        if !audio_buffer.is_empty() {
            if let Err(e) = 
                self.video_recorder.process_audio(audio_buffer
            ) {
                log::error!("Failed to process audio: {}", e);
            }
        }

        // Finally stop video recording
        self.video_recorder.stop();
    }



    fn action_button(&mut self, ui: &mut egui::Ui, label: &str, action: HotkeyAction) -> bool {
        // Check if Alt is pressed for underline
        let alt_pressed = ui.input(|i| i.modifiers.alt);

        // Get hotkey text if exists
        let hotkey_text = format!(
            " ({})",
            self.hotkey_manager
                .get_shortcut_text(&action)
                .unwrap_or_default()
        );

        // Create full text for size calculation
        let full_text = format!("{}{}", label, hotkey_text);

        // Calculate size with padding
        let galley = ui.painter().layout_no_wrap(
            full_text.clone(),
            egui::TextStyle::Button.resolve(ui.style()),
            egui::Color32::PLACEHOLDER,
        );
        let padding = ui.spacing().button_padding;
        let min_size = egui::vec2(
            galley.size().x + padding.x * 2.0,
            galley.size().y + padding.y * 2.0,
        );

        // Create displayed text (with or without hotkey)
        let display_text = if alt_pressed {
            full_text
        } else {
            label.to_string()
        };

        // Create button with fixed minimum size
        ui.add_sized(min_size, egui::Button::new(display_text))
            .clicked()
            || self.triggered_actions.contains(&action)
    }

    fn clickable_element<T>(
        &mut self,
        ui: &mut egui::Ui,
        label: T,
        action: HotkeyAction,
        show_hotkey: bool,
    ) -> bool
    where
        T: Into<RichText>,
    {
        let alt_pressed: bool = ui.input(|i| i.modifiers.alt);

        let base_text: RichText = label.into();

        // Only show hotkey if enabled and alt is pressed
        let combined_text = if show_hotkey && alt_pressed {
            if let Some(hotkey) = self.hotkey_manager.get_shortcut_text(&action) {
                RichText::new(format!("{}{}", base_text.text(), hotkey))
            } else {
                base_text
            }
        } else {
            base_text
        };

        let response = ui
            .add(egui::Label::new(combined_text))
            .on_hover_cursor(egui::CursorIcon::PointingHand);

        response.clicked() || self.triggered_actions.contains(&action)
    }

    /// Add a texture to the texture map
    /// If the texture fails to load, an error texture is loaded instead
    /// The error texture is a red square
    /// The error texture is loaded only once and is reused for all errors
    /// # Arguments
    /// * `textures` - A mutable reference to the texture map
    /// * `ctx` - A reference to the egui context
    /// * `name` - The name of the texture
    /// * `img_bytes` - The bytes of the image to load
    /// * `texture_options` - Optional texture options
    ///
    /// # Panics
    /// If the error texture is not found
    ///
    /// # Example
    /// ```rust
    /// let mut textures = HashMap<TextureId, TextureHandle>::new();
    /// let ctx = egui::Context::new();
    /// let img_bytes = include_bytes!("../assets/icons/home.svg");
    /// add_texture_to_map(&mut textures, &ctx, TextureId::HomeIcon, img_bytes, None);
    /// ```
    fn add_texture_to_map(
        textures: &mut HashMap<TextureId, TextureHandle>,
        ctx: &Context,
        resource: &TextureResource,
        texture_options: Option<egui::TextureOptions>,
    ) {
        let image: ColorImage = match egui_extras::image::load_svg_bytes(resource.path) {
            Ok(img) => img,
            Err(e) => {
                log::error!("Failed to load image: {}", e);
                if let Some(error_texture) = textures.get(&TextureId::default()) {
                    textures.insert(resource.id, error_texture.clone());
                    return;
                } else {
                    log::error!("Default Texture not found.");
                    ColorImage::new([50, 50], egui::Color32::RED)
                }
            }
        };

        let loaded_texture = ctx.load_texture(
            resource.id.to_string(),
            image,
            texture_options.unwrap_or_default(),
        );
        textures.insert(resource.id, loaded_texture);
    }
}

impl eframe::App for RustreamApp {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        if let Some(action) = self.hotkey_manager.handle_input(ctx) {
            self.triggered_actions.push(action);
        }
        TopBottomPanel::top("header")
            // .frame(
            //     Frame::none()
            //         .fill(ctx.style().visuals.window_fill())
            //         .inner_margin(8.0),
            // )
            .show(ctx, |ui| {
                self.render_header(ui);
            });

        CentralPanel::default().show(ctx, |ui| match self.page {
            PageView::HomePage => self.home_page(ui),

            PageView::Caster => self.caster_page(ui, ctx, frame),

            PageView::Receiver => self.receiver_page(ui),
        });

        self.triggered_actions.clear();
        ctx.request_repaint();
    }
}

