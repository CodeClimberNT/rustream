use crate::screen_capture::ScreenCapture;

// Importing all the necessary libraries
// self -> import the module egui itself
use eframe::egui::{self, CentralPanel, Color32, ColorImage, ComboBox, Context, FontId, RichText};
use log::debug;

#[derive(Default)]
pub struct AppInterface {
    // selected_monitor: usize,                  // Index of the selected monitor
    screen_capturer: ScreenCapture, // List of monitors as strings for display in the menu
    screen_texture: Option<egui::TextureHandle>, // Texture for the screen capture
    placeholder_texture: Option<egui::TextureHandle>, // Texture for the screen capture
    mode: PageView,                 // Enum to track modes
    // home_icon_path: &'static str, // Path for the home icon
    address_text: String, // Text input for the receiver mode
    is_rendering_screen: bool,
    // home_icon: egui::TextureHandle,
}

#[derive(Default, Debug, PartialEq)]
pub enum PageView {
    #[default]
    HomePage,
    Sender,
    Receiver,
}

impl AppInterface {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // let svg_home_icon_path: &'static str = "../assets/icons/home.svg";
        // Create a simple placeholder image (e.g., a solid color)
        let placeholder_image = egui::ColorImage::new([1, 1], egui::Color32::GOLD);

        // Load the placeholder texture
        let placeholder_texture = cc.egui_ctx.load_texture(
            "placeholder",
            placeholder_image,
            egui::TextureOptions::default(),
        );
        let screen_capture = ScreenCapture::default();
        let ctx: &Context = &cc.egui_ctx;
        egui_extras::install_image_loaders(ctx);

        AppInterface {
            screen_capturer: screen_capture,
            screen_texture: None,
            placeholder_texture: Some(placeholder_texture),
            mode: PageView::default(),
            // home_icon_path: svg_home_icon_path,
            address_text: String::new(),
            is_rendering_screen: false,
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

                let texture = self
                    .screen_texture
                    .as_ref()
                    .unwrap_or(self.placeholder_texture.as_ref().unwrap());
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
}

impl eframe::App for AppInterface {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    // Home button

                    if ui
                        .add_sized(
                            [30., 30.],
                            egui::ImageButton::new(egui::include_image!(
                                // TODO: use the home_icon_path variable instead of the hardcoded path
                                "../assets/icons/home.svg"
                            )),
                        )
                        .on_hover_text("Home")
                        .clicked()
                    {
                        self.reset_ui();
                    }
                });

                // ui.image(egui::include_image!("../assets/icons/home.svg"));
                // ;
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
