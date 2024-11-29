use crate::config::Config;
use crate::screen_capture::{CapturedFrame, FrameGrabber};
use crate::video_recorder::VideoRecorder;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use eframe::egui;
use egui::{
    CentralPanel, Color32, ColorImage, ComboBox, Context, FontId, Pos2, Rect, RichText,
    TextureHandle,
};

use log::debug;

const NUM_TEXTURES: usize = 3;

#[derive(Default)]
pub struct RustreamApp {
    config: Arc<Mutex<Config>>, // Wrap in Mutex for interior mutability
    frame_grabber: FrameGrabber,
    video_recorder: VideoRecorder,
    page: PageView,                           // Enum to track modes
    display_texture: Option<TextureHandle>,   // Texture for the screen capture
    textures: HashMap<String, TextureHandle>, // List of textures
    cropped_frame: Option<CapturedFrame>,     // Cropped image to send
    address_text: String,                     // Text input for the receiver mode
    preview_active: bool,
    should_quit: bool,
    is_selecting: bool,
    drag_start: Option<Pos2>,
    capture_area: Option<(u32, u32, u32, u32)>,
    new_capture_area: Option<Rect>,
}

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

        let mut textures = HashMap::with_capacity(NUM_TEXTURES);
        RustreamApp::add_texture_to_map(
            &mut textures,
            ctx,
            "error",
            include_bytes!("../assets/icons/error.svg"),
            None,
        );

        RustreamApp::add_texture_to_map(
            &mut textures,
            ctx,
            "home_icon",
            include_bytes!("../assets/icons/home.svg"),
            None,
        );

        RustreamApp::add_texture_to_map(
            &mut textures,
            ctx,
            "quit_icon",
            include_bytes!("../assets/icons/quit.svg"),
            None,
        );

        assert_eq!(
            textures.len(),
            NUM_TEXTURES,
            r"Numbers of Textures Declared: {} | Actual number of textures: {}, 
            Check: 
                1. If the `NUM_TEXTURES` is correct
                2. If the texture name is unique
                3. Try again and pray to the Rust gods",
            NUM_TEXTURES,
            textures.len()
        );

        let config = Arc::new(Mutex::new(Config::default()));
        let frame_grabber = FrameGrabber::new(config.clone());
        let video_recorder = VideoRecorder::new(config.clone());

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
        self.frame_grabber = FrameGrabber::new(self.config.clone());
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

    fn render_caster_page(&mut self, ui: &mut egui::Ui, ctx: &Context, _frame: &mut eframe::Frame) {
        // show the selected monitor as continuous feedback of frames
        ui.heading("Monitor Feedback");

        ui.add_space(40.0);

        ui.vertical_centered(|ui| {
            let mut selected_monitor_index = self.frame_grabber.get_monitor_index();
            ComboBox::from_label("Select Monitor")
                .selected_text(format!("Monitor {}", selected_monitor_index))
                .show_ui(ui, |ui| {
                    self.frame_grabber
                        .get_monitors()
                        .iter()
                        .enumerate()
                        .for_each(|(i, m)| {
                            ui.selectable_value(&mut selected_monitor_index, i, m);
                        });
                });

            if selected_monitor_index != self.frame_grabber.get_monitor_index() {
                self.frame_grabber.set_monitor_index(selected_monitor_index);
            }

            // TODO: Select capture area
            if self.preview_active {
                self.is_selecting ^= ui.button("Select Capture Area").clicked();
                if self.is_selecting {
                    // Save current window size
                }
            }
            if self.is_selecting {
                // TODO: Select capture area
                // display a rectangle to show the selected area
                let response = ui.allocate_rect(ctx.available_rect(), egui::Sense::drag());

                // display a rectangle to show the selected area
                if response.drag_started() {
                    self.drag_start = Some(response.interact_pointer_pos().unwrap());
                }

                if let Some(start) = self.drag_start {
                    if let Some(current) = response.interact_pointer_pos() {
                        self.new_capture_area = Some(egui::Rect::from_two_pos(start, current));
                        // Draw the selection rectangle
                        if let Some(rect) = self.new_capture_area {
                            ui.painter().rect_filled(
                                rect,
                                0.0,
                                egui::Color32::from_rgba_premultiplied(0, 255, 0, 100),
                            );
                            ui.painter().rect_stroke(
                                rect,
                                0.0,
                                egui::Stroke::new(2.0, egui::Color32::GREEN),
                            );
                        }
                    }
                }

                // OK button to confirm selection
                if self.new_capture_area.is_some() && ui.button("OK").clicked() {
                    self.capture_area = (
                        self.new_capture_area.unwrap().min.x as u32,
                        self.new_capture_area.unwrap().min.y as u32,
                        self.new_capture_area.unwrap().width() as u32,
                        self.new_capture_area.unwrap().height() as u32,
                    )
                        .into();
                    log::debug!(
                        "Capture Area: x:{}, y:{}, width:{}, height:{}",
                        self.capture_area.unwrap().0,
                        self.capture_area.unwrap().1,
                        self.capture_area.unwrap().2,
                        self.capture_area.unwrap().3
                    );
                    self.is_selecting = false;
                    self.drag_start = None;
                    self.new_capture_area = None;
                }
                // Cancel selection
                if ui.button("Cancel").clicked() {
                    self.is_selecting = false;
                    self.new_capture_area = None;
                    self.drag_start = None;
                }
            }

            // Update capture area in config when it changes
            if let Some(area) = self.capture_area {
                let mut config = self.config.lock().unwrap();
                config.capture.capture_area = Some(area);
            }

            // TODO: Select capture area
            // Disable the preview button if the user is not selecting the capture area
            // Disable the preview button if the user is not selecting the capture area
            // if !self.is_selecting {
            self.preview_active ^= ui
                .button(if self.preview_active {
                    "Stop Preview Screen"
                } else {
                    "Start Preview Screen"
                })
                .clicked();
            // }

            if self.preview_active {
                if let Some(screen_image) = self.frame_grabber.capture_frame() {
                    let image: ColorImage = if let Some((x, y, width, height)) = self.capture_area {
                        // Apply cropping if we have a capture area
                        if let Some(cropped) = screen_image.view(x, y, width, height) {
                            egui::ColorImage::from_rgba_unmultiplied(
                                [cropped.width as usize, cropped.height as usize],
                                &cropped.rgba_data, // Changed from frame_data to rgba_data
                            )
                        } else {
                            // Fallback to full image if crop parameters are invalid
                            egui::ColorImage::from_rgba_unmultiplied(
                                [screen_image.width as usize, screen_image.height as usize],
                                &screen_image.rgba_data, // Changed from frame_data to rgba_data
                            )
                        }
                    } else {
                        // No crop area selected, show full image
                        egui::ColorImage::from_rgba_unmultiplied(
                            [screen_image.width as usize, screen_image.height as usize],
                            &screen_image.rgba_data, // Changed from frame_data to rgba_data
                        )
                    };

                    // Store the active frame for network transmission if needed
                    self.cropped_frame = if let Some((x, y, width, height)) = self.capture_area {
                        screen_image.view(x, y, width, height)
                    } else {
                        Some(screen_image)
                    };

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
                }

                let texture = self
                    .display_texture
                    .as_ref()
                    .unwrap_or(self.textures.get("error").unwrap());
                ui.add(egui::Image::new(texture).max_size(self.get_preview_screen_rect(ui).size()));
            } else {
                self.display_texture = None;
                self.cropped_frame = None;
            }

            // Add recording controls
            ui.horizontal(|ui| {
                if self.video_recorder.is_recording() {
                    if ui.button("⏹ Stop Recording").clicked() && self.video_recorder.stop() {
                        debug!("Recording stopped and saved successfully");
                    }
                } else if ui.button("⏺ Start Recording").clicked() {
                    self.video_recorder.start();
                }
            });

            ui.horizontal(|ui| {
                ui.label("Output path:");
                let mut recording_path = self
                    .config
                    .lock()
                    .unwrap()
                    .video
                    .output_path
                    .to_string_lossy()
                    .into_owned();
                // ? Beware: if the path use unicode characters, it may not be displayed correctly
                ui.text_edit_singleline(&mut recording_path)
                    // Show a tooltip with the full path when hovering over the text field (useful if it's too long)
                    .on_hover_text(recording_path);
                if ui.button("📂 Browse").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Save recording as...")
                        .set_file_name("recording.mp4")
                        .add_filter("MP4 video", &["mp4"])
                        .save_file()
                    {
                        let mut config = self.config.lock().unwrap();
                        config.video.output_path = path;
                    }
                }
            });

            {
                let mut config = self.config.lock().unwrap();
                ui.add(
                    egui::DragValue::new(&mut config.video.fps)
                        .range(1..=60)
                        .suffix(" FPS"),
                );
            }

            // Record frame if we're recording
            if self.video_recorder.is_recording() {
                if let Some(frame) = self.cropped_frame.as_ref() {
                    self.video_recorder.record_frame(frame);
                }
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
                debug!("Address: {}", self.address_text);
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
    /// let mut textures = HashMap::new();
    /// let ctx = egui::Context::new();
    /// let img_bytes = include_bytes!("../assets/icons/home.svg");
    /// add_texture_to_map(&mut textures, &ctx, "home_icon", img_bytes, None);
    /// ```
    fn add_texture_to_map(
        textures: &mut HashMap<String, TextureHandle>,
        ctx: &Context,
        name: &str,
        img_bytes: &[u8],
        texture_options: Option<egui::TextureOptions>,
    ) {
        let image: ColorImage = match egui_extras::image::load_svg_bytes(img_bytes) {
            Ok(img) => img,
            Err(e) => {
                log::warn!("Failed to load image: {}", e);
                if let Some(error_texture) = textures.get("error") {
                    textures.insert(name.to_string(), error_texture.clone());
                    return;
                } else {
                    log::warn!("Error Texture not found. Loading RED SQUARE as error texture");
                    ColorImage::new([50, 50], egui::Color32::RED)
                }
            }
        };

        let texture = ctx.load_texture(name, image, texture_options.unwrap_or_default());
        textures.insert(name.to_string(), texture.clone());
    }
}

impl eframe::App for RustreamApp {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        if self.should_quit {
            // Spawn a new thread to send the close command to the egui context
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            });
        }
        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    // Home button
                    if ui
                        .add_sized(
                            [80., 30.],
                            egui::Button::image_and_text(
                                &self.textures.get("home_icon").unwrap().clone(),
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
                                &self.textures.get("quit_icon").unwrap().clone(),
                                "🚪 Quit",
                            ),
                        )
                        .clicked()
                    {
                        self.should_quit = true;
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
            ui.separator();
            ui.add_space(20.0);

            match self.page {
                PageView::HomePage => self.home_page(ui),

                PageView::Caster => self.render_caster_page(ui, ctx, frame),

                PageView::Receiver => self.render_receiver_mode(ui),
            }
        });
    }
}
