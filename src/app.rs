use crate::common::CaptureArea;
use crate::config::Config;
use crate::frame_grabber::{CapturedFrame, FrameGrabber};
use crate::video_recorder::VideoRecorder;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;
use serde_json::json;

use eframe::egui;
use egui::{
    CentralPanel, Color32, ColorImage, ComboBox, Context, FontId, Pos2, Rect, RichText,
    TextureHandle, TopBottomPanel, Ui, Window,
};

use std::env;
use std::io::Write;
use std::process::Command;

lazy_static! {
    pub static ref GLOBAL_CAPTURE_AREA: Arc<Mutex<CaptureArea>> =
        Arc::new(Mutex::new(CaptureArea::default()));
}

#[derive(Default)]
pub struct RustreamApp {
    pub config: Arc<Mutex<Config>>,
    frame_grabber: FrameGrabber,
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

#[derive(Default, Debug)]
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
        let frame_grabber: FrameGrabber = FrameGrabber::new(config.clone());
        let video_recorder: VideoRecorder = VideoRecorder::new(config.clone());

        RustreamApp {
            config,
            frame_grabber,
            video_recorder,
            textures,
            ..Default::default()
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

    fn home_page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);

                if ui.button("CAST NEW STREAMING").clicked() {
                    self.set_page(PageView::Caster);
                }

                ui.add_space(30.0);

