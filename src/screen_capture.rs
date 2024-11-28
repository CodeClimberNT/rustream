use image::{ImageBuffer, Rgba};
use scrap::{Capturer, Display};

pub struct ScreenCapture {
    selected_monitor: usize,
    monitors: Vec<String>,
    capturer: Option<Capturer>,
    width: u32,
    height: u32,
}

impl Default for ScreenCapture {
    fn default() -> Self {
        let mut monitors_list = Vec::new();
        if let Ok(displays) = get_monitors() {
            for (i, _monitor) in displays.iter().enumerate() {
                monitors_list.push(format!("Monitor {}", i));
            }
        }

        ScreenCapture {
            monitors: monitors_list,
            capturer: None,
            selected_monitor: 0,
            width: 0,
            height: 0,
        }
    }
}

impl ScreenCapture {
    pub fn get_monitor_index(&self) -> usize {
        self.selected_monitor
    }

    pub fn set_monitor_index(&mut self, index: usize) {
        self.selected_monitor = index;
        self.capturer = None;
        self.width = 0;
        self.height = 0;
        log::info!("Selected monitor: {}", self.selected_monitor);
    }
    pub fn reset_capture(&mut self) {
        self.capturer = None;
        self.width = 0;
        self.height = 0;
    }

    pub fn get_monitors(&self) -> &Vec<String> {
        &self.monitors
    }

    pub fn capture_screen(&mut self) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        if self.capturer.is_none() {
            let monitor = get_monitor_from_index(self.selected_monitor).unwrap();
            self.height = monitor.height() as u32;
            self.width = monitor.width() as u32;
            log::debug!("Monitor dimensions: {}x{}", self.width, self.height);
            self.capturer = Some(Capturer::new(monitor).expect("Couldn't begin capture."));
        }

        let capturer = self.capturer.as_mut().unwrap();
        return match capturer.frame() {
            Ok(frame) => {
                let mut buffer: Vec<u8> = frame.iter().cloned().collect();
                // Convert BGRA to RGBA
                buffer = bgra_to_rgba(&mut buffer).to_vec();
                let image: ImageBuffer<Rgba<u8>, Vec<u8>> =
                    ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(self.width, self.height, buffer)
                        .unwrap();

                log::debug!(
                    "Captured frame with dimensions {}x{}",
                    self.width,
                    self.height
                );
                Some(image)
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock => {
                    log::debug!("Frame not ready; skipping this frame.");
                    None
                }
                std::io::ErrorKind::ConnectionReset => {
                    // TODO: Fix 16:9 ratio issue
                    log::debug!("Strange Error: {e}. Resetting capturer. Make sure that if you changed your screen size, keep it at 16:9 ratio.");
                    self.reset_capture();

                    None
                }
                _ => {
                    log::error!("{e:?}");
                    None
                }
            },
        };
    }
}

fn bgra_to_rgba(buffer: &mut [u8]) -> &mut [u8] {
    assert_eq!(
        buffer.len() % 4,
        0,
        "Buffer length must be a multiple of 4."
    );
    for chunk in buffer.chunks_exact_mut(4) {
        chunk.swap(0, 2); // Swap B and R
    }
    buffer
}

fn get_monitors() -> Result<Vec<Display>, ()> {
    let monitors: Vec<Display> = Display::all().expect("Couldn't find any display.");
    if monitors.is_empty() {
        return Err(());
    }

    Ok(monitors)
}

pub fn get_monitor_from_index(index: usize) -> Result<Display, ()> {
    let mut monitors = get_monitors().unwrap();

    if index >= monitors.len() {
        return Err(());
    }

    let monitor: Display = monitors.remove(index);
    Ok(monitor)
}
