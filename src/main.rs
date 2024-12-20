// https://github.com/emilk/egui/blob/master/examples/images/src/main.rs
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
use std::env;
mod app;
// mod audio_capture;
mod common;
mod config;
mod frame_grabber;
mod video_recorder;

use app::{RustreamApp, SecondaryApp};
use egui::debug_text::print;
use egui::{ViewportBuilder, X11WindowType};
use env_logger::Env;
use log::LevelFilter;
use eframe::egui::Pos2;

use std::sync::{Arc, Mutex};
use eframe::egui::Vec2;
use eframe::NativeOptions;

use winit;

const APP_TITLE: &str = "RUSTREAM";

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .filter_module("eframe", LevelFilter::Off)
        .filter_module("wgpu", LevelFilter::Off)
        .filter_module("naga", LevelFilter::Off)
        .filter_module("egui_wgpu", LevelFilter::Off)
        .filter_module("resvg", LevelFilter::Off)
        .init();

    let args: Vec<String> = env::args().collect();
    let is_secondary = args.iter().any(|arg| arg == "--secondary");
    let rustream_app = Arc::new(Mutex::new(RustreamApp::default()));
    // make the options easier to change
    let options: eframe::NativeOptions = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: ViewportBuilder {
            transparent: Some(false),
            fullscreen: Some(false),
            title: Some(APP_TITLE.to_string()),
            window_type: Option::from(X11WindowType::Toolbar),
            ..Default::default()
        },
        ..Default::default()
    };

    let options2 = NativeOptions {
        renderer: eframe::Renderer::Glow,
        //persist_window: true,
        viewport: ViewportBuilder {
            transparent: Some(true),
            fullscreen: Some(false),
            min_inner_size: Some(Vec2::new(1920.0, 1080.0)), // Set the desired full screen size
            decorations: Some(false),
            title: Some(APP_TITLE.to_string()),
            position: Some(Pos2::new(0.0, 0.0)),
            window_type: Option::from(X11WindowType::Toolbar),
            ..Default::default()
        },
        
        ..Default::default()
    };

    if is_secondary {
        println!("Running Secondary App");
        eframe::run_native(
            "Resize Me",
            options2,
            Box::new(|_cc| Ok(Box::new(SecondaryApp::new(rustream_app.clone())))),
        )
        .expect("Failed to run Resize Screen");
        return;
    }
    else {
    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|cc: &eframe::CreationContext<'_>| Ok(Box::new(RustreamApp::new(cc)))),
    )
    .expect("Failed to run RustreamApp");
    }
}
