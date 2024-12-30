use eframe::egui::{self, CentralPanel, Color32, Pos2, Rect};
use serde_json::json;
use std::io::Write;
use crate::common::CaptureArea;
use scrap::Display;

#[derive(Default)]
pub struct SecondaryApp {
    is_selecting: bool,
    capture_area: Option<CaptureArea>,
    drag_start: Option<Pos2>,
    new_capture_area: Option<Rect>,
    show_popup: bool,
    selected_monitor: usize,
}

impl SecondaryApp {
    pub fn new(selected_monitor: usize) -> Self {
        Self {
            is_selecting: false,
            capture_area: None,
            drag_start: None,
            new_capture_area: None,
            show_popup: false,
            selected_monitor,
        }
    }
}

impl eframe::App for SecondaryApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) {
            println!("{{\"status\": \"cancelled\"}}\n");
            std::io::stdout().flush().unwrap();
            std::process::exit(0);
        }

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
                     Press ESC at any time to exit"
                );
            });

        // Get display info for the selected monitor
        if let Ok(displays) = Display::all() {
            if let Some(display) = displays.get(self.selected_monitor) {
                CentralPanel::default()
                    .frame(egui::Frame::none().fill(Color32::TRANSPARENT))
                    .show(ctx, |ui| {
                        let response = ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::drag());

                        if response.drag_started() {
                            if let Some(pos) = response.interact_pointer_pos() {
                                // Adjust coordinates for selected monitor
                                let screen_pos = egui::pos2(
                                    pos.x * scale_factor + display.width() as f32 * self.selected_monitor as f32,
                                    pos.y * scale_factor
                                );
                                self.drag_start = Some(screen_pos);
                            }
                            self.show_popup = false;
                        }

                        if let Some(start) = self.drag_start {
                            if let Some(current) = response.interact_pointer_pos() {
                                let screen_pos = egui::pos2(
                                    current.x * scale_factor + display.width() as f32 * self.selected_monitor as f32,
                                    current.y * scale_factor
                                );
                                self.new_capture_area = Some(egui::Rect::from_two_pos(start, screen_pos));
                            }
                        }

                        if let Some(rect) = self.new_capture_area {
                            let display_rect = egui::Rect::from_min_max(
                                egui::pos2(rect.min.x/scale_factor, rect.min.y/scale_factor),
                                egui::pos2(rect.max.x/scale_factor, rect.max.y/scale_factor)
                            );

                            ui.painter().rect_stroke(
                                display_rect,
                                0.0,
                                egui::Stroke::new(3.0, egui::Color32::from_rgba_unmultiplied(0, 255, 0, 128)),
                            );

                            if response.drag_released() {
                                self.show_popup = true;
                            }
                        }

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
                                                if ui.add_sized([button_width, 40.0], egui::Button::new("OK")).clicked() {
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

                                                    println!("{}\n", serde_json::to_string(&output_with_status).unwrap());
                                                    std::io::stdout().flush().unwrap();
                                                    std::process::exit(0);
                                                }

                                                ui.add_space(spacing);
                                                
                                                if ui.add_sized([button_width, 40.0], egui::Button::new("Cancel")).clicked() {
                                                    println!("{{\"status\": \"cancelled\"}}\n");
                                                    std::io::stdout().flush().unwrap();
                                                    std::process::exit(0);
                                                }
                                            }
                                        );
                                    });
                                });
                        }
                    });
            }
        }
    }
}