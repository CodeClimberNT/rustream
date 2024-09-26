mod app;
mod capture_screen;

// use capture_screen::capture_screen;
use app::AppInterface;

const APP_TITLE: &str = "Multi-Platform Screen Casting";

fn main() -> Result<(), eframe::Error> {
    eframe::run_native(
        APP_TITLE,
        eframe::NativeOptions::default(),
        Box::new(|cc: &eframe::CreationContext<'_>| Ok(Box::new(AppInterface::new(cc)))),
    )
}
