// https://github.com/emilk/egui/blob/master/examples/images/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::env;
mod app;
mod audio_capture;
mod common;
mod config;
mod hotkey;
mod screen_capture;
mod video_recorder;
mod secondaryapp;

use app::{RustreamApp};
use secondaryapp::{SecondaryApp};
use egui::{Vec2, ViewportBuilder, X11WindowType, Pos2};
use env_logger::Env;
use log::LevelFilter;

use eframe::NativeOptions;

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

    let options: eframe::NativeOptions = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: ViewportBuilder {
            transparent: Some(false),
            fullscreen: Some(false),
            title: Some(APP_TITLE.to_string()),
            window_type: Option::from(X11WindowType::Toolbar),
            min_inner_size: Some(Vec2::new(870.0, 585.0)), 
            ..Default::default()
        },
        ..Default::default()
    };

    

    let (width, height, scale_factor) = if is_secondary {
        let w = args.get(4)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(1920.0);
        let h = args.get(5)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(1080.0);
        let scale = args.get(6)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(1.0);
        

        //apply scale factor to the window size
        let scaled_width = w * scale;
        let scaled_height = h * scale;

        (scaled_width ,scaled_height , scale)

        
    } else {
        (1920.0, 1080.0, 1.0)
    };

    let (window_x, window_y) = if is_secondary {
        let x = args.get(2)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0);
        let y = args.get(3)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0);
        (x / scale_factor, y / scale_factor)
    } else {
        (0.0, 0.0)
    };

    //scale width height and scale factor
    log::info!("width: {}, height: {}, scale_factor: {}", width, height, scale_factor);
    //apply the scale factor to the window size
     

    let options2 = NativeOptions {
        renderer: eframe::Renderer::Glow,
        //persist_window: true,
        viewport: ViewportBuilder {
            transparent: Some(true),
            fullscreen: Some(false),
            maximized: Some(true), 
            decorations: Some(false),
            position: Some(Pos2::new(window_x, window_y)),
            title: Some(APP_TITLE.to_string()),
            resizable: Some(false),
            window_type: Option::from(X11WindowType::Toolbar),
            inner_size: Some(Vec2::new(width, height)),
            ..Default::default()
        },

        ..Default::default()
    };


    if is_secondary {
        eframe::run_native(
            "Resize Me",
            options2,
            Box::new(|_cc| Ok(Box::new(SecondaryApp::default()))),
        )
        .expect("Failed to run Resize Screen");
    } else {
        eframe::run_native(
            APP_TITLE,
            options,
            Box::new(|cc: &eframe::CreationContext<'_>| Ok(Box::new(RustreamApp::new(cc)))),
        )
        .expect("Failed to run RustreamApp");
    }
}
