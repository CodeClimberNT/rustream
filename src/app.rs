use crate::audio_capture::AudioCapturer;
use crate::common::CaptureArea;
use crate::config::Config;
use crate::hotkey::{HotkeyAction, HotkeyManager, KeyCombination};
use crate::screen_capture::{CapturedFrame, ScreenCapture};
use crate::video_recorder::VideoRecorder;
use crate::data_streaming::{Sender, Receiver, start_streaming, start_receiving, PORT};
use tokio::sync::oneshot::{channel, error::TryRecvError};
use tokio::sync::Notify;
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use std::collections::VecDeque;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
// use std::time::Duration;

use lazy_static::lazy_static;

use eframe::egui;
use egui::{
    CentralPanel, Color32, ColorImage, ComboBox, Context, FontId, Pos2, Rect, RichText, TextStyle, TextureHandle, TopBottomPanel, Ui, Window
};

use std::env;
use std::process::Command;

use display_info::DisplayInfo;

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
    preview_active: bool,
    cropped_frame: Option<CapturedFrame>,        // Cropped image to send
    address_text: String,     
    caster_addr: Option<SocketAddr>,                   // Text input for the receiver mode
    //preview_active: bool,
    streaming_active: bool,
    is_selecting: bool,
    drag_start: Option<Pos2>,
    capture_area: Option<CaptureArea>,
    show_config: bool, // Add this field
    sender: Option<Arc<Sender>>,
    receiver: Option<Arc<tokio::sync::Mutex<Receiver>>>,
    sender_rx: Option<tokio::sync::oneshot::Receiver<Arc<Sender>>>,
    receiver_rx: Option<tokio::sync::oneshot::Receiver<Receiver>>,
    socket_created: bool,
    //frame_rx: Option<tokio::sync::oneshot::Receiver<CapturedFrame>>,
    last_frame_time: Option<std::time::Instant>,
    frame_times: std::collections::VecDeque<std::time::Duration>,
    current_fps: f32,
    pub received_frames: Arc<Mutex<VecDeque<CapturedFrame>>>,
    //frame_ready: bool,
    pub stop_notify: Arc<Notify>,
    is_receiving: bool,
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
            sender: None,
            sender_rx: None,
            streaming_active: false,
            socket_created: false,
            //frame_rx: None,
            receiver: None,
            receiver_rx: None,
            last_frame_time: None,
            frame_times: std::collections::VecDeque::with_capacity(60),
            current_fps: 0.0,
            received_frames: Arc::new(Mutex::new(VecDeque::new())),
            //frame_ready: false,
            stop_notify: Arc::new(Notify::new()),
            is_receiving: false,
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
            caster_addr: None,
            drag_start: None,
            
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
                            if ui.selectable_value(selected_monitor, i, m).clicked() && current_monitor != self.previous_monitor {
                                    self.capture_area = None;
                                    self.previous_monitor = current_monitor;
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
                        //log::info!("Displays: {:?}", displays);
                        let display = displays.get(*selected_monitor).unwrap_or_else(|| {
                            log::error!("Monitor not found: {}", selected_monitor);
                            std::process::exit(1);
                        });
                        //display name + x and y
                        log::info!("Display: {} ({},{}) ({}x{}) {}", display.name, display.x, display.y,display.width,display.height, display.scale_factor);
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
                    if self.streaming_active {
                        "Stop Streaming"
                    } else {
                        "Start Streaming"
                    },
                    HotkeyAction::TogglePreview, //FIXME: Streaming action
                ) {
                    self.streaming_active = !self.streaming_active;
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
            // if self.preview_active {
                if let Some(display_frame) = self.frame_grabber.next_frame(self.capture_area) {
                    // Record if active
                    if self.video_recorder.is_recording() {
                        self.video_recorder.record_frame(&display_frame);
                    }
                    
                if self.streaming_active {
                    // Initialize sender if it doesn't exist
                    /*if s.is_none() {
                        let runtime = tokio::runtime::Runtime::new().unwrap();
                        runtime.block_on(async {
                            let sender = Arc::new(Sender::new().await);
                            self.sender = Some(sender);
                        });
                    }*/
                    
                    // Initialize sender if it doesn't exist
                    if self.sender.is_none() && !self.socket_created {
                        let  (tx, rx) = channel();
                        //let tx_clone = tx.clone();
                        self.socket_created = true;

                        tokio::spawn(async move {
                            let sender = Sender::new().await;
                            let _ = tx.send(Arc::new(sender));
                        });
                        //store rx to poll it later to see if initialization completed, since the channel sender is async
                        self.sender_rx = Some(rx);   
                    }

                    // Check if we have a pending sender initialization
                    if let Some(mut rx) = self.sender_rx.take() {  //take consumes the sender_rx
                        // Try to receive the sender
                        if let Ok(sender) = rx.try_recv() {
                            self.sender = Some(sender);
                        }
                        else {
                            // Put the receiver back if we haven't received yet
                            self.sender_rx = Some(rx);
                        }
                    } 

                    // Send frame if we have a sender
                    if let Some(sender) = &self.sender { //i redo the check to extract the sender from Option<Sender>
                        let sender_clone = sender.clone();
                        let clone_frame = display_frame.clone();
                        tokio::spawn(async move {
                            if let Err(e) = start_streaming(sender_clone, clone_frame).await {
                                eprintln!("Error sending frame: {}", e);
                            }
                        });
                    }
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

            // } else {
                //     self.display_texture = None;
                //     self.cropped_frame = None;
                // }
        });
    }

    pub fn receiver_page(&mut self, ctx: &Context) {

        // Add a container for the entire window content
        egui::TopBottomPanel::top("fps_counter").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.current_fps > 0.0 {
                        ui.colored_label(egui::Color32::GREEN, format!("FPS: {:.1}", self.current_fps));
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Receiver Mode");
            ui.vertical_centered(|ui| {

                if !self.is_receiving {
                    ui.label("Enter the Sender's IP Address");
                    ui.add_space(10.0);

                    let connect_button = egui::Button::new(
                        egui::RichText::new("Connect").color(egui::Color32::WHITE).size(15.0)
                    )
                    .fill(egui::Color32::from_rgb(0, 200, 0))
                    .min_size(egui::vec2(60.0, 30.0));

                    //ui.vertical_centered(|ui| {
                        ui.horizontal(|ui| {
                            ui.add_space( 150.0);
                            ui.style_mut().text_styles.insert(
                                TextStyle::Body,
                                FontId::new(15.0, egui::FontFamily::Proportional), // 24 logical points
                            );
                            ui.add_sized(
                                egui::vec2(300.0, 30.0), // Width: 300, Height: 40
                                egui::TextEdit::singleline(&mut self.address_text)
                                .frame(true)
                            );
                            ui.add_space(20.0);

                            if ui.add_enabled(!self.address_text.trim().is_empty(), connect_button).clicked(){

                                //check if inserted address is valid
                                if let Ok(addr) = self.address_text.parse::<Ipv4Addr>() {
                                    let caster_addr = SocketAddr::new(IpAddr::V4(addr), PORT);
                                    self.caster_addr = Some(caster_addr); 
                            
                                    // Initialize receiver
                                    //if self.receiver.is_none()  {  //&& !self.socket_created
                                        let  (tx, rx) = channel();
                                        
                                        tokio::spawn(async move {
                                            let receiver = Receiver::new(caster_addr).await;

                                            let _ = tx.send(receiver);
                                        });

                                        //store rx to poll it later to see if initialization completed, since the channel sender is async
                                        self.receiver_rx = Some(rx); 
                                        
                                    //}

                                    //let tx_clone = tx.clone();
                                    //tokio::spawn(async move {
                                        /*
                                        if !self.socket_created {
                                        }
                                        let socket = connect_to_sender(caster_addr).await;
                                        match socket {
                                            Ok(socket) => {
                                                
                                                println!("Connected to Sender");

                                                match recv_data(socket).await {
                                                    Ok(frame) => {
                                                        print!("Frame received from receiver");
                                                        let _ = tx_clone.send(frame);
                                                        drop(tx_clone);
                                                    }
                                                    Err(e) => {
                                                        eprintln!("Failed to receive data: {}", e);
                                                    }
                                                }
                                                /*if let Err(e) = recv_data(caster_addr, socket).await {
                                                    eprintln!("Failed to receive data: {}", e);
                                                }*/
                                            }
                                            Err(e) => {
                                                println!("No data received");
                                                println!("{}", e);
                                            }
                                        }*/
                                    //});

                                    /*match rx.try_recv() { //eseguita anche se la socket non si è connessa
                                        Ok(frame) => {
                                            println!("Frame received from channel");
                                            
                                            let image: ColorImage =  egui::ColorImage::from_rgba_unmultiplied(
                                                [frame.width as usize, frame.height as usize],
                                                &frame.rgba_data );

                                            // Update texture
                                            if let Some(ref mut texture) = self.display_texture {
                                                texture.set(image, egui::TextureOptions::default());
                                            } else {
                                                self.display_texture = Some(ctx.load_texture(
                                                    "display_texture",
                                                    image,
                                                    egui::TextureOptions::default(),
                                                ));
                                            }

                                            let texture = self
                                                .display_texture
                                                .as_ref()
                                                .unwrap_or(self.textures.get("error").unwrap());
                                            ui.add(egui::Image::new(texture).max_size(self.get_preview_screen_rect(ui).size()));
                                            
                                        },
                                        Err(_) => println!("the sender dropped or no data received"),
                                    }*/
                                    self.is_receiving = true;
                                }
                                else {
                                    ui.label(RichText::new("Invalid IP Address").color(Color32::RED));
                                    //come faccio a farla comparire per più tempo?? scompare in un secondo
                                }

                                /*// Add a loading indicator while waiting for receiver initialization
                                if self.socket_created && self.receiver.is_none() {
                                    ui.spinner(); // Show a spinner while connecting
                                    ui.label("Connecting to sender...");
                                }*/

                                

                                
                                //check if inserted address is valid
                                /*if let Ok(addr) = self.address_text.parse::<Ipv4Addr>() {
                                    let caster_addr = SocketAddr::new(IpAddr::V4(addr), PORT);    
                                    
                                    let (tx, rx) = channel();
                                    self.frame_rx = Some(rx);

                                    //let mut addr_vec: Vec<&str> = self.address_text.split(".").collect();
                                    ///let port  = addr_vec[3].split(":").collect::<Vec<&str>>()[1];
                                    ///addr_vec[3] = addr_vec[3].split(":").collect::<Vec<&str>>()[0];

                                    /*let caster_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(
                                        addr_vec[0].parse::<u8>().unwrap(), 
                                        addr_vec[1].parse::<u8>().unwrap(), 
                                        addr_vec[2].parse::<u8>().unwrap(), 
                                        addr_vec[3].parse::<u8>().unwrap())), 
                                        PORT);  //port.parse::<u16>().unwrap()*/
                                    
                                    //let  (tx, mut rx) = channel();
                                    
                                    tokio::spawn(async move {
                                        let socket = connect_to_sender(caster_addr).await;
                                        match socket {
                                            Ok(socket) => {
                                                println!("Connected to Sender");

                                                match recv_data(socket).await {
                                                    Ok(frame) => {
                                                        println!("Received data");
                                                        //let _ = tx.send(frame);
                                                    }
                                                    Err(e) => {
                                                        eprintln!("Failed to receive data: {}", e);
                                                    }
                                                }
                                                /*if let Err(e) = recv_data(caster_addr, socket).await {
                                                    eprintln!("Failed to receive data: {}", e);
                                                }*/
                                            }
                                            Err(_) => {
                                                println!("No data received");
                                            }
                                        }
                                    });
                                } 
                                else {
                                    ui.label(RichText::new("Invalid IP Address").color(Color32::RED));
                                }*/    
                            }

                            
                        });
                    //});
                } else {

                    let stop_button = egui::Button::new(
                        egui::RichText::new("Stop").color(egui::Color32::WHITE).size(15.0)
                    )
                    .fill(egui::Color32::from_rgb(200, 0, 0))
                    .min_size(egui::vec2(60.0, 30.0));
                    
                    if ui.add(stop_button).clicked() {
                        println!("Stop clicked");
                        self.stop_notify.notify_waiters(); //in this way recv_data releases the lock
                        
                        if let Some(receiver) = &self.receiver {
                            let receiver = receiver.clone();
                            tokio::spawn(async move {
                                let recv = receiver.lock().await;
                                recv.stop_receiving();                               
                            });                            
                        }
                        //these are not executed after the tokio::spawn, so the last texture is not erased, how to fix it??
                        self.receiver = None; //non lo elimino, lo sovrascrivo direttamente, altrimenti il tokio spawn fa casino
                        self.receiver_rx = None;
                        self.display_texture = None;
                        self.is_receiving = false;
                        //clear the frame queue
                        let mut frames = self.received_frames.lock().unwrap();
                        frames.clear();
                        self.last_frame_time = None;
                        self.frame_times.clear();
                        self.current_fps = 0.0;
                        
                    }
                    ui.add_space(10.0);
                }

                // Check if we have a pending receiver initialization
                if let Some(mut rx) = self.receiver_rx.take() {  //take consumes the receiver_rx
                    
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
                            ui.label(RichText::new("Failed to receive receiver from channel").color(Color32::RED));
                            //self.socket_created = false;
                            self.receiver_rx = None;
                        }
                    }
                }

                // Start/continue receiving if we have a receiver
                if let Some(receiver) = &mut self.receiver { //i redo the check to extract the sender from Option<Sender>
                    let receiver_clone = receiver.clone();

                    //let  (tx, rx) = channel(); //oneshot
                    //self.frame_rx = Some(rx);

                    let rcv_frames = self.received_frames.clone();

                    //if let Some(caster_addr) = self.caster_addr {
                        let stop_notify = self.stop_notify.clone();
                    
                        tokio::spawn(async move {
                            
                            let receiver_clone2 = Arc::clone(&receiver_clone);
                            let receiver = receiver_clone.lock().await;
                            //let frame;
                            
                            //let rx = receiver.frame_rx.take();
                            
                            //caster address changed, stop previous streaming and start the new one
                            /*if receiver.caster != caster_addr { 
                                receiver.caster = caster_addr;
                                drop(receiver);
                                println!("Caster address changed, reconnecting to new sender");
                                //connect the socket to the new address and start receiving
                                start_receiving(rcv_frames, receiver_clone2, false, stop_notify).await;
                              */
                              //start receiving only if it's the first time
                            //} else 
                            if !receiver.started_receiving { 
                                //drop the lock before starting the receiving task
                                drop(receiver);
                                //println!("Receiving from the same sender");
                                start_receiving(rcv_frames, receiver_clone2, stop_notify).await;
                                
                            }
                            //println!("After start_receiving completed");
                            /*if let Some(frame) = frame {
                                println!("Frame received from start_receiving");
                                let mut frames = rcv_frames.lock().unwrap();
                                println!("Frame queue length: {}", frames.len());
                                frames.push_back(frame); //insert new frame in the queue
                                //self.frame_ready = true; //come mantengo questo stato?
                                //println!("Frame queue lock 1 acquired");

                                /*match tx.send(frame) {
                                    Ok(_) => println!("🔍 Frame successfully sent to channel"),
                                    Err(_) => println!("❌ Failed to send frame to channel"),
                                }*/
                            }*/
                            /*let rcv = rcv_clone.lock().await;
                            println!("last Lock on receiver acquired");
                            let mut frame_vec = rcv.frames.lock().await; 
                            println!("Lock on frames acquired");
                            println!("Frame queue length: {}", frame_vec.len());
                            if let Some(frame) = frame_vec.pop_front(){ // retrieve the oldest frame first  
                                println!("Frame popped from queue");
                                drop(frame_vec);
                                //let _ = tx_clone.send(frame);
                            }*/
                        }); 

                        //while let Some(rx) = &mut self.frame_rx {
                        
                            //if let Ok(frame) = rx.try_recv() {
                            let frame = { //in this way the lock is released immediately
                                let mut frames = self.received_frames.lock().unwrap();
                                //println!("Frame queue lock 2 acquired");
                                //println!("Frame queue size before pop: {}", frames.len());
                                frames.pop_front()
                            };
                            //let mut frames = self.received_frames.lock().unwrap();
                            
                                if let Some(frame) = frame {
                                    println!("Frame popped from mutex");
                                    //self.frame_rx = None;
                                    //println!("Frame received from mutex");
                                    let image = egui::ColorImage::from_rgba_unmultiplied(
                                        [frame.width as usize, frame.height as usize], 
                                        &frame.rgba_data
                                    );
                                    println!("image created");
                                    
                                    // Update FPS counter
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
                                            let avg_frame_time: std::time::Duration = self.frame_times.iter().sum::<std::time::Duration>() 
                                                / self.frame_times.len() as u32;
                                            self.current_fps = 1.0 / avg_frame_time.as_secs_f32();
                                        }
                                    }
                                    self.last_frame_time = Some(now);

                                    // Update texture
                                    if let Some(ref mut texture) = self.display_texture {
                                        texture.set(image, egui::TextureOptions::default());
                                        println!("texture updated");
                                    } else {
                                        self.display_texture = Some(ctx.load_texture(
                                            "display_texture",
                                            image,
                                            egui::TextureOptions::default(),
                                        ));
                                        println!("texture loaded");
                                    }
                                    
                                }   
                                ctx.request_repaint();                 
                        //}
                    //} 

                    
                    /*tokio::spawn(async move {
                        let rcv = rcv_clone.lock().await;
                        let mut frame_vec = rcv.frames.lock().await; 
                        if let Some(frame) = frame_vec.pop_front(){ // retrieve the oldest frame first  
                            println!("Frame popped from queue {:?}", frame);
                            //let _ = tx_clone.send(frame);
                        }
                    });  */

                    
                }
                
                

                let texture = self
                    .display_texture
                    .as_ref()
                    .unwrap_or(self.textures.get(&TextureId::Error).unwrap());
                ui.add(egui::Image::new(texture).max_size(self.get_preview_screen_rect(ui).size()));
                

                // Display FPS counter
                if self.current_fps > 0.0 {
                    ui.label(format!("FPS: {:.1}", self.current_fps));
                }
                            
            });
        });
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

            PageView::Receiver => self.receiver_page(ctx),
        });

        self.triggered_actions.clear();
        ctx.request_repaint();
        // ctx.request_repaint_after(Duration::from_millis(1000));
    }
}

