// https://github.com/emilk/egui/blob/master/examples/images/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::env;
mod annotation;
mod app;
mod area_capture;
mod common;
mod config;
mod data_streaming;
mod hotkey;
mod screen_capture;
mod video_recorder;

use app::RustreamApp;
use area_capture::AreaCaptureApp;
use egui::{Pos2, Vec2, ViewportBuilder, X11WindowType};

use env_logger::Env;
use log::{error, LevelFilter};

use eframe::NativeOptions;

const APP_TITLE: &str = "RUSTREAM";

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .filter_module("eframe", LevelFilter::Off)
        .filter_module("wgpu", LevelFilter::Off)
        .filter_module("naga", LevelFilter::Off)
        .filter_module("egui_wgpu", LevelFilter::Off)
        .filter_module("resvg", LevelFilter::Off)
        .init();

    let args: Vec<String> = env::args().collect();
    let is_overlay = args.iter().any(|arg| arg.contains("overlay"));
    let mode = args
        .iter()
        .find(|arg| arg.starts_with("--overlay:"))
        .map(|arg| arg.split(':').nth(1).unwrap_or(""))
        .unwrap_or("");

    let rustream_options: eframe::NativeOptions = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: ViewportBuilder {
            transparent: Some(false),
            fullscreen: Some(false),
            title: Some(APP_TITLE.to_string()),
            window_type: Option::from(X11WindowType::Toolbar),
            min_inner_size: Some(Vec2::new(700.0, 500.0)),
            ..Default::default()
        },
        ..Default::default()
    };

    let (width, height, scale_factor, window_x, window_y) = if is_overlay {
        let w = args
            .get(4)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(1920.0);
        let h = args
            .get(5)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(1080.0);
        let scale = args
            .get(6)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(1.0);
        let x = args
            .get(2)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0);
        let y = args
            .get(3)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0);

        //apply scale factor to the window size
        let scaled_width = w / scale;
        let scaled_height = h / scale;

        (scaled_width, scaled_height, scale, x, y)
    } else {
        (1920.0, 1080.0, 1.0, 0.0, 0.0)
    };

    let overlay_options = NativeOptions {
        renderer: eframe::Renderer::Glow,
        //persist_window: true,
        viewport: ViewportBuilder {
            transparent: Some(true),
            fullscreen: Some(false),
            maximized: Some(true),
            decorations: Some(false),
            position: Some(Pos2::new(window_x / scale_factor, window_y / scale_factor)),
            title: Some(APP_TITLE.to_string()),
            resizable: Some(false),
            window_type: Option::from(X11WindowType::Toolbar),
            inner_size: Some(Vec2::new(width, height)),
            ..Default::default()
        },
        ..Default::default()
    };

    if is_overlay {
        match mode {
            "selection" => {
                eframe::run_native(
                    "Select Area",
                    overlay_options,
                    Box::new(|_cc| Ok(Box::new(AreaCaptureApp::default()))),
                )
                .expect("Failed to run Resize Screen");
            }
            "annotation" => {
                eframe::run_native(
                    "Annotation",
                    overlay_options,
                    Box::new(|_cc| Ok(Box::new(annotation::AnnotationApp::default()))),
                )
                .expect("Failed to run Resize Screen");
            }
            _ => {
                error!("Invalid mode: {}", mode);
            }
        }
        return;
    }

    eframe::run_native(
        APP_TITLE,
        rustream_options,
        Box::new(|cc: &eframe::CreationContext<'_>| Ok(Box::new(RustreamApp::new(cc)))),
    )
    .expect("Failed to run RustreamApp");
}
