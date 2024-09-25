// extern crate scrap;

use scrap::{Capturer, Display};
use std::thread;
use std::time::Duration;
use image::{ImageBuffer, Rgba};

fn get_monitors() -> Vec<Display> {
    let monitors: Vec<Display> = Display::all().expect("Couldn't find any display.");
    monitors
}

fn set_monitor(mut monitors: Vec<Display>, index: usize) -> Display {
    monitors.remove(index)
}

fn capture_screen() {

    // configurazione recupera monitor del sistema
    let monitors = get_monitors();
    for i in 0..monitors.len() {
        println!("Monitor {}", i);
    }
    
    let monitor = set_monitor(monitors, 0);

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
                let image =
                    ImageBuffer::<Rgba<u8>, _>::from_raw(width as u32, height as u32, buffer)
                        .unwrap();
                image.save("screenshot.png").expect("Failed to save image");
                println!("Captured frame with dimensions {}x{}", width, height);
                break; // Exit after capturing one frame
            }
            Err(_) => {
                // Capture failed, retry
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

fn main() {
    capture_screen();
}
