// https://github.com/emilk/egui/blob/master/examples/images/src/main.rs
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;
mod screen_capture;

use app::AppInterface;
use env_logger::Env;
use log::LevelFilter;

const APP_TITLE: &str = "RUSTREAM";

fn main() -> Result<(), eframe::Error> {
    // env_logger::init();
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .filter_module("eframe", LevelFilter::Info)
        .filter_module("wgpu", LevelFilter::Off)
        .filter_module("naga", LevelFilter::Off)
        .filter_module("egui_wgpu", LevelFilter::Off)
        .init();

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
