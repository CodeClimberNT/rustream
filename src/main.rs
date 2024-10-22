// https://github.com/emilk/egui/blob/master/examples/images/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod capture_screen;

mod app;
use app::AppInterface;

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    const APP_TITLE: &str = "RUSTREAM";
    
    // make the options easier to change
    let options: eframe::NativeOptions = eframe::NativeOptions {
        ..Default::default()
    };


    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|cc: &eframe::CreationContext<'_>| Ok(Box::new(AppInterface::new(cc)))),
    )
}
