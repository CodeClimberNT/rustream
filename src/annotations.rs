use crate::capture::ScreenData;
use eframe::egui::{self, Color32, Stroke, Vec2};
use image::{ImageBuffer, Rgba};

pub struct AnnotationState {
    pub active: bool,
    pub annotations: Vec<Annotation>,
    pub current_tool: AnnotationTool,
    pub drawing: bool,
    pub current_points: Vec<Vec2>,
    pub text_input: String, // Add this field
}

impl Default for AnnotationState {
    fn default() -> Self {
        Self {
            active: false,
            annotations: Vec::new(),
            current_tool: AnnotationTool::Pen,
            drawing: false,
            current_points: Vec::new(),
            text_input: String::new(),
        }
    }
}

impl AnnotationState {
    pub fn add_annotation(&mut self, shape: AnnotationShape, color: Color32) {
        self.annotations.push(Annotation {
            shape,
            color,
            thickness: 2.0,
        });
    }
}

pub enum AnnotationTool {
    Pen,
    Arrow,
    Text,
}

pub struct Annotation {
    pub shape: AnnotationShape,
    pub color: Color32,
    pub thickness: f32,
}

pub enum AnnotationShape {
    Line(Vec<Vec2>),
    Arrow(Vec2, Vec2),
    Text(String, Vec2),
}

pub fn toggle_annotations(state: &mut AnnotationState, active: bool) {
    state.active = active;
}

pub fn apply_annotations(screen_data: &mut ScreenData, state: &AnnotationState) {
    if !state.annotations.is_empty() {
        // Convert the screen data into an image buffer
        let width = screen_data.width;
        let height = screen_data.height;
        let mut img =
            match ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, screen_data.data.clone()) {
                Some(buffer) => buffer,
                None => {
                    eprintln!("Failed to create image buffer from screen data.");
                    return;
                }
            };

        // Draw annotations onto the image buffer
        for annotation in &state.annotations {
            if let AnnotationShape::Line(points) = &annotation.shape {
                for window in points.windows(2) {
                    if let [start, end] = window {
                        draw_line(&mut img, start, end, annotation.color);
                    }
                }
            }
        }

        // Update the screen data with the annotated image
        screen_data.data = img.into_raw();
    }
}

fn draw_line(image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, start: &Vec2, end: &Vec2, color: Color32) {
    let color = Rgba([color.r(), color.g(), color.b(), color.a()]);
    let (x0, y0) = (start.x as i32, start.y as i32);
    let (x1, y1) = (end.x as i32, end.y as i32);

    // Bresenham's line algorithm
    // https://en.wikipedia.org/wiki/Bresenham%27s_line_algorithm
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let (mut x, mut y) = (x0, y0);

    loop {
        if x >= 0 && y >= 0 && x < image.width() as i32 && y < image.height() as i32 {
            image.put_pixel(x as u32, y as u32, color);
        }
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

pub fn draw_annotations(ui: &mut egui::Ui, state: &mut AnnotationState) {
    if state.active {
        egui::Window::new("Annotations").show(ui.ctx(), |ui| {
            ui.horizontal(|ui| {
                if ui.button("Pen").clicked() {
                    state.current_tool = AnnotationTool::Pen;
                }
                if ui.button("Arrow").clicked() {
                    state.current_tool = AnnotationTool::Arrow;
                }
                if ui.button("Text").clicked() {
                    state.current_tool = AnnotationTool::Text;
                }
            });

            let response = ui.allocate_response(ui.available_size(), egui::Sense::drag());
            let response_rect = response.rect;

            match state.current_tool {
                AnnotationTool::Pen => handle_pen_tool(response, state),
                AnnotationTool::Arrow => handle_arrow_tool(response, state),
                AnnotationTool::Text => handle_text_tool(response, ui, state),
            }

            // Draw existing annotations
            let painter = ui.painter_at(response_rect);
            for annotation in &state.annotations {
                match &annotation.shape {
                    AnnotationShape::Line(points) => {
                        painter.add(egui::Shape::line(
                            points
                                .iter()
                                .map(|v| egui::Pos2::from((v.x, v.y)))
                                .collect(),
                            Stroke::new(annotation.thickness, annotation.color),
                        ));
                    }
                    AnnotationShape::Arrow(start, end) => {
                        draw_arrow(
                            &painter,
                            *start,
                            *end,
                            annotation.color,
                            annotation.thickness,
                        );
                    }
                    AnnotationShape::Text(text, pos) => {
                        painter.text(
                            egui::Pos2::from((pos.x, pos.y)),
                            egui::Align2::LEFT_TOP,
                            text,
                            egui::FontId::proportional(14.0),
                            annotation.color,
                        );
                    }
                }
            }
        });
    }
}

fn handle_pen_tool(response: egui::Response, state: &mut AnnotationState) {
    if response.drag_started() {
        state.drawing = true;
        state.current_points.clear();
    }
    if state.drawing && response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            state.current_points.push(pos.to_vec2());
        }
    }
    if response.drag_stopped() {
        state.drawing = false;
        if !state.current_points.is_empty() {
            state.add_annotation(
                AnnotationShape::Line(state.current_points.clone()),
                Color32::RED,
            );
            state.current_points.clear();
        }
    }
}

