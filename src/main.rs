mod capture_screen;

mod app;
use app::AppInterface;

const APP_TITLE: &str = "RUSTREAM";

fn main() -> Result<(), eframe::Error> {
    eframe::run_native(
        APP_TITLE,
        eframe::NativeOptions::default(),
        Box::new(|cc: &eframe::CreationContext<'_>| Ok(Box::new(AppInterface::new(cc)))),
    )
}
