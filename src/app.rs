
use crate::capture_screen::{get_monitors, take_screenshot};
use egui::{
    Button, CentralPanel, Color32, ColorImage, ComboBox, Context, FontId, RichText, TextureHandle,
    TextureOptions,
};
use egui_extras::image::load_svg_bytes;

#[derive(Default)]
pub struct AppInterface {
    selected_monitor: usize,          // Index of the selected monitor
    monitors: Vec<String>,            // List of monitors as strings for display in the menu
    mode: Mode,                       // Enum to track modes
    home_icon: Option<TextureHandle>, // Texture for the home icon
}

#[derive(Default, Debug, PartialEq)]
pub enum Mode {
    #[default]
    HomePage,
    Sender,
    Receiver,
}

fn bytes_into_texture(
    cc: &eframe::CreationContext<'_>,
    image_bytes: &[u8],
    path: &str,
) -> TextureHandle {
    // let image_bytes: &[u8] = include_bytes!(path);
    let image: ColorImage = load_svg_bytes(image_bytes).unwrap();
    let texture: TextureHandle = cc.egui_ctx.load_texture(
        path,
        ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            &image
                .pixels
                .iter()
                .flat_map(|&c| c.to_array())
                .collect::<Vec<_>>(),
        ),
        TextureOptions::default(),
    );
    texture
}

impl AppInterface {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Retrieve the list of monitors at initialization
        let mut monitors_list = Vec::new();
        if let Ok(displays) = get_monitors() {
            for (i, _monitor) in displays.iter().enumerate() {
                monitors_list.push(format!("Monitor {}", i));
            }
        }


        let ctx: &Context = &cc.egui_ctx;
        egui_extras::install_image_loaders(ctx);

        let home_icon_texture = bytes_into_texture(
            cc,
            include_bytes!("../assets/icons/home.svg"),
            "../assets/icons/home.svg",
        );

        AppInterface {
            selected_monitor: 0,
            monitors: monitors_list,
            mode: Mode::default(),
            home_icon: Some(home_icon_texture),
        }
    }

    pub fn reset_ui(&mut self) {
        // Reset the application
        self.selected_monitor = 0;
        self.mode = Mode::default();
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    pub fn is_sender_mode(&self) -> bool {
        self.mode == Mode::Sender
    }

    pub fn is_receive_mode(&self) -> bool {
        self.mode == Mode::Receiver
    }

    pub fn is_home_page(&self) -> bool {
        self.mode == Mode::HomePage
    }
}

impl eframe::App for AppInterface {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(home_icon) = &self.home_icon {
                    if ui.add(Button::image(home_icon)).clicked() {
                        self.reset_ui();
                    }
                }
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("RUSTREAM")
                            .font(FontId::proportional(40.0))
                            .color(Color32::GOLD),
                    );
                });
            });
            ui.add_space(20.0);

            if self.is_home_page() {
                if ui.button("SENDER").clicked() {
                    self.set_mode(Mode::Sender);
                }

                if ui.button("RECEIVER").clicked() {
                    self.set_mode(Mode::Receiver);
                }
            } else if self.is_sender_mode() {
                ui.heading("Select Monitor");

                ComboBox::from_label("Monitor")
                    .selected_text(format!("Monitor {}", self.selected_monitor))
                    .show_ui(ui, |ui| {
                        for (index, monitor) in self.monitors.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_monitor, index, monitor);
                        }
                    });

                if ui.button("Start Capture").clicked() {
                    take_screenshot(self.selected_monitor);
                }
            } else if self.is_receive_mode() {
                ui.heading("Receiver Mode");
            }
        });
    }
}
