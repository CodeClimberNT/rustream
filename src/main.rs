// https://github.com/emilk/egui/blob/master/examples/images/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// use log::info;
// use std::sync::Arc;

mod annotations;
mod capture;
mod hotkeys;
mod multimonitor;
mod network;
mod recording;
mod ui;

// use annotations::AnnotationState;
// use hotkeys::{initialize_hotkeys, HotkeyConfig};

fn main() {
    // Initialize the logger
    env_logger::init();

    // Start the UI
    ui::initialize_ui();
}
