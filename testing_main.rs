// This program is just a testing application
// Refer to `lib.rs` for the library source code

use image::{ImageBuffer, Rgb, RgbaImage};
use scap::frame::Frame;
use std::sync::mpsc::channel;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
mod recorder;

#[derive(Debug)]
pub enum RecorderCommand {
    Capture,
    Stop,
}

#[derive(Debug)]
enum AppState {
    WaitingForInput,
    CapturingFrame,
    Quitting,
}

fn save_frame(frame: Frame) -> Result<String, Box<dyn std::error::Error>> {
    match frame {
        Frame::BGRA(bgra_frame) => {
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            let filename = format!("capture_{}.png", timestamp);

            // Create a new buffer for RGBA data
            let mut rgba_data =
                Vec::with_capacity((bgra_frame.width * bgra_frame.height * 4) as usize);

            // Convert BGRA to RGBA properly handling stride
            for y in 0..bgra_frame.height {
                let row_start = (y * bgra_frame.width * 4) as usize;
                let row_end = row_start + (bgra_frame.width * 4) as usize;
                let row = &bgra_frame.data[row_start..row_end];

                for pixel in row.chunks_exact(4) {
                    rgba_data.push(pixel[2]); // R (from B)
                    rgba_data.push(pixel[1]); // G (same)
                    rgba_data.push(pixel[0]); // B (from R)
                    rgba_data.push(pixel[3]); // A (same)
                }
            }

            // Create image from RGBA data
            let img: RgbaImage =
                ImageBuffer::from_vec(bgra_frame.width as u32, bgra_frame.height as u32, rgba_data)
                    .ok_or("Failed to create image buffer")?;

            img.save(&filename)?;
            Ok(filename)
        }
        _ => Err(format!("Unsupported frame format {:?}", frame).into()),
    }
}

fn main() {
    // Create channels to communicate with the recorder thread
    let (request_tx, request_rx) = channel::<RecorderCommand>();
    let (frame_tx, frame_rx) = channel();

    // Start the recorder thread
    let recorder_handle = thread::spawn(move || {
        recorder::start_recorder(request_rx, frame_tx);
    });

    let mut state = AppState::WaitingForInput;
    println!("Commands available:");
    println!("  Press Enter to capture a frame");
    println!("  Type 'q' to quit");

    loop {
        match state {
            AppState::WaitingForInput => {
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).unwrap();
                
                match input.trim() {
                    "q" => state = AppState::Quitting,
                    _ => {
                        request_tx.send(RecorderCommand::Capture).unwrap();
                        state = AppState::CapturingFrame;
                    }
                }
            }
            
            AppState::CapturingFrame => {
                let frame = frame_rx.recv().unwrap();
                match save_frame(frame) {
                    Ok(filename) => println!("Frame saved as: {}", filename),
                    Err(e) => println!("Error saving frame: {}", e),
                }
                state = AppState::WaitingForInput;
            }
            
            AppState::Quitting => {
                request_tx.send(RecorderCommand::Stop).unwrap();
                break;
            }
        }
    }

    recorder_handle.join().unwrap();
    println!("Application terminated.");
}
