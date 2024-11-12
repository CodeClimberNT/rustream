// https://github.com/emilk/egui/blob/master/examples/images/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use crate::capture::ScreenData;
use eframe::egui::{self, Color32, Stroke, Vec2};
use image::{ImageBuffer, Rgba};

pub struct AnnotationState {
    pub active: bool,
    pub annotations: Vec<Annotation>,
    pub current_tool: AnnotationTool,
    pub drawing: bool,
    pub current_points: Vec<Vec2>,
}

impl Default for AnnotationState {
    fn default() -> Self {
        Self {
            active: false,
            annotations: Vec::new(),
            current_tool: AnnotationTool::Pen,
            drawing: false,
            current_points: Vec::new(),
        }
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
            match &annotation.shape {
                AnnotationShape::Line(points) => {
                    for window in points.windows(2) {
                        if let [start, end] = window {
                            draw_line(&mut img, start, end, annotation.color);
                        }
                    }
                }
                // Handle other shapes as needed
                _ => {}
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
            ui.label("Use the mouse to draw annotations.");
            let response = ui.allocate_response(ui.available_size(), egui::Sense::drag());
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
                    state.annotations.push(Annotation {
                        shape: AnnotationShape::Line(state.current_points.clone()),
                        color: Color32::RED,
                        thickness: 2.0,
                    });
                    state.current_points.clear();
                }
            }
            // Draw existing annotations
            let painter = ui.painter_at(response.rect);
            for annotation in &state.annotations {
                match &annotation.shape {
                    AnnotationShape::Line(points) => {
                        painter.add(egui::Shape::line(
                            points
                                .clone()
                                .into_iter()
                                .map(|v| egui::Pos2::from((v.x, v.y)))
                                .collect(),
                            Stroke::new(annotation.thickness, annotation.color),
                        ));
                    }
                    // ...handle other shapes...
                    _ => {}
                }
            }
        });
    }
}