                if ui.button("VIEW STREAMING").clicked() {
                    self.set_page(PageView::Receiver);
                }
            });
        });
    }

    fn render_config_window(&mut self, ctx: &Context) {
        let mut show_config = self.show_config;

        Window::new("Configuration")
            .open(&mut show_config)
            .resizable(false)
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

                // TODO: Select capture area
                ui.horizontal(|ui| {
                    self.is_selecting ^= ui.button("Select Capture Area").clicked();

                    if self.is_selecting {
                        println!("{:?}", self.capture_area);
                        //creating a transparent window with egui-overlay

                        // Open a new full-size window for selecting capture area
                        //check if the process with arg --secondary is opened yet

                        let output = Command::new(env::current_exe().unwrap())
                            .arg("--secondary")
                            .output()
                            .expect("failed to execute process");

                            if output.status.success() {
                                // Parse stdout with error handling
                                let stdout = std::str::from_utf8(&output.stdout).unwrap_or_else(|e| {
                                    eprintln!("Failed to read stdout: {}", e);
                                    ""
                                });
                                println!("Main process received: {}", stdout);
                            
                                // Parse JSON with detailed error handling
                                let json_response: serde_json::Value = serde_json::from_str(stdout).unwrap_or_else(|e| {
                                    eprintln!("Failed to parse JSON response: {}", e);
                                    serde_json::json!({ "status": "error" })
                                });
                                
                                match json_response["status"].as_str() {
                                    Some("success") => {
                                        if let Some(data) = json_response.get("data") {
                                            // Detailed error handling for struct mismatch
                                            let capture_area = serde_json::from_value(data.clone()).unwrap_or_else(|e| {
                                                eprintln!("Failed to parse capture area data: {}", e);
                                                eprintln!("Possible struct mismatch between SecondaryApp and main process");
                                                eprintln!("Expected format: {{x: usize, y: usize, width: usize, height: usize}}");
                                                None
                                            });
                                            self.capture_area = capture_area;
                                        }
                                    }
                                    Some("cancelled") => {
                                        println!("User cancelled the capture operation");
                                    }
                                    _ => {
                                        eprintln!("Unknown status in response");
                                    }
                                }
                            } else {
                                // Handle process errors
                                match std::str::from_utf8(&output.stderr) {
                                    Ok(stderr) if !stderr.is_empty() => {
                                        eprintln!("Secondary process error: {}", stderr);
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to read stderr: {}", e);
                                    }
                                    _ => {
                                        eprintln!("Secondary process failed with no error output");
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

                ui.heading("Recording Settings");
                ui.separator();

                // Monitor selection
                let selected_monitor = &mut config.capture.selected_monitor;
                ComboBox::from_label("Monitor")
                    .selected_text(format!("Monitor {}", selected_monitor))
                    .show_ui(ui, |ui| {
                        self.frame_grabber
                            .get_monitors()
                            .iter()
                            .enumerate()
                            .for_each(|(i, m)| {
                                ui.selectable_value(selected_monitor, i, m);
                            });
                    });

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
                            .set_file_name("recording.mp4")
                            .add_filter("MP4 video", &["mp4"])
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

        if ui
            .button(if recording {
                "⏹ Stop Recording"
            } else {
                "⏺ Start Recording"
            })
            .clicked()
        {
            if recording {
                self.video_recorder.stop();
            } else {
                self.video_recorder.start();
            }
        }
    }
    fn render_caster_page(&mut self, ui: &mut egui::Ui, ctx: &Context, _frame: &mut eframe::Frame) {
        // show the selected monitor as continuous feedback of frames
        ui.heading("Monitor Feedback");
        ui.separator();
        ui.vertical_centered(|ui| {
            ui.horizontal(|ui| {
                self.preview_active ^= ui
                    .button(if self.preview_active {
                        "Stop Preview Screen"
                    } else {
                        "Start Preview Screen"
                    })
                    .clicked();

                if ui.button("⚙ Settings").clicked() {
                    self.show_config = true;
                }
                self.render_recording_controls(ui);

                let output_path = self.config.lock().unwrap().video.output_path.clone();
                let dir_name = output_path
                    .parent()
                    .map(|p| p.to_string_lossy())
                    .unwrap_or_default();

                ui.add(egui::Label::new("📂 ".to_string() + &dir_name))
                    .on_hover_text(output_path.to_string_lossy());
            });
        });

        // Render the config window if it's open
        self.render_config_window(ctx);

        ui.vertical_centered(|ui| {
            if self.preview_active {
                if let Ok(display_frame) = self.frame_grabber.capture_frame(self.capture_area) {
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

    pub fn render_receiver_mode(&mut self, ui: &mut egui::Ui) {
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
        TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    // Home button
                    if ui
                        .add_sized(
                            [80., 30.],
                            egui::Button::image_and_text(
                                &self.textures.get(&TextureId::HomeIcon).unwrap().clone(),
                                "🏠 Home",
                            ),
                        )
                        .clicked()
                    {
                        self.reset_ui();
                    }

                    // Quit button
                    if ui
                        .add_sized(
                            [80., 30.],
                            egui::Button::image_and_text(
                                &self.textures.get(&TextureId::QuitIcon).unwrap().clone(),
                                "🚪 Quit",
                            ),
                        )
                        .clicked()
                    {
                        std::process::exit(0);
                    }
                });

                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("Welcome to RUSTREAM!")
                            .font(FontId::proportional(40.0))
                            .color(Color32::GOLD),
                    );
                });
            });
        });
        CentralPanel::default().show(ctx, |ui| match self.page {
            PageView::HomePage => self.home_page(ui),

            PageView::Caster => self.render_caster_page(ui, ctx, frame),

            PageView::Receiver => self.render_receiver_mode(ui),
        });

        ctx.request_repaint();
    }
}

#[derive(Default)]
pub struct SecondaryApp {
    is_selecting: bool,
    capture_area: Option<CaptureArea>,
    drag_start: Option<Pos2>,
    new_capture_area: Option<Rect>,
    show_popup: bool,
}

impl eframe::App for SecondaryApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // let mut app = self.rustream_app.lock().unwrap();

        // Handle Alt+F4 and window close events
        if ctx.input(|i| i.viewport().close_requested()) {
            println!("{{\"status\": \"cancelled\"}}\n");
            std::io::stdout().flush().unwrap();
            std::process::exit(0);
        }

        // Add Esc key handling
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            println!("{{\"status\": \"cancelled\"}}\n");
            std::io::stdout().flush().unwrap();
            std::process::exit(0);
        }

        let scale_factor = ctx.pixels_per_point();

        // Add tutorial window
        egui::Window::new("Tutorial")
            .fixed_pos([10.0, 10.0])
            .title_bar(false)
            .frame(egui::Frame::none().fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180)))
            .show(ctx, |ui| {
                ui.colored_label(
                    egui::Color32::WHITE,
                    "How to select an area:\n\
                     1. Click and drag to select the desired area\n\
                     2. Release to confirm the selection\n\
                     3. Click OK to capture or Cancel to try again\n\
                     Press ESC at any time to exit",
                );
            });

        CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let response =
                    ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::drag());

                // Handle drag start
                if response.drag_started() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let screen_pos = egui::pos2(pos.x * scale_factor, pos.y * scale_factor);
                        self.drag_start = Some(screen_pos);
                    }
                    self.show_popup = false;
                }

                // Handle dragging and drawing
                if let Some(start) = self.drag_start {
                    if let Some(current) = response.interact_pointer_pos() {
                        let screen_pos =
                            egui::pos2(current.x * scale_factor, current.y * scale_factor);
                        self.new_capture_area = Some(egui::Rect::from_two_pos(start, screen_pos));
                    }
                }

                // Draw persistent border rectangle
                if let Some(rect) = self.new_capture_area {
                    let display_rect = egui::Rect::from_min_max(
                        egui::pos2(rect.min.x / scale_factor, rect.min.y / scale_factor),
                        egui::pos2(rect.max.x / scale_factor, rect.max.y / scale_factor),
                    );

                    // Draw semi-transparent border
                    ui.painter().rect_stroke(
                        display_rect,
                        0.0,
                        egui::Stroke::new(
                            3.0,
                            egui::Color32::from_rgba_unmultiplied(0, 255, 0, 128),
                        ),
                    );

                    if response.drag_stopped() {
                        self.show_popup = true;
                    }
                }

                // Show centered popup
                if self.show_popup && self.new_capture_area.is_some() {
                    let rect = self.new_capture_area.unwrap();
                    egui::Window::new("Confirm Selection")
                        .fixed_size([300.0, 150.0])
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .resizable(false)
                        .collapsible(false)
                        .show(ctx, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add_space(20.0);
                                ui.heading("Do you want to confirm this selection?");
                                ui.add_space(30.0);

                                let total_width = 240.0;
                                let button_width = 100.0;
                                let spacing = (total_width - (2.0 * button_width)) / 2.0;

                                ui.allocate_ui_with_layout(
                                    egui::vec2(total_width, 40.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .add_sized(
                                                [button_width, 40.0],
                                                egui::Button::new("OK"),
                                            )
                                            .clicked()
                                        {
                                            let output = CaptureArea::new(
                                                (rect.min.x).round() as usize,
                                                (rect.min.y).round() as usize,
                                                (rect.width()).round() as usize,
                                                (rect.height()).round() as usize,
                                            );

                                            let output_with_status = json!({
                                                "status": "success",
                                                "data": output
                                            });

                                            println!(
                                                "{}\n",
                                                serde_json::to_string(&output_with_status).unwrap()
                                            );
                                            std::io::stdout().flush().unwrap();

                                            self.capture_area = Some(output);
                                            self.show_popup = false;
                                            self.is_selecting = false;
                                            self.drag_start = None;
                                            self.new_capture_area = None;
                                            std::process::exit(0);
                                        }

                                        ui.add_space(spacing);

                                        if ui
                                            .add_sized(
                                                [button_width, 40.0],
                                                egui::Button::new("Cancel"),
                                            )
                                            .clicked()
                                        {
                                            self.show_popup = false;
                                            self.is_selecting = false;
                                            self.drag_start = None;
                                            self.new_capture_area = None;
                                        }
                                    },
                                );
                            });
                        });
                }
            });
    }
}
