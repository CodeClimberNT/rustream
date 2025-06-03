use egui::Rect;
use serde::{Deserialize, Serialize};

pub type RgbaBuffer = Vec<u8>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize, Serialize)]
pub struct CaptureArea {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl CaptureArea {

    pub fn new_with_safeguards(
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        buffer_width: usize,
        buffer_height: usize,
    ) -> Self {
        // Clamp x and y within buffer bounds
        let x = x.min(buffer_width);
        let y = y.min(buffer_height);

        // Adjust width and height to ensure they fit within the buffer
        let width = if x + width > buffer_width {
            buffer_width - x
        } else {
            width
        };

        let height = if y + height > buffer_height {
            buffer_height - y
        } else {
            height
        };

        // Default to full buffer if width or height is zero
        if width == 0 || height == 0 {
            Self::full(buffer_width, buffer_height)
        } else {
            Self {
                x,
                y,
                width,
                height,
            }
        }
    }

    pub fn from_rect_safe(rect: Rect, display_width: usize, display_height: usize) -> Self {
        // Ensure positive width and height by normalizing the rect
        let (min_x, max_x) = if rect.min.x <= rect.max.x {
            (rect.min.x, rect.max.x)
        } else {
            (rect.max.x, rect.min.x)
        };

        let (min_y, max_y) = if rect.min.y <= rect.max.y {
            (rect.min.y, rect.max.y)
        } else {
            (rect.max.y, rect.min.y)
        };

        Self::new_with_safeguards(
            min_x.round() as usize,
            min_y.round() as usize,
            (max_x - min_x).round() as usize,
            (max_y - min_y).round() as usize,
            display_width,
            display_height,
        )
    }

    pub fn full(buffer_width: usize, buffer_height: usize) -> Self {
        Self {
            x: 0,
            y: 0,
            width: buffer_width,
            height: buffer_height,
        }
    }
}
