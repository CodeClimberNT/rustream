// https://github.com/emilk/egui/blob/master/examples/images/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod annotations;
mod capture;
mod hotkeys;
mod multimonitor;
mod network;
mod recording;
mod ui;

fn main() {
    // Initialize the logger
    env_logger::init();

    // Start the UI
    ui::initialize_ui();
}
