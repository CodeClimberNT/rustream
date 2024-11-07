use image::{ImageBuffer, Rgba};
use log::debug;
use scrap::{Capturer, Display};
use std::thread;
use std::time::Duration;

pub fn get_monitors() -> Result<Vec<Display>, ()> {
    let monitors: Vec<Display> = Display::all().expect("Couldn't find any display.");
    if monitors.is_empty() {
        return Err(());
    }

    Ok(monitors)
}

pub fn get_primary_monitor() -> Result<Display, ()> {
    let monitor: Display = Display::primary().expect("Couldn't find any display.");
    Ok(monitor)
}

pub fn set_monitor(index: usize) -> Result<Display, ()> {
    if index == 0 {
        return get_primary_monitor();
    }

    let mut monitors = get_monitors().unwrap();

    if index >= monitors.len() {
        return Err(());
    }

    let monitor: Display = monitors.remove(index);
    Ok(monitor)
}

#[allow(dead_code)]
pub fn save_screenshot(image: ImageBuffer<Rgba<u8>, Vec<u8>>, path: &str) {
    image.save(path).expect("Failed to save image");
}

pub fn take_screenshot(index: usize) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let monitors: Vec<Display> = get_monitors().unwrap();
    for i in 0..monitors.len() {
        debug!("Monitor {}", i);
    }

    let monitor: Display = set_monitor(index).unwrap();

    let mut capturer: Capturer = Capturer::new(monitor).expect("Couldn't begin capture.");
    let (width, height) = (capturer.width(), capturer.height());

    loop {
        match capturer.frame() {
            Ok(frame) => {
                let mut buffer: Vec<u8> = frame.iter().cloned().collect();
                // Convert BGRA to RGBA
                for chunk in buffer.chunks_exact_mut(4) {
                    chunk.swap(0, 2); // Swap B and R
                }
                let image: ImageBuffer<Rgba<u8>, Vec<u8>> =
                    ImageBuffer::<Rgba<u8>, _>::from_raw(width as u32, height as u32, buffer)
                        .unwrap();
                debug!("Captured frame with dimensions {}x{}", width, height);
                // save_screenshot(image.clone(), "screenshot.png");
                return image;
            }
            Err(_) => {
                eprintln!("Screenshot failed, retrying...");
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

#[allow(dead_code)]
pub fn take_screenshot_from_monitor(monitor: Display) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut capturer: Capturer = Capturer::new(monitor).expect("Couldn't begin capture.");
    let (width, height) = (capturer.width(), capturer.height());

    loop {
        match capturer.frame() {
            Ok(frame) => {
                let mut buffer: Vec<u8> = frame.iter().cloned().collect();
                // Convert BGRA to RGBA
                for chunk in buffer.chunks_exact_mut(4) {
                    chunk.swap(0, 2); // Swap B and R
                }
                let image: ImageBuffer<Rgba<u8>, Vec<u8>> =
                    ImageBuffer::<Rgba<u8>, _>::from_raw(width as u32, height as u32, buffer)
                        .unwrap();
                debug!("Captured frame with dimensions {}x{}", width, height);
                // save_screenshot(image.clone(), "screenshot.png");
                return image;
            }
            Err(_) => {
                // Capture failed, retry
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}