fn handle_arrow_tool(response: egui::Response, state: &mut AnnotationState) {
    if response.drag_started() {
        state.drawing = true;
        if let Some(pos) = response.interact_pointer_pos() {
            state.current_points = vec![pos.to_vec2()];
        }
    }
    if state.drawing && response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            if state.current_points.len() > 1 {
                state.current_points[1] = pos.to_vec2();
            } else {
                state.current_points.push(pos.to_vec2());
            }
        }
    }
    if response.drag_stopped() {
        state.drawing = false;
        if state.current_points.len() == 2 {
            state.add_annotation(
                AnnotationShape::Arrow(state.current_points[0], state.current_points[1]),
                Color32::RED,
            );
            state.current_points.clear();
        }
    }
}

fn handle_text_tool(response: egui::Response, ui: &mut egui::Ui, state: &mut AnnotationState) {
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            egui::Window::new("Enter Text")
                .id(egui::Id::new("text_input"))
                .show(ui.ctx(), |ui| {
                    ui.heading("Enter Text");
                    let text_edit = ui.text_edit_singleline(&mut state.text_input);
                    if (ui.button("Add").clicked()
                        || text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        && !state.text_input.is_empty()
                    {
                        state.add_annotation(
                            AnnotationShape::Text(state.text_input.clone(), pos.to_vec2()),
                            Color32::RED,
                        );
                        state.text_input.clear();
                        ui.close_menu();
                    }
                });
        }
    }
}

fn draw_arrow(painter: &egui::Painter, start: Vec2, end: Vec2, color: Color32, thickness: f32) {
    painter.add(egui::Shape::line_segment(
        [
            egui::Pos2::from((start.x, start.y)),
            egui::Pos2::from((end.x, end.y)),
        ],
        Stroke::new(thickness, color),
    ));

    // Calculate arrow head
    let direction = (end - start).normalized();
    let arrow_size = 10.0;
    let angle = std::f32::consts::PI / 6.0; // 30 degrees

    let rot_left = Vec2::new(
        direction.x * f32::cos(angle) - direction.y * f32::sin(angle),
        direction.x * f32::sin(angle) + direction.y * f32::cos(angle),
    );
    let rot_right = Vec2::new(
        direction.x * f32::cos(-angle) - direction.y * f32::sin(-angle),
        direction.x * f32::sin(-angle) + direction.y * f32::cos(-angle),
    );

    let arrow_left = end - rot_left * arrow_size;
    let arrow_right = end - rot_right * arrow_size;

    painter.add(egui::Shape::line_segment(
        [
            egui::Pos2::from((end.x, end.y)),
            egui::Pos2::from((arrow_left.x, arrow_left.y)),
        ],
        Stroke::new(thickness, color),
    ));
    painter.add(egui::Shape::line_segment(
        [
            egui::Pos2::from((end.x, end.y)),
            egui::Pos2::from((arrow_right.x, arrow_right.y)),
        ],
        Stroke::new(thickness, color),
    ));
}
