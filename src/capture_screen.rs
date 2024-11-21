use image::{ImageBuffer, Rgba};
use log::debug;
use scrap::{Capturer, Display};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::mpsc::SyncSender;
use std::sync::Mutex;

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

pub fn take_screenshot(index: usize) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, ()> { //Result<ImageBuffer<Rgba<u8>, Vec<u8>>, ()>

    let monitors: Vec<Display> = get_monitors().unwrap();
    for i in 0..monitors.len() {
        debug!("Monitor {}", i);
    }

    let monitor: Display = set_monitor(index).unwrap();
    drop(monitors);
    //let display = Mutex::new(monitor);
    //let d = display.lock().unwrap();

    //let mut monitors: Vec<Display> = get_monitors().unwrap();
    //let monitor = monitors.remove(index);
    thread::sleep(Duration::from_millis(16)); //take screeenshots at 60 FPS
    let mut capturer: Capturer = match Capturer::new(monitor){
        Ok(capturer) => capturer,
        Err(_) => {
            println!("Failed to create capturer");
            return Err(());
        }
    };
    
    let (width, height) = (capturer.width(), capturer.height());

    let expected_size = width * height * 4;
    
    loop {

        let frame = match capturer.frame() {
            Ok(frame) => frame,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                // No frame available
                println!("No frame available, retrying...");
                thread::sleep(Duration::from_millis(16));
                continue;
            },
            Err(_) => {
                println!("Screenshot failed, retrying...");
                thread::sleep(Duration::from_millis(100));
                continue;
            }
        };
        let mut buffer = frame.to_vec();

        if buffer.len() != expected_size {
            println!("Wrong buffer dimension: {}, expected: {}", buffer.len(), expected_size);
            continue;
        }

        // Convert BGRA to RGBA
        for chunk in buffer.chunks_exact_mut(4) {
            chunk.swap(0, 2); // Swap B and R
        }
        
        let image: ImageBuffer<Rgba<u8>, Vec<u8>> =
            match ImageBuffer::<Rgba<u8>, _>::from_raw(width as u32, height as u32, buffer.clone()){
                Some(image) => image,
                None => {
                    println!("Failed to create image buffer");
                    continue;
                }
            };
        println!("Captured frame with dimensions {}x{}", width, height);
        // save_screenshot(image.clone(), "screenshot.png");
        //thread::sleep(Duration::from_millis(16));
        drop(buffer);
        return Ok(image);
        //thread::sleep(Duration::from_millis(16)); //take screeenshots at 60 FPS
    } 

   
}

pub fn take_screenshot_thread(index: usize, tx: SyncSender<ImageBuffer<Rgba<u8>, Vec<u8>>>) -> () {
    
    //let mut image: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::<Rgba<u8>, _>::from_raw(capturer.width as u32, capturer.height as u32, Vec::new()).unwrap();

    thread::spawn(move || {

        let monitors: Vec<Display> = get_monitors().unwrap();
        for i in 0..monitors.len() {
            debug!("Monitor {}", i);
        }

        let monitor: Display = set_monitor(index).unwrap();
        //let display = Mutex::new(monitor);
        //let d = display.lock().unwrap();

        //let mut monitors: Vec<Display> = get_monitors().unwrap();
        //let monitor = monitors.remove(index);
        
        let mut capturer: Capturer = match Capturer::new(monitor){
            Ok(capturer) => capturer,
            Err(_) => {
                println!("Failed to create capturer");
                return;
            }
        };

        let (width, height) = (capturer.width(), capturer.height());
    
        
        let expected_size = width * height * 4;
        //let mut buffer: Vec<u8> = Vec::with_capacity(expected_size);
        
        loop {
    
            match capturer.frame() {
                Ok(frame) => {
                    //let mut buffer: Vec<u8> = frame.iter().cloned().collect();
                    //buffer.clear();
                    //buffer.extend_from_slice(&frame);
                    let mut buffer=frame.to_vec();

                    if buffer.len() != expected_size {
                        println!("Dimensione del buffer inattesa: {}, prevista: {}", buffer.len(), expected_size);
                        continue;
                    }

                    // Convert BGRA to RGBA
                    for chunk in buffer.chunks_exact_mut(4) {
                        chunk.swap(0, 2); // Swap B and R
                    }
                    
                    let image: ImageBuffer<Rgba<u8>, Vec<u8>> =
                        match ImageBuffer::<Rgba<u8>, _>::from_raw(width as u32, height as u32, buffer.clone()){
                            Some(image) => image,
                            None => {
                                eprintln!("Failed to create image buffer");
                                continue;
                            }
                        };
                    debug!("Captured frame with dimensions {}x{}", width, height);
                    // save_screenshot(image.clone(), "screenshot.png");
                    //return image;
                    if tx.send(image).is_err() {
                        println!("Receiver dropped, stopping capture...");
                        break;
                    }
                },
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    // Nessun frame disponibile, continua il ciclo
                    thread::sleep(Duration::from_millis(16));
                    continue;
                },
                Err(_) => {
                    println!("Screenshot failed, retrying...");
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }
            }
            thread::sleep(Duration::from_millis(16)); //take screeenshots at 60 FPS
        } 
    });
   
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
