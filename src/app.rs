use crate::common::CaptureArea;
use crate::config::Config;
use crate::data_streaming::{start_receiving, start_streaming, Receiver, Sender, PORT};
use crate::hotkey::{HotkeyAction, HotkeyManager, KeyCombination};
use crate::screen_capture::{CapturedFrame, ScreenCapture};
use crate::video_recorder::VideoRecorder;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot::{channel, error::TryRecvError};
use tokio::sync::Notify;
// use std::time::Duration;

use eframe::egui;
use egui::{
    CentralPanel, Color32, ColorImage, ComboBox, Context, FontId, Rect, RichText, TextStyle,
    TextureHandle, TopBottomPanel, Ui, Window,
};

use std::env;
use std::process::Command;

use display_info::DisplayInfo;
use log::{debug, error, info};

pub struct RustreamApp {
    pub config: Arc<Mutex<Config>>,
    pub received_frames: Arc<Mutex<VecDeque<CapturedFrame>>>,
    pub stop_notify: Arc<Notify>, // Notify to stop the frame receiving task
    frame_grabber: ScreenCapture,
    video_recorder: VideoRecorder,
    page: PageView,                                       // Enum to track modes
    display_texture: Option<TextureHandle>,               // Texture for the screen capture
    textures: HashMap<TextureId, TextureHandle>,          // List of textures
    captured_frames: Arc<Mutex<VecDeque<CapturedFrame>>>, // Queue of captured frames
    address_text: String,
    caster_addr: Option<SocketAddr>, // Text input for the receiver mode
    streaming_active: bool,
    is_selecting: bool,
    cropped_frame: Option<CapturedFrame>,
    capture_area: Option<CaptureArea>,
    show_config: bool, // Add this field
    sender: Option<Arc<tokio::sync::Mutex<Sender>>>,
    receiver: Option<Arc<tokio::sync::Mutex<Receiver>>>,
    sender_rx: Option<tokio::sync::oneshot::Receiver<Arc<tokio::sync::Mutex<Sender>>>>,
    receiver_rx: Option<tokio::sync::oneshot::Receiver<Receiver>>,
    socket_created: bool,
    last_frame_time: Option<std::time::Instant>,
    frame_times: std::collections::VecDeque<std::time::Duration>,
    current_fps: f32,
    is_receiving: bool,
    started_capture: bool,
    hotkey_manager: HotkeyManager,
    editing_hotkey: Option<HotkeyAction>,
    triggered_actions: Vec<HotkeyAction>,
    previous_monitor: usize,
    is_address_valid: bool,
    host_unreachable: Arc<AtomicBool>,
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

