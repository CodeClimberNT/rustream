use eframe::egui::{self, CentralPanel, Color32, Pos2, Rect, Stroke, TopBottomPanel};
use egui::{RichText, StrokeKind};

#[derive(Debug, Clone, PartialEq)]
enum Shape {
    Rectangle {
        rect: Rect,
        color: Color32,
    },
    Circle {
        center: Pos2,
        radius: f32,
        color: Color32,
    },
    Arrow {
        start: Pos2,
        end: Pos2,
        color: Color32,
    },
    FreeHand {
        points: Vec<Pos2>,
        color: Color32,
    },
    Text {
        position: Pos2,
        content: String,
        color: Color32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Tool {
    #[default]
    Rectangle,
    Circle,
    Arrow,
    FreeHand,
    Text,
    Eraser,
}

pub struct AnnotationApp {
    current_tool: Tool,
    is_drawing: bool,
    drag_start: Option<Pos2>,
    current_shape: Option<Shape>,
    annotations: Vec<Shape>,
    free_hand_points: Vec<Pos2>,
    current_color: Color32,
    show_tutorial: bool,
    show_text_input: bool,
    temp_text: String,
    text_position: Option<Pos2>,
}

impl Default for AnnotationApp {
    fn default() -> Self {
        Self {
            current_tool: Tool::default(),
            is_drawing: false,
            drag_start: None,
            current_shape: None,
            annotations: Vec::new(),
            free_hand_points: Vec::new(),
            current_color: Color32::from_rgba_unmultiplied(255, 0, 0, 255),
            show_tutorial: false,
            show_text_input: false,
            temp_text: String::new(),
            text_position: None,
        }
    }
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
                    .selectable_label(self.current_tool == Tool::Arrow, "-> Arrow")
                    .clicked()
                {
                    self.current_tool = Tool::Arrow;
                }
                if ui
                    .selectable_label(self.current_tool == Tool::FreeHand, "🖊 Free Draw")
                    .clicked()
                {
                    self.current_tool = Tool::FreeHand;
                }
                if ui
                    .selectable_label(self.current_tool == Tool::Text, "T Text")
                    .clicked()
                {
                    self.current_tool = Tool::Text;
                }
                if ui
                    .selectable_label(self.current_tool == Tool::Eraser, "Eraser")//TODO: add icon
                    .clicked()
                {
                    self.current_tool = Tool::Eraser;
                }

                ui.separator();

                // Color picker
                let color_array = self.current_color.to_array();
                let mut color_f32 = [
                    color_array[0] as f32 / 255.0,
                    color_array[1] as f32 / 255.0,
                    color_array[2] as f32 / 255.0,
                    color_array[3] as f32 / 255.0,
                ];

                ui.color_edit_button_rgba_unmultiplied(&mut color_f32);

                self.current_color = Color32::from_rgba_unmultiplied(
                    (color_f32[0] * 255.0) as u8,
                    (color_f32[1] * 255.0) as u8,
                    (color_f32[2] * 255.0) as u8,
                    (color_f32[3] * 255.0) as u8,
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Exit button on the right
                    if ui.button(RichText::new("❌").size(20.0)).clicked() {
                        std::process::exit(0);
                    }
                    // Tutorial button
                    if ui.button(RichText::new("❓").size(20.0)).clicked() {
                        self.show_tutorial = !self.show_tutorial;
                    }
                    ui.separator();
                    if ui.button(RichText::new("🗑 Clear")).clicked() {
                        self.annotations.clear();
                    }
                });
            });
        });

        // Show tutorial window
        if self.show_tutorial {
            egui::Window::new("Tutorial")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("How to use the annotation tool:");
                    ui.label("1. Select a drawing tool from the top bar");
                    ui.label("2. Choose a color using the color picker");
                    ui.label("3. Click and drag to draw");
                    ui.label("4. Release to finish the shape");
                    ui.label("\nKeyboard shortcuts:");
                    ui.label("ESC - Exit application");
                    if ui.button("Close").clicked() {
                        self.show_tutorial = false;
                    }
                });
        }

        CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let response = ui.allocate_rect(
                    ui.available_rect_before_wrap(),
                    egui::Sense::click_and_drag(),
                );
                let pointer_pos = response
                    .interact_pointer_pos()
                    .map(|pos| egui::pos2(pos.x * scale_factor, pos.y * scale_factor));

                // Handle text tool separately
                if self.current_tool == Tool::Text && response.clicked() {
                    if let Some(pos) = pointer_pos {
                        self.show_text_input = true;
                        self.text_position = Some(pos);
                        self.temp_text.clear();
                    }
                }
                // Handle other tools
                else if response.drag_started() {
                    if let Some(pos) = pointer_pos {
                        self.drag_start = Some(pos);
                        self.is_drawing = true;
                        if self.current_tool == Tool::FreeHand {
                            self.free_hand_points.clear();
                            self.free_hand_points.push(pos);
                        }
                    }
                }

                // Handle drawing
                if self.is_drawing {
                    if let Some(current_pos) = pointer_pos {
                        match self.current_tool {
                            Tool::Rectangle => {
                                if let Some(start) = self.drag_start {
                                    self.current_shape = Some(Shape::Rectangle {
                                        rect: Rect::from_two_pos(start, current_pos),
                                        color: self.current_color,
                                    });
                                }
                            }
                            Tool::Circle => {
                                if let Some(start) = self.drag_start {
                                    let radius = (current_pos - start).length();
                                    self.current_shape = Some(Shape::Circle {
                                        center: start,
                                        radius,
                                        color: self.current_color,
                                    });
                                }
                            }
                            Tool::Arrow => {
                                if let Some(start) = self.drag_start {
                                    self.current_shape = Some(Shape::Arrow {
                                        start,
                                        end: current_pos,
                                        color: self.current_color,
                                    });
                                }
                            }
                            Tool::FreeHand => {
                                self.free_hand_points.push(current_pos);
                                self.current_shape = Some(Shape::FreeHand {
                                    points: self.free_hand_points.clone(),
                                    color: self.current_color,
                                });
                            }
                            Tool::Text => {
                                if !self.show_text_input && response.clicked() {
                                    if let Some(pos) = pointer_pos {
                                        self.show_text_input = true;
                                        self.text_position = Some(pos);
                                        self.temp_text.clear();
                                    }
                                }
                            }
                            Tool::Eraser => {
                                // Find indices of shapes to remove
                                let indices_to_remove: Vec<usize> = self
                                    .annotations
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, shape)| {
                                        AnnotationApp::shape_contains_point(
                                            shape,
                                            current_pos,
                                            scale_factor,
                                        )
                                    })
                                    .map(|(i, _)| i)
                                    .collect();
                                // Remove shapes in reverse order to maintain correct indices
                                for &i in indices_to_remove.iter().rev() {
                                    self.annotations.remove(i);
                                }
                            }
                        }
                    }
                }

                // Text input window - Moved outside the main panel
                if self.show_text_input {
                    let mut show = true;
                    egui::Window::new("Write your text")
                        .open(&mut show)
                        .collapsible(false)
                        .resizable(false)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .show(ctx, |ui| {
                            ui.vertical_centered_justified(|ui| {
                                ui.add_space(8.0);
                                // Text input with larger size
                                ui.add_sized(
                                    [200.0, 30.0],
                                    egui::TextEdit::singleline(&mut self.temp_text)
                                        .hint_text("Type your text here..."),
                                );
                                ui.add_space(8.0);

                                ui.horizontal(|ui| {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("Insert")
                                                    .size(16.0)
                                                    .color(Color32::WHITE),
                                            )
                                            .fill(Color32::from_rgb(0, 150, 0))
                                            .min_size(egui::vec2(80.0, 30.0)),
                                        )
                                        .clicked() && !self.temp_text.is_empty() {
                                        if let Some(pos) = self.text_position {
                                            self.annotations.push(Shape::Text {
                                                position: pos,
                                                content: self.temp_text.clone(),
                                                color: self.current_color,
                                            });
                                        }
                                        self.show_text_input = false;
                                    }

                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("Cancel")
                                                    .size(16.0)
                                                    .color(Color32::WHITE),
                                            )
                                            .fill(Color32::from_rgb(150, 0, 0))
                                            .min_size(egui::vec2(80.0, 30.0)),
                                        )
                                        .clicked()
                                    {
                                        self.show_text_input = false;
                                    }
                                });
                            });
                        });

                    if !show {
                        self.show_text_input = false;
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

                // Draw all existing annotations
                for shape in &self.annotations {
                    self.draw_shape(ui, shape, scale_factor);
                }

                // Draw current shape while dragging
                if let Some(shape) = &self.current_shape {
                    self.draw_shape(ui, shape, scale_factor);
                }
            });
    }
}

