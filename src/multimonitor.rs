use scrap::Display;

#[derive(Clone)]
pub struct MonitorInfo {
    pub id: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
}

pub fn list_monitors() -> Vec<MonitorInfo> {
    let displays = match Display::all() {
        Ok(displays) => displays,
        Err(e) => {
            eprintln!("Could not get displays: {}", e);
            return Vec::new();
        }
    };

    displays
        .iter()
        .enumerate()
        .map(|(i, display)| MonitorInfo {
            id: i,
            name: format!("Monitor {}", i + 1),
            width: display.width() as u32,
            height: display.height() as u32,
            // refresh_rate: display.refresh_rate(),
        })
        .collect()
}

pub fn select_monitor(monitor_id: usize) -> MonitorInfo {
    list_monitors()
        .into_iter()
        .find(|m| m.id == monitor_id)
        .unwrap_or_else(|| {
            eprintln!(
                "Monitor with ID {} not found. Selecting primary monitor.",
                monitor_id
            );
            list_monitors()
                .first()
                .cloned()
                .expect("No monitors available.")
        })
}
