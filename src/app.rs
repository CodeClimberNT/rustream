use crate::capture_screen::{get_monitors, take_screenshot};
use egui::{CentralPanel, Color32, ComboBox, Context, FontId, RichText};
use log::debug;

// #[derive(Default)]
pub struct AppInterface {
    selected_monitor: usize, // Index of the selected monitor
    monitors: Vec<String>,   // List of monitors as strings for display in the menu
    mode: PageView,          // Enum to track modes
    // home_icon_path: &'static str, // Path for the home icon
    address_text: String, // Text input for the receiver mode
    streaming: bool,
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
        // Retrieve the list of monitors at initialization
        let mut monitors_list = Vec::new();
        if let Ok(displays) = get_monitors() {
            for (i, _monitor) in displays.iter().enumerate() {
                monitors_list.push(format!("Monitor {}", i));
            }
        }

        let ctx: &Context = &cc.egui_ctx;
        egui_extras::install_image_loaders(ctx);

        // let svg_home_icon_path: &'static str = "../assets/icons/home.svg";

        AppInterface {
            selected_monitor: 0,
            monitors: monitors_list,
            mode: PageView::default(),
            // home_icon_path: svg_home_icon_path,
            address_text: String::new(),
            streaming: false,
        }
    }

    pub fn reset_ui(&mut self) {
        // Reset the application
        self.selected_monitor = 0;
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

        ui.horizontal_centered(|ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);

                if ui.button("Start Capture").clicked() {
                    self.streaming = true;
                }
                if ui.button("Stop Capture").clicked() {
                    self.streaming = false;
                }

                if self.streaming {
                    let monitor_feedback = take_screenshot(0); // Capture the primary monitor (index 0)
                    let image = egui::ColorImage::from_rgba_unmultiplied(
                        [
                            monitor_feedback.width() as usize,
                            monitor_feedback.height() as usize,
                        ],
                        &monitor_feedback,
                    );

                    let texture = ui.ctx().load_texture(
                        "monitor_feedback",
                        image,
                        egui::TextureOptions::default(),
                    );
                    ui.add(egui::Image::new(&texture).max_height(300.));
                    // ui.image(&texture);
                }
            });
        });
        ui.add_space(40.0);

        ui.heading("Select Monitor");

        ComboBox::from_label("Monitor")
            .selected_text(format!("Monitor {}", self.selected_monitor))
            .show_ui(ui, |ui| {
                for (index, monitor) in self.monitors.iter().enumerate() {
                    ui.selectable_value(&mut self.selected_monitor, index, monitor);
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