impl AnnotationApp {
    fn draw_shape(&self, ui: &mut egui::Ui, shape: &Shape, scale_factor: f32) {
        match shape {
            Shape::Rectangle { rect, color } => {
                let display_rect = Rect::from_min_max(
                    egui::pos2(rect.min.x / scale_factor, rect.min.y / scale_factor),
                    egui::pos2(rect.max.x / scale_factor, rect.max.y / scale_factor),
                );
                ui.painter().rect_stroke(
                    display_rect,
                    0.0,
                    Stroke::new(3.0, *color),
                    StrokeKind::Outside,
                );
            }
            Shape::Circle {
                center,
                radius,
                color,
            } => {
                let display_center = egui::pos2(center.x / scale_factor, center.y / scale_factor);
                let display_radius = radius / scale_factor;
                ui.painter().circle_stroke(
                    display_center,
                    display_radius,
                    Stroke::new(3.0, *color),
                );
            }
            Shape::Arrow { start, end, color } => {
                let display_start = egui::pos2(start.x / scale_factor, start.y / scale_factor);
                let display_end = egui::pos2(end.x / scale_factor, end.y / scale_factor);
                ui.painter().arrow(
                    display_start,
                    display_end - display_start,
                    Stroke::new(3.0, *color),
                );
            }
            Shape::FreeHand { points, color } => {
                let display_points: Vec<Pos2> = points
                    .iter()
                    .map(|p| egui::pos2(p.x / scale_factor, p.y / scale_factor))
                    .collect();
                if display_points.len() >= 2 {
                    for points in display_points.windows(2) {
                        if let [p1, p2] = points {
                            ui.painter()
                                .line_segment([*p1, *p2], Stroke::new(3.0, *color));
                        }
                    }
                }
            }
            Shape::Text {
                position,
                content,
                color,
            } => {
                let display_pos = egui::pos2(position.x / scale_factor, position.y / scale_factor);
                ui.painter().text(
                    display_pos,
                    egui::Align2::LEFT_TOP,
                    content,
                    egui::FontId::proportional(16.0),
                    *color,
                );
            }
        }
    }