        RustreamApp {
            config,
            frame_grabber,
            video_recorder,
            textures,
            sender: None,
            sender_rx: None,
            streaming_active: false,
            socket_created: false,
            receiver: None,
            receiver_rx: None,
            last_frame_time: None,
            frame_times: std::collections::VecDeque::with_capacity(60),
            current_fps: 0.0,
            received_frames: Arc::new(Mutex::new(VecDeque::new())),
            stop_notify: Arc::new(Notify::new()),
            is_receiving: false,
            captured_frames: Arc::new(Mutex::new(VecDeque::new())),
            started_capture: false,
            hotkey_manager: HotkeyManager::new(),
            page: PageView::HomePage,
            display_texture: None,
            address_text: String::new(),
            is_selecting: false,
            capture_area: None,
            show_config: false,
            editing_hotkey: None,
            triggered_actions: Vec::new(),
            previous_monitor: 0,
            caster_addr: None,
            cropped_frame: None,
            is_address_valid: true,
            host_unreachable: Arc::new(AtomicBool::new(false)),
        }
    }

    fn get_preview_screen_rect(&self, ui: &egui::Ui) -> Rect {
        // Adjust this based on how your preview is laid out
        // For example, occupy the full available space
        ui.available_rect_before_wrap()
    }

    fn reset_ui(&mut self) {
        // Reset the application when rertuning to the home page
        self.frame_grabber.reset_capture();
        self.page = PageView::default();
        self.address_text.clear();
        self.sender = None;
        self.socket_created = false;
        self.streaming_active = false;
        self.started_capture = false;
        //self.frame_rx = None;
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
                                .size(25.0)
                                .strong(),
                        )
                        .min_size(egui::vec2(300.0, 50.0)),
                    )
                    .clicked()
                {
                    self.set_page(PageView::Caster);
                }

                ui.add_space(30.0);
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("VIEW STREAMING").size(25.0).strong(),
                        )
                        .min_size(egui::vec2(300.0, 50.0)),
                    )
                    .clicked()
                {
                    self.set_page(PageView::Receiver);
                }

                ui.add_space(30.0);
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

                // Display Settings Section
                ui.heading("Display Settings");
                ui.separator();

                // Monitor selection
                let selected_monitor = &mut config.capture.selected_monitor;
                let current_monitor = *selected_monitor;

                ComboBox::from_label("Monitor")
                    .selected_text(format!("Monitor {}", selected_monitor))
                    .show_ui(ui, |ui| {
                        self.frame_grabber
                            .get_monitors()
                            .iter()
                            .enumerate()
                            .for_each(|(i, m)| {
                                if ui.selectable_value(selected_monitor, i, m).clicked()
                                    && current_monitor != self.previous_monitor
                                {
                                    self.capture_area = None;
                                    self.previous_monitor = current_monitor;
                                }
                            });
                    });

                // Capture Area Section
                ui.add_space(10.0);
                ui.label(RichText::new(if self.capture_area.is_some() {
                    "Selected Capture Area"
                } else {
                    "No Capture Area Selected - Use 'Select Capture Area' button to define region"
                }).size(16.0));

                ui.horizontal(|ui| {
                    self.is_selecting ^= ui.button("Select Capture Area").clicked();
                    if self.capture_area.is_some() {
                        ui.add_space(10.0);
                        if ui.button("Reset Capture Area").clicked() {
                            self.capture_area = None;
                        }
                    }
                });

                if let Some(area) = self.capture_area {
                    egui::Grid::new("capture_area_grid")
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Position:");
                            ui.label(format!("X: {} px, Y: {} px", area.x, area.y));
                            ui.end_row();

                            ui.label("Size:");
                            ui.label(format!("{}x{} px", area.width, area.height));
                            ui.end_row();
                        });
                }

                // Handle capture area selection process
                if self.is_selecting {
                    self.handle_capture_area_selection(selected_monitor);
                    self.is_selecting = false;
                }

                // Hotkey Settings Section
                ui.add_space(20.0);
                ui.heading("Hotkey Settings");
                ui.separator();

                egui::Grid::new("hotkeys_grid")
                    .num_columns(3)
                    .spacing([40.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Action");
                        ui.label("Shortcut");
                        ui.label("Controls");
                        ui.end_row();

                        let actions: Vec<_> = self
                            .hotkey_manager
                            .default_shortcuts
                            .values()
                            .filter(|action| action.is_visible())
                            .cloned()
                            .collect();

                        for action in actions {
                            ui.label(action.to_string());

                            let (combo_text, is_default) = self
                                .hotkey_manager
                                .shortcuts
                                .iter()
                                .find(|(_, a)| *a == &action)
                                .map(|(k, _)| {
                                    let text = k.to_string();
                                    let is_default = self.hotkey_manager.is_default(k, &action);
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
                                if ui
                                    .add_enabled(!is_default, egui::Button::new("↺"))
                                    .clicked()
                                {
                                    self.hotkey_manager.reset_action(&action);
                                }
                            });
                            ui.end_row();
                        }
                    });

                if ui.button("Reset to Default Hotkeys").clicked() {
                    self.hotkey_manager.reset_to_defaults();
                }

                // Hotkey editing popup
                if let Some(editing_action) = &self.editing_hotkey {
                    self.show_hotkey_popup(ctx, editing_action.clone());
                }

                // Apply changes
                if self.config.lock().unwrap().clone() != config {
                    debug!("Config changed: {:?}", config);
                    self.config.lock().unwrap().update(config);
                    self.frame_grabber.reset_capture();
                }
            });

        self.show_config = show_config;
    }

    // Helper method to show hotkey popup
    fn show_hotkey_popup(&mut self, ctx: &Context, editing_action: HotkeyAction) {
        Window::new("Configure Hotkey")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!(
                    "Press new key combination for {:?}",
                    editing_action
                ));
                ui.label("Press Esc to cancel");

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

    // Helper method to handle capture area selection
    fn handle_capture_area_selection(&mut self, selected_monitor: &usize) {
        let displays = DisplayInfo::all().unwrap_or_default();
        let display = match displays.get(*selected_monitor) {
            Some(d) => d,
            None => {
                error!("Monitor not found: {}", selected_monitor);
                return;
            }
        };

        info!(
            "Display: {} ({},{}) ({}x{}) | scale factor: {}",
            display.name, display.x, display.y, display.width, display.height, display.scale_factor
        );

        let output = match Command::new(env::current_exe().unwrap())
            .arg("--overlay:selection")
            .arg(display.x.to_string())
            .arg(display.y.to_string())
            .arg(display.width.to_string())
            .arg(display.height.to_string())
            .arg(display.scale_factor.to_string())
            .output()
        {
            Ok(output) => output,
            Err(e) => {
                error!("Failed to execute selection process: {}", e);
                return;
            }
        };

        if output.status.success() {
            match std::str::from_utf8(&output.stdout)
                .map_err(|e| error!("Failed to read stdout: {}", e))
                .and_then(|stdout| {
                    debug!("Main process received: {}", stdout);
                    serde_json::from_str(stdout)
                        .map_err(|e| error!("Failed to parse JSON response: {}", e))
                }) {
                Ok(json_response) => self.process_selection_response(json_response),
                Err(_) => error!("Failed to process selection response"),
            }
        } else if let Ok(stderr) = std::str::from_utf8(&output.stderr) {
            if !stderr.is_empty() {
                error!("Secondary process error: {}", stderr);
            }
        }
    }

    fn render_recording_settings(&mut self, ctx: &Context) {
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
                    ComboBox::from_label("")
                        .selected_text(format!("{} FPS", config.video.fps))
                        .show_ui(ui, |ui| {
                            for &fps in &[6, 24, 25, 30, 50, 60] {
                                ui.selectable_value(
                                    &mut config.video.fps,
                                    fps,
                                    format!("{} FPS", fps),
                                );
                            }
                        });
                });

                // Apply changes if the config has changed
                let has_config_changed: bool = self.config.lock().unwrap().clone() != config;
                if has_config_changed {
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
            //TODO: add toggle preview to save resources
            ui.horizontal(|ui| {
                if self.action_button(
                    ui,
                    if self.streaming_active {
                        "Stop Streaming"
                    } else {
                        "Start Streaming"
                    },
                    HotkeyAction::ToggleStreaming,
                ) {
                    self.streaming_active = !self.streaming_active;
                }

                if self.action_button(ui, "🖊 Annotation", HotkeyAction::Annotation) {
                    let selected_monitor = self.config.lock().unwrap().capture.selected_monitor;
                    let displays = DisplayInfo::all().unwrap_or_default();
                    //info!("Displays: {:?}", displays);
                    let display = displays.get(selected_monitor).unwrap_or_else(|| {
                        error!("Monitor not found: {}", selected_monitor);
                        std::process::exit(1);
                    });

                    #[allow(unused_must_use)]
                    Command::new(env::current_exe().unwrap())
                        .arg("--overlay:annotation")
                        .arg(display.x.to_string())
                        .arg(display.y.to_string())
                        .arg(display.width.to_string())
                        .arg(display.height.to_string())
                        .arg(display.scale_factor.to_string())
                        .spawn();
                }

                if self.action_button(ui, "⚙ Settings", HotkeyAction::ClosePopup) {
                    self.show_config = true;
                }
                ui.add_space(50.0);
            });
        });

        // Render the config window if it's open
        self.render_config_window(ctx);

        ui.vertical_centered(|ui| {
            let cap_frames = self.captured_frames.clone();

            if !self.started_capture {
                //call capture_frame only once
                self.started_capture = true;
                //FIXME: capture area need to be sent across thread
                self.frame_grabber.capture_frame(cap_frames);
            }

            let mut frames = self.captured_frames.lock().unwrap();
            if let Some(display_frame) = frames.pop_front() {
                //front().cloned()
                if frames.len() >= 7 {
                    println!("Captured_Frames len: {}, dropping frames", frames.len());
                    frames.clear() //truncate(3); //only leave 3 elements
                }

                drop(frames);

                if self.streaming_active {
                    // Initialize sender if it doesn't exist
                    if self.sender.is_none() && !self.socket_created {
                        let (tx, rx) = channel();
                        self.socket_created = true;

                        tokio::spawn(async move {
                            let sender = Sender::new().await;
                            let _ = tx.send(Arc::new(tokio::sync::Mutex::new(sender)));
                        });

                        //store rx to poll it later to see if initialization completed, since the channel sender is async
                        self.sender_rx = Some(rx);
                    }

                    // Check if we have a pending sender initialization
                    if let Some(mut rx) = self.sender_rx.take() {
                        //take consumes the sender_rx
                        // Try to receive the sender
                        if let Ok(sender) = rx.try_recv() {
                            self.sender = Some(sender);
                        } else {
                            // Put the receiver back if we haven't received yet
                            self.sender_rx = Some(rx);
                        }
                    }

                    // Send frame if we have a sender
                    if let Some(sender) = &self.sender {
                        //i redo the check to extract the sender from Option<Sender>
                        let sender_clone = sender.clone();
                        //let mut frames = self.captured_frames.lock().unwrap();

                        /*for frame in frames.drain(..) { // Take all frames for streaming
                            let sender = sender.clone();
                            tokio::spawn(async move {
                                if let Err(e) = start_streaming(sender, frame).await {
                                    eprintln!("Error sending frame: {}", e);
                                }
                            });
                        }*/

                        // Store the cropped frame if a capture area is selected
                        if let Some(area) = self.capture_area {
                            self.cropped_frame = display_frame.clone().view(
                                area.x as u32,
                                area.y as u32,
                                area.width as u32,
                                area.height as u32,
                            );
                        }
                        // Send a cropped frame if we have one, otherwise send the full frame
                        let clone_frame =
                            self.cropped_frame.clone().unwrap_or(display_frame.clone());
                        tokio::spawn(async move {
                            if let Err(e) = start_streaming(sender_clone, clone_frame).await {
                                eprintln!("Error sending frame: {}", e);
                            }
                        });
                    }
                }

                // Convert to ColorImage for display
                let image: ColorImage = if let Some(area) = self.capture_area {
                    // Apply cropping if we have a capture area
                    if let Some(cropped) = display_frame.clone().view(
                        area.x as u32,
                        area.y as u32,
                        area.width as u32,
                        area.height as u32,
                    ) {
                        egui::ColorImage::from_rgba_unmultiplied(
                            [cropped.width, cropped.height],
                            &cropped.rgba_data, // Changed from frame_data to rgba_data
                        )
                    } else {
                        // Fallback to full image if crop parameters are invalid
                        egui::ColorImage::from_rgba_unmultiplied(
                            [display_frame.width, display_frame.height],
                            &display_frame.rgba_data, // Changed from frame_data to rgba_data
                        )
                    }
                } else {
                    // No crop area selected, show full image
                    egui::ColorImage::from_rgba_unmultiplied(
                        [display_frame.width, display_frame.height],
                        &display_frame.rgba_data, // Changed from frame_data to rgba_data
                    )
                };

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
                ctx.request_repaint();
            }

            // Update texture in UI
            let texture = self
                .display_texture
                .as_ref()
                .unwrap_or(self.textures.get(&TextureId::Error).unwrap());
            ui.add(egui::Image::new(texture).max_size(self.get_preview_screen_rect(ui).size()));
        });
    }

    pub fn receiver_page(&mut self, ctx: &Context, _ui: &mut Ui) {
        self.show_fps_counter(ctx);
        // Render the recording settings window if it's open
        self.render_recording_settings(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Receiver Mode");
            ui.vertical_centered(|ui| {
                if !self.is_receiving {
                    ui.label(RichText::new("Enter the Sender's IP Address").size(15.0));
                    ui.add_space(10.0);

                    let connect_button = egui::Button::new(
                        egui::RichText::new("Connect")
                            .color(egui::Color32::WHITE)
                            .size(15.0),
                    )
                    .fill(egui::Color32::from_rgb(0, 200, 0))
                    .min_size(egui::vec2(60.0, 30.0));

                    ui.style_mut().text_styles.insert(
                        TextStyle::Body,
                        FontId::new(15.0, egui::FontFamily::Proportional),
                    );
                    ui.add_sized(
                        egui::vec2(300.0, 30.0), // Width: 300, Height: 30
                        egui::TextEdit::singleline(&mut self.address_text).frame(true),
                    );
                    ui.add_space(20.0);

                    //if connect button is clicked
                    if ui
                        .add_enabled(!self.address_text.trim().is_empty(), connect_button)
                        .clicked()
                    {
                        //check if inserted address is valid
                        if let Ok(addr) = self.address_text.parse::<Ipv4Addr>() {
                            self.is_address_valid = true;
                            let caster_addr = SocketAddr::new(IpAddr::V4(addr), PORT);
                            self.caster_addr = Some(caster_addr);

                            //clear the previous frame queue to prevent frames from previous streaming from being displayed
                            let mut frames = self.received_frames.lock().unwrap();
                            frames.clear();
                            drop(frames);

                            let (tx, rx) = channel();

                            // Initialize receiver
                            tokio::spawn(async move {
                                let receiver = Receiver::new(caster_addr).await;
                                let _ = tx.send(receiver);
                            });

                            //store rx to poll it later to see if initialization completed, since the channel sender is async
                            self.receiver_rx = Some(rx);
                            self.is_receiving = true;
                        } else {
                            self.is_address_valid = false;
                        }
                    }
                    //show Invalid IP Address message if the address is not valid
                    if !self.is_address_valid {
                        ui.add_space(20.0);
                        ui.label(
                            RichText::new("Invalid IP Address")
                                .color(Color32::RED)
                                .size(15.0),
                        );
                    }
                } else {
                    //receiving already started
                    // Show Stop, Start Recording and Recording Settings buttons
                    ui.horizontal(|ui| {
                        ui.vertical_centered(|ui| {
                            let stop_button = egui::Button::new(
                                egui::RichText::new("Stop")
                                    .color(egui::Color32::WHITE)
                                    .size(15.0),
                            )
                            .fill(egui::Color32::from_rgb(200, 0, 0))
                            .min_size(egui::vec2(60.0, 30.0));

                            if ui.add(stop_button).clicked() {
                                self.stop_notify.notify_waiters();
                                self.host_unreachable.store(false, Ordering::SeqCst);
                                self.receiver = None;
                                self.receiver_rx = None;
                                self.display_texture = None;
                                self.is_receiving = false;
                                let mut frames = self.received_frames.lock().unwrap();
                                frames.clear();
                                self.last_frame_time = None;
                                self.frame_times.clear();
                                self.current_fps = 0.0;
                            }
                        });

                        // Push "Start Recording" to the right
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(10.0);
                            self.render_recording_controls(ui);

                            ui.add_space(5.0);
                            if self.action_button(
                                ui,
                                "⚙ Recording Settings",
                                HotkeyAction::ClosePopup,
                            ) {
                                self.show_config = true;
                            }
                        });
                    });
                }

                // Show Host Unreachable message if the host is unreachable
                if self.host_unreachable.load(Ordering::SeqCst) {
                    ui.add_space(20.0);
                    ui.label(
                        RichText::new("Host Unreachable")
                            .color(Color32::RED)
                            .size(15.0),
                    );
                }

                // Check if we have a pending receiver initialization
                if let Some(mut rx) = self.receiver_rx.take() {
                    //take consumes the receiver_rx

                    // Try to receive the receiver
                    match rx.try_recv() {
                        Ok(receiver) => {
                            println!("Receiver initialized successfully");
                            self.receiver = Some(Arc::new(tokio::sync::Mutex::new(receiver)));
                        }
                        Err(TryRecvError::Empty) => {
                            // Put the channel receiver back if we haven't received yet
                            self.receiver_rx = Some(rx);
                        }
                        Err(TryRecvError::Closed) => {
                            ui.label(
                                RichText::new("Failed to receive receiver from channel")
                                    .color(Color32::RED),
                            );
                            self.receiver_rx = None;
                        }
                    }
                }

                // Start/continue receiving if we have a receiver
                if let Some(receiver) = &mut self.receiver {
                    //i redo the check to extract the sender from Option<Sender>

                    let receiver_clone = receiver.clone();
                    let rcv_frames = self.received_frames.clone();
                    let stop_notify = self.stop_notify.clone();
                    let host_unreachable = self.host_unreachable.clone();

                    tokio::spawn(async move {
                        let receiver = receiver_clone.lock().await;

                        //start receiving only if it's the first time
                        if !receiver.started_receiving {
                            drop(receiver); //drop the lock before starting the receiving task
                            start_receiving(
                                rcv_frames,
                                receiver_clone,
                                stop_notify,
                                host_unreachable,
                            )
                            .await;
                        }
                    });

                    // Retrieve the latest frame from the queue
                    let frame = {
                        //in this way the lock is released immediately
                        let mut frames = self.received_frames.lock().unwrap();
                        frames.pop_front()
                    };

                    if let Some(frame) = frame {
                        if self.video_recorder.is_recording() {
                            self.video_recorder.record_frame(&frame);
                        }

                        // Convert to ColorImage for display
                        let image = egui::ColorImage::from_rgba_unmultiplied(
                            [frame.width, frame.height],
                            &frame.rgba_data,
                        );

                        // Update texture in memory
                        if let Some(ref mut texture) = self.display_texture {
                            texture.set(image, egui::TextureOptions::default());
                            //println!("texture updated");
                        } else {
                            self.display_texture = Some(ctx.load_texture(
                                "display_texture",
                                image,
                                egui::TextureOptions::default(),
                            ));
                            //println!("texture loaded");
                        }

                        // Update FPS counter
                        self.update_fps_counter();
                    } else {
                        // Add a loading indicator while waiting for receiver initialization
                        if self.display_texture.is_none()
                            && !self.host_unreachable.load(Ordering::SeqCst)
                        {
                            ui.add_space(40.0);
                            ui.add_sized(egui::vec2(30.0, 30.0), egui::Spinner::new()); // Show a spinner while connecting
                            ui.label(RichText::new("Connecting to sender...").size(15.0));
                        }
                    }
                    ctx.request_repaint();
                }
                // Update texture in UI
                let texture = self
                    .display_texture
                    .as_ref()
                    .unwrap_or(self.textures.get(&TextureId::Error).unwrap());
                ui.add(egui::Image::new(texture).max_size(self.get_preview_screen_rect(ui).size()));
            });
        });
    }

    fn update_fps_counter(&mut self) {
        let now = std::time::Instant::now();
        if let Some(last_frame_time) = self.last_frame_time {
            let frame_time = now.duration_since(last_frame_time);
            self.frame_times.push_back(frame_time);

            // Keep only last 60 frame times for moving average
            if self.frame_times.len() > 60 {
                self.frame_times.pop_front();
            }

            // Calculate average FPS
            if !self.frame_times.is_empty() {
                let avg_frame_time: std::time::Duration =
                    self.frame_times.iter().sum::<std::time::Duration>()
                        / self.frame_times.len() as u32;
                self.current_fps = 1.0 / avg_frame_time.as_secs_f32();
            }
        }
        self.last_frame_time = Some(now);
    }

    fn start_recording(&mut self) {
        // Then start video recording
        self.video_recorder.start();
    }

    fn stop_recording(&mut self) {
        self.video_recorder.stop();
    }

    fn action_button(&mut self, ui: &mut egui::Ui, label: &str, action: HotkeyAction) -> bool {
        // Get hotkey text if exists
        let hotkey_text = format!(
            " ({})",
            self.hotkey_manager
                .get_shortcut_text(&action)
                .unwrap_or_default()
        );

        // Calculate size with padding for the label only
        let galley = ui.painter().layout_no_wrap(
            label.to_string(),
            egui::TextStyle::Button.resolve(ui.style()),
            egui::Color32::PLACEHOLDER,
        );
        let padding = ui.spacing().button_padding;
        let min_size = egui::vec2(
            galley.size().x + padding.x * 2.0,
            galley.size().y + padding.y * 2.0,
        );

        // Create button with fixed minimum size and hover text
        let response = ui
            .add_sized(
                min_size,
                egui::Button::new(egui::RichText::new(label.to_string()).size(15.0)),
            )
            .on_hover_text(format!("{}{}", label, hotkey_text));

        response.clicked() || self.triggered_actions.contains(&action)
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

    fn show_fps_counter(&self, ctx: &Context) {
        egui::TopBottomPanel::top("fps_counter").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.current_fps > 0.0 {
                        ui.colored_label(
                            egui::Color32::GREEN,
                            format!("FPS: {:.1}", self.current_fps),
                        );
                    }
                });
            });
        });
    }

    fn process_selection_response(&mut self, json_response: serde_json::Value) {
        match json_response["status"].as_str() {
            Some("success") => {
                if let Some(data) = json_response.get("data") {
                    // Parse capture area data with detailed error handling
                    let capture_area = serde_json::from_value(data.clone()).unwrap_or_else(|e| {
                        error!("Failed to parse capture area data: {}", e);
                        error!("Possible struct mismatch between SecondaryApp and main process");
                        error!(
                            "Expected format: {{x: usize, y: usize, width: usize, height: usize}}"
                        );
                        None
                    });
                    self.capture_area = capture_area;
                }
            }
            Some("cancelled") => {
                debug!("User cancelled the capture operation");
            }
            _ => {
                error!("Unknown status in response");
            }
        }
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
                error!("Failed to load image: {}", e);
                if let Some(error_texture) = textures.get(&TextureId::default()) {
                    textures.insert(resource.id, error_texture.clone());
                    return;
                } else {
                    error!("Default Texture not found.");
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

            PageView::Receiver => self.receiver_page(ctx, ui),
        });

        self.triggered_actions.clear();
        //ctx.request_repaint();
        ctx.request_repaint_after(Duration::from_millis(200));
    }
}
