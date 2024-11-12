use scrap::{Capturer, Display};
use std::io::ErrorKind;
use std::thread;
use std::time::Duration;

#[derive(Clone)]
pub struct ScreenArea {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub struct ScreenData {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub fn capture_screen(area: Option<ScreenArea>) -> ScreenData {
    let display = Display::primary().expect("No primary display found.");

    let mut capturer = Capturer::new(display).expect("Could not begin capture.");
    let (width, height) = (capturer.width(), capturer.height());

    loop {
        match capturer.frame() {
            Ok(frame) => {
                let buffer = frame.to_vec();
                // If a custom area is specified, extract that portion
                let data = if let Some(area) = &area {
                    extract_area(&buffer, width, area)
                } else {
                    buffer
                };
                return ScreenData {
                    data,
                    width: area.as_ref().map_or(width, |a| a.width),
                    height: area.as_ref().map_or(height, |a| a.height),
                };
            }
            Err(error) => {
                if error.kind() == ErrorKind::WouldBlock {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                } else {
                    panic!("Error: {}", error);
                }
            }
        }
    }
}

fn extract_area(buffer: &[u8], width: u32, area: &ScreenArea) -> Vec<u8> {
    let bytes_per_pixel = 4; // Assuming BGRA format
    let mut area_buffer = Vec::with_capacity((area.width * area.height * bytes_per_pixel) as usize);
    for y in area.y..(area.y + area.height) {
        let start = ((y * width + area.x) * bytes_per_pixel) as usize;
        let end = start + (area.width * bytes_per_pixel) as usize;
        area_buffer.extend_from_slice(&buffer[start..end]);
    }
    area_buffer
}
