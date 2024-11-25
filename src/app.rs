use crate::screen_capture::ScreenCapture;

// Importing all the necessary libraries
// self -> import the module egui itself
use eframe::egui;
use egui::{CentralPanel, Color32, ColorImage, ComboBox, Context, FontId, RichText, TextureHandle};
use log::debug;

pub struct RustreamApp {
    screen_capturer: ScreenCapture, // List of monitors as strings for display in the menu
    mode: PageView,                 // Enum to track modes
    // Texture
    screen_texture: Option<TextureHandle>, // Texture for the screen capture
    error_texture: TextureHandle,          // Texture for the screen capture
    home_icon_texture: TextureHandle,
    quit_icon_texture: TextureHandle,
    address_text: String, // Text input for the receiver mode
    is_rendering_screen: bool,
    should_quit: bool,
}

#[derive(Default, Debug)]
pub enum PageView {
    #[default]
    HomePage,
    Sender,
    Receiver,
}

impl RustreamApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // let svg_home_icon_path: &'static str = "../assets/icons/home.svg";
        let ctx: &Context = &cc.egui_ctx;
        egui_extras::install_image_loaders(ctx);

        let error_icon_texture = RustreamApp::load_texture(
            ctx,
            "error_icon",
            include_bytes!("../assets/icons/error.svg"),
            None,
            None,
        );

        let home_icon_texture = RustreamApp::load_texture(
            ctx,
            "home_icon",
            include_bytes!("../assets/icons/home.svg"),
            Some(&error_icon_texture),
            None,
        );
        let quit_icon_texture = RustreamApp::load_texture(
            ctx,
            "quit_icon",
            include_bytes!("../assets/icons/quit.svg"),
            Some(&error_icon_texture),
            None,
        );

        let screen_capture = ScreenCapture::default();

        RustreamApp {
            screen_capturer: screen_capture,
            screen_texture: None,
            error_texture: error_icon_texture,
            mode: PageView::default(),
            address_text: String::new(),
            is_rendering_screen: false,
            home_icon_texture,
            quit_icon_texture,
            should_quit: false,
        }
    }

    pub fn width(&self, ctx: &Context) -> f32 {
        ctx.screen_rect().width()
    }

    #[allow(dead_code)]
    pub fn height(&self, ctx: &Context) -> f32 {
        ctx.screen_rect().height()
    }

    pub fn reset_ui(&mut self) {
        // Reset the application
        self.screen_capturer.reset();
        self.mode = PageView::default();
        self.address_text.clear();
    }

    pub fn set_mode(&mut self, mode: PageView) {
        self.mode = mode;
    }

    pub fn render_home_page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);

                if ui.button("CAST NEW STREAMING").clicked() {
                    self.set_mode(PageView::Sender);
                }

                ui.add_space(30.0);

                if ui.button("VIEW STREAMING").clicked() {
                    self.set_mode(PageView::Receiver);
                }
            });
        });
    }

    pub fn render_sender_page(&mut self, ui: &mut egui::Ui) {
        // TODO: better screen recording method than taking a screenshot
        // show the selected monitor as continuous feedback of frames
        ui.heading("Monitor Feedback");

        ui.add_space(40.0);

        ui.vertical_centered(|ui| {
            let mut selected = self.screen_capturer.get_monitor_index();

            let current_monitor = selected;
            ComboBox::from_label("Select Monitor")
                .selected_text(format!("Monitor {}", current_monitor))
                .show_ui(ui, |ui| {
                    self.screen_capturer
                        .get_monitors()
                        .iter()
                        .enumerate()
                        .for_each(|(i, m)| {
                            ui.selectable_value(&mut selected, i, m);
                        });
                });
            if selected != current_monitor {
                self.screen_capturer.set_monitor_index(selected);
            }

            // Toggle the rendering of the screenshot when the button is clicked and update the button text
            self.is_rendering_screen ^= ui
                .button(if self.is_rendering_screen {
                    "Stop Capture"
                } else {
                    "Start Capture"
                })
                .clicked();

            if self.is_rendering_screen {
                // take_screenshot(self.selected_monitor);
                if let Some(screen_image) = self.screen_capturer.capture_screen() {
                    let image: ColorImage = egui::ColorImage::from_rgba_unmultiplied(
                        [
                            screen_image.width() as usize,
                            screen_image.height() as usize,
                        ],
                        screen_image.as_flat_samples().as_slice(),
                    );

                    if let Some(ref mut texture) = self.screen_texture {
                        // Update existing texture
                        texture.set(image, egui::TextureOptions::default());
                    } else {
                        // Load texture for the first time
                        self.screen_texture = Some(ui.ctx().load_texture(
                            "screen_image",
                            image,
                            egui::TextureOptions::default(),
                        ));
                    }
                }

                let texture = self.screen_texture.as_ref().unwrap_or(&self.error_texture);
                ui.add(egui::Image::new(texture).max_width(self.width(ui.ctx()) / 1.5));
            } else {
                self.screen_texture = None;
            }
        });
    }

    pub fn render_receiver_mode(&mut self, ui: &mut egui::Ui) {
        ui.heading("Receiver Mode");
        ui.vertical_centered(|ui| {
            ui.label("Enter the Sender's IP Address");
            if ui.text_edit_singleline(&mut self.address_text).lost_focus() {
                ui.label(format!("Address:{}", self.address_text.clone()));
                debug!("Address: {}", self.address_text);
            }
        });
        ui.button("Connect")
            .on_hover_text("NOT IMPLEMENTED")
            .on_hover_cursor(egui::CursorIcon::NotAllowed);
    }

    /// Utility function to load a texture from image bytes.
    /// If loading fails, it returns the provided error_texture.
    ///
    /// # Arguments
    ///
    /// * `ctx` - A reference to egui::Context.
    /// * `name` - The unique name for the texture.
    /// * `img_bytes` - The image data as a byte slice.
    /// * `error_texture` - A reference to the fallback error texture.
    ///
    /// # Returns
    ///
    /// * `TextureHandle` - The loaded texture or the error texture if loading fails.
    fn load_texture(
        ctx: &Context,
        name: &str,
        img_bytes: &[u8],
        error_texture: Option<&TextureHandle>,
        texture_options: Option<egui::TextureOptions>,
    ) -> TextureHandle {
        let image = match egui_extras::image::load_svg_bytes(img_bytes) {
            Ok(img) => img,
            Err(e) => {
                log::error!("Failed to load image: {}", e);
                if let Some(error_texture) = error_texture {
                    return error_texture.clone();
                } else {
                    log::error!("Error Texture not found. Loading RED SQUARE as error texture");
                    ColorImage::new([50, 50], egui::Color32::RED)
                }
            }
        };

        ctx.load_texture(name, image, texture_options.unwrap_or_default())
    }
}

impl eframe::App for RustreamApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
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
                            egui::Button::image_and_text(&self.home_icon_texture, "Home"),
                        )
                        .clicked()
                    {
                        self.reset_ui();
                    }

                    // Quit button
                    if ui
                        .add_sized(
                            [80., 30.],
                            egui::Button::image_and_text(&self.quit_icon_texture, "Quit"),
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

            match self.mode {
                PageView::HomePage => self.render_home_page(ui),

                PageView::Sender => self.render_sender_page(ui),

                PageView::Receiver => self.render_receiver_mode(ui),
            }
        });
    }
}
