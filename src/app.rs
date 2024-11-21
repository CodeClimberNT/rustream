use std::thread;
use std::time::Duration;
use image::{ImageBuffer, Rgba};
use std::sync::mpsc::sync_channel;

use crate::capture_screen::{get_monitors, take_screenshot};
// Importing all the necessary libraries
// self -> import the module egui itself
use eframe::egui::{self, CentralPanel, Color32, ComboBox, Context, FontId, RichText, TextureHandle};
use log::debug;
// use scrap::Display;


#[derive(Default)]
pub struct AppInterface {
    selected_monitor: usize, // Index of the selected monitor
    monitors: Vec<String>,   // List of monitors as strings for display in the menu
    mode: PageView,          // Enum to track modes
    // home_icon_path: &'static str, // Path for the home icon
    address_text: String, // Text input for the receiver mode

    is_rendering_screenshot: bool,
    current_texture: Option<egui::TextureHandle>,
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
            is_rendering_screenshot: false,
            current_texture: None,
        }
    }

    pub fn width(&self, ctx: &Context) -> f32 {
        ctx.screen_rect().width()
    }

    // pub fn height(&self, ctx: &Context) -> f32 {
    //     ctx.screen_rect().height()
    // }

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

                // Toggle the rendering of the screenshot when the button is clicked and update the button text
                self.is_rendering_screenshot ^= ui
                    .button(if self.is_rendering_screenshot {
                        "Stop Capture"
                    } else {
                        "Start Capture"
                    })
                    .clicked();


                if self.is_rendering_screenshot {
                    
                    // take_screenshot(self.selected_monitor);
                    //let (tx, rx) = sync_channel(1);

                    let monitor_feedback = take_screenshot(self.selected_monitor); // Capture the primary monitor (index 0) , tx.clone()
                    match monitor_feedback {
                        Ok(monitor_feedback) => {
                            let image = egui::ColorImage::from_rgba_unmultiplied(
                                [
                                    monitor_feedback.width() as usize,
                                    monitor_feedback.height() as usize,
                                ],
                                &monitor_feedback,
                            );

                          
                            let texture = ui.ctx().load_texture("monitor_feedback", image, egui::TextureOptions::default());
                            ui.add(egui::Image::new(&texture).max_width(self.width(ui.ctx()) / 1.5));
                            //ui.add(egui::Image::new(texture));
                            //ui.ctx().request_repaint_after(Duration::from_millis(200));
                            //drop(monitor_feedback);
                            ui.ctx().request_repaint();
                            
                            //drop(texture);
                           
                            //ui.ctx().request_repaint();
                        },
                        Err(_) => {
                            ui.label("Failed to capture the monitor");
                        }
                    }
                    //while let Ok(monitor_feedback) = rx.try_recv() {
                        /*let image = egui::ColorImage::from_rgba_unmultiplied(
                            [
                                monitor_feedback.width() as usize,
                                monitor_feedback.height() as usize,
                            ],
                            &monitor_feedback,
                        );

                        self.texture = Some(ui.ctx().load_texture(
                            "monitor_feedback",
                            image,
                            egui::TextureOptions::default(),
                        ));*/
                        /*let texture = ui.ctx().load_texture(
                            "monitor_feedback",
                            image,
                            egui::TextureOptions::default(),
                        );*/
                        
                    //ui.add(egui::Image::new(&texture).max_width(self.width(ui.ctx()) / 1.5));
                    
                    //let image = load_image_from_bytes(monitor_feedback).expect("Failed to load image");
                    
                    //ui.add(egui::Image::from_bytes(&monitor_feedback).max_width(self.width(ui.ctx()) / 1.5));
                    //}
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

                PageView::Sender => {
                    self.render_sender_page(ui);
                    //ctx.request_repaint_after(Duration::from_millis(16));
                },

                PageView::Receiver => self.render_receiver_mode(ui),
            }
        });
        //ctx.tex_manager().debug_ui(ctx); //per monitorare l'uso della memoria, ma non trova il metodo debug_ui
    }
}
