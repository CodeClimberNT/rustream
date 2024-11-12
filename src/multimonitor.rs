use scrap::Display;

#[derive(Clone)]
pub struct MonitorInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
}

pub fn list_monitors() -> Vec<MonitorInfo> {
    let displays = match Display::all() {
        Ok(displays) => displays,
        Err(e) => {
            eprintln!("Failed to get displays: {}", e);
            return Vec::new();
        }
    };

    displays
        .iter()
        .enumerate()
        .map(|(i, display)| MonitorInfo {
            name: format!("Display {}", i + 1),
            width: display.width() as u32,
            height: display.height() as u32,
        })
        .collect()
}

pub fn select_monitor(index: usize) -> MonitorInfo {
    let displays = Display::all().unwrap();
    if let Some(display) = displays.get(index) {
        MonitorInfo {
            name: format!("Display {}", index + 1),
            width: display.width() as u32,
            height: display.height() as u32,
        }
    } else {
        // Fallback to primary display
        let primary = Display::primary().unwrap();
        MonitorInfo {
            name: "Primary Display".to_string(),
            width: primary.width() as u32,
            height: primary.height() as u32,
        }
    }
}