    fn shape_contains_point(shape: &Shape, point: Pos2, scale_factor: f32) -> bool {
        let scaled_point = Pos2::new(point.x / scale_factor, point.y / scale_factor);

        match shape {
            Shape::Rectangle { rect, .. } => {
                let scaled_rect = Rect::from_min_max(
                    Pos2::new(rect.min.x / scale_factor, rect.min.y / scale_factor),
                    Pos2::new(rect.max.x / scale_factor, rect.max.y / scale_factor),
                );
                // Add a small margin for easier selection
                scaled_rect.expand(3.0).contains(scaled_point)
            }
            Shape::Circle { center, radius, .. } => {
                let scaled_point = Pos2::new(point.x / scale_factor, point.y / scale_factor);
                let scaled_center = Pos2::new(center.x / scale_factor, center.y / scale_factor);
                scaled_point.distance(scaled_center) <= radius / scale_factor
            }
            Shape::Arrow { start, end, .. } => {
                let scaled_point = Pos2::new(point.x / scale_factor, point.y / scale_factor);
                let scaled_start = Pos2::new(start.x / scale_factor, start.y / scale_factor);
                let scaled_end = Pos2::new(end.x / scale_factor, end.y / scale_factor);
                let threshold = 5.0; // Distance threshold in pixels
                point_to_line_segment_distance(scaled_point, scaled_start, scaled_end) < threshold
            }
            Shape::FreeHand { points, .. } => {
                let scaled_point = Pos2::new(point.x / scale_factor, point.y / scale_factor);
                points.windows(2).any(|window| {
                    if let [p1, p2] = window {
                        let scaled_p1 = Pos2::new(p1.x / scale_factor, p1.y / scale_factor);
                        let scaled_p2 = Pos2::new(p2.x / scale_factor, p2.y / scale_factor);
                        point_to_line_segment_distance(scaled_point, scaled_p1, scaled_p2) < 5.0
                    } else {
                        false
                    }
                })
            }
            Shape::Text { position, .. } => {
                let scaled_point = Pos2::new(point.x / scale_factor, point.y / scale_factor);
                let scaled_pos = Pos2::new(position.x / scale_factor, position.y / scale_factor);
                let text_rect = Rect::from_min_size(scaled_pos, egui::vec2(100.0, 20.0));
                text_rect.contains(scaled_point)
            }
        }
    }
}

fn point_to_line_segment_distance(point: Pos2, start: Pos2, end: Pos2) -> f32 {
    let line = end - start;
    let len_sq = line.length_sq();
    if len_sq == 0.0 {
        return point.distance(start);
    }
    let t = ((point - start).dot(line) / len_sq).clamp(0.0, 1.0);
    let projection = start + line * t;
    point.distance(projection)
}
