extern crate scrap;

use scrap::{Capturer, Display};
use std::thread;
use std::time::Duration;
// use std::fs::File;
// use std::io::Write;
use image::{ImageBuffer, Rgba};

fn capture_screen() {
    let display = Display::primary().expect("Couldn't find primary display.");
    let mut capturer = Capturer::new(display).expect("Couldn't begin capture.");
    let (width, height) = (capturer.width(), capturer.height());

    loop {
        match capturer.frame() {
            Ok(frame) => {
                let buffer: Vec<u8> = frame.iter().cloned().collect();
                let image =
                    ImageBuffer::<Rgba<u8>, _>::from_raw(width as u32, height as u32, buffer)
                        .unwrap();
                image.save("screenshot.png").expect("Failed to save image");
                println!("Captured frame with dimensions {}x{}", width, height);
                break; // Exit after capturing one frame
            }
            Err(_) => {
                // La cattura non è riuscita, riproviamo
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

fn main() {
    capture_screen();
}
