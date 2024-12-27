// https://github.com/emilk/egui/blob/master/examples/images/src/main.rs
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;
// mod audio_capture;
mod common;
mod config;
mod frame_grabber;
mod hotkey;
mod video_recorder;

use app::RustreamApp;
use egui::{ViewportBuilder, X11WindowType};
use env_logger::Env;
use log::LevelFilter;

const APP_TITLE: &str = "RUSTREAM";

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .filter_module("eframe", LevelFilter::Off)
        .filter_module("wgpu", LevelFilter::Off)
        .filter_module("naga", LevelFilter::Off)
        .filter_module("egui_wgpu", LevelFilter::Off)
        .filter_module("resvg", LevelFilter::Off)
        .init();

    //TODO: min size
    //~870x630

    let options: eframe::NativeOptions = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: ViewportBuilder {
            transparent: Some(true),
            title: Some(APP_TITLE.to_string()),
            window_type: Option::from(X11WindowType::Toolbar),
            ..Default::default()
        },
        ..Default::default()
    };

    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|cc: &eframe::CreationContext<'_>| Ok(Box::new(RustreamApp::new(cc)))),
    )
    .expect("Failed to run RustreamApp");
}
