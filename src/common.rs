// use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CaptureArea {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl CaptureArea {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn from_tuple(t: (u32, u32, u32, u32)) -> Self {
        Self::new(t.0, t.1, t.2, t.3)
    }

    pub fn to_tuple(&self) -> (u32, u32, u32, u32) {
        (self.x, self.y, self.width, self.height)
    }
}
