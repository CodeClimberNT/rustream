use serde::{Deserialize, Serialize};

pub type RgbaBuffer = Vec<u8>;
pub type BgraBuffer = Vec<u8>;

#[derive(Debug, Clone, Copy, Default, PartialEq,Deserialize, Serialize)]
pub struct CaptureArea {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl CaptureArea {
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

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

    pub fn full(buffer_width: usize, buffer_height: usize) -> Self {
        Self {
            x: 0,
            y: 0,
            width: buffer_width,
            height: buffer_height,
        }
    }
}
