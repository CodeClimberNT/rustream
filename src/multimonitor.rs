use scrap::Display;

#[derive(Clone)]
pub struct MonitorInfo {
    pub id: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
}

pub fn list_monitors() -> Vec<MonitorInfo> {
    let displays = Display::all().expect("Could not get displays");
    displays
        .iter()
        .enumerate()
        .map(|(i, display)| MonitorInfo {
            id: i,
            name: format!("Monitor {}", i + 1),
            width: display.width(),
            height: display.height(),
            refresh_rate: display.refresh_rate(),
        })
        .collect()
}

pub fn select_monitor(monitor_id: usize) -> MonitorInfo {
    list_monitors()
        .into_iter()
        .find(|m| m.id == monitor_id)
        .expect("Monitor not found")
}
