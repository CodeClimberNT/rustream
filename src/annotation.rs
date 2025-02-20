use eframe::egui::{self, CentralPanel, Color32, Pos2, Rect, Stroke, TopBottomPanel};
use egui::StrokeKind;

#[derive(Debug, Clone, PartialEq)]
enum Shape {
    Rectangle(Rect),
    Circle { center: Pos2, radius: f32 },
    Arrow { start: Pos2, end: Pos2 },
    FreeHand(Vec<Pos2>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tool {
    Rectangle,
    Circle,
    Arrow,
    FreeHand,
}

impl Default for Tool {
    fn default() -> Self {
        Tool::Rectangle
    }
}

#[derive(Default)]
pub struct AnnotationApp {
    current_tool: Tool,
    is_drawing: bool,
    drag_start: Option<Pos2>,
    current_shape: Option<Shape>,
    annotations: Vec<Shape>,
    free_hand_points: Vec<Pos2>,
}

impl eframe::App for AnnotationApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) {
            std::process::exit(0);
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            std::process::exit(0);
        }

        let scale_factor = ctx.pixels_per_point();

        // Add tool selection panel
        TopBottomPanel::top("tools").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(self.current_tool == Tool::Rectangle, "⬜ Rectangle")
                    .clicked()
                {
                    self.current_tool = Tool::Rectangle;
                }
                if ui
                    .selectable_label(self.current_tool == Tool::Circle, "⭕ Circle")
                    .clicked()
                {
                    self.current_tool = Tool::Circle;
                }
                if ui
                    .selectable_label(self.current_tool == Tool::Arrow, "➡ Arrow")
                    .clicked()
                {
                    self.current_tool = Tool::Arrow;
                }
                if ui
                    .selectable_label(self.current_tool == Tool::FreeHand, "✎ Free Draw")
                    .clicked()
                {
                    self.current_tool = Tool::FreeHand;
                }
            });
        });

        CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let response =
                    ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::drag());
                let pointer_pos = response
                    .interact_pointer_pos()
                    .map(|pos| egui::pos2(pos.x * scale_factor, pos.y * scale_factor));

                // Handle drawing start
                if response.drag_started() {
                    if let Some(pos) = pointer_pos {
                        self.drag_start = Some(pos);
                        self.is_drawing = true;
                        if self.current_tool == Tool::FreeHand {
                            self.free_hand_points.clear();
                            self.free_hand_points.push(pos);
                        }
                    }
                }

                // Handle dragging
                if self.is_drawing {
                    if let Some(current_pos) = pointer_pos {
                        match self.current_tool {
                            Tool::Rectangle => {
                                if let Some(start) = self.drag_start {
                                    self.current_shape = Some(Shape::Rectangle(
                                        Rect::from_two_pos(start, current_pos),
                                    ));
                                }
                            }
                            Tool::Circle => {
                                if let Some(start) = self.drag_start {
                                    let radius = (current_pos - start).length();
                                    self.current_shape = Some(Shape::Circle {
                                        center: start,
                                        radius,
                                    });
                                }
                            }
                            Tool::Arrow => {
                                if let Some(start) = self.drag_start {
                                    self.current_shape = Some(Shape::Arrow {
                                        start,
                                        end: current_pos,
                                    });
                                }
                            }
                            Tool::FreeHand => {
                                self.free_hand_points.push(current_pos);
                                self.current_shape =
                                    Some(Shape::FreeHand(self.free_hand_points.clone()));
                            }
                        }
                    }
                }

                // Handle drawing end
                if response.drag_stopped() {
                    if let Some(shape) = self.current_shape.take() {
                        self.annotations.push(shape);
                    }
                    self.drag_start = None;
                    self.is_drawing = false;
                }

                // Draw all annotations
                for shape in &self.annotations {
                    self.draw_shape(ui, shape, scale_factor);
                }

                // Draw current shape
                if let Some(shape) = &self.current_shape {
                    self.draw_shape(ui, shape, scale_factor);
                }
            });
    }
}

impl AnnotationApp {
    fn draw_shape(&self, ui: &mut egui::Ui, shape: &Shape, scale_factor: f32) {
        let stroke = Stroke::new(3.0, Color32::from_rgba_unmultiplied(255, 0, 0, 128));

        match shape {
            Shape::Rectangle(rect) => {
                let display_rect = Rect::from_min_max(
                    egui::pos2(rect.min.x / scale_factor, rect.min.y / scale_factor),
                    egui::pos2(rect.max.x / scale_factor, rect.max.y / scale_factor),
                );
                ui.painter()
                    .rect_stroke(display_rect, 0.0, stroke, StrokeKind::Outside);
            }
            Shape::Circle { center, radius } => {
                let display_center = egui::pos2(center.x / scale_factor, center.y / scale_factor);
                let display_radius = radius / scale_factor;
                ui.painter()
                    .circle_stroke(display_center, display_radius, stroke);
            }
            Shape::Arrow { start, end } => {
                let display_start = egui::pos2(start.x / scale_factor, start.y / scale_factor);
                let display_end = egui::pos2(end.x / scale_factor, end.y / scale_factor);
                ui.painter()
                    .arrow(display_start, display_end - display_start, stroke);
            }
            Shape::FreeHand(points) => {
                let display_points: Vec<Pos2> = points
                    .iter()
                    .map(|p| egui::pos2(p.x / scale_factor, p.y / scale_factor))
                    .collect();

                // Draw line segments between consecutive points
                if display_points.len() >= 2 {
                    for points in display_points.windows(2) {
                        if let [p1, p2] = points {
                            ui.painter().line_segment([*p1, *p2], stroke);
                        }
                    }
                }
            }
        }
    }
}
