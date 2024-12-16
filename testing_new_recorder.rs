use crate::RecorderCommand;
use scap::{
    capturer::{Area, Capturer, Options, Point, Size},
    frame::Frame,
};
use std::sync::mpsc::{Receiver, Sender};

pub fn start_recorder(request_rx: Receiver<RecorderCommand>, frame_tx: Sender<Frame>) {
    // Check if the platform is supported
    if !scap::is_supported() {
        println!("❌ Platform not supported");
        return;
    }

    // Check if we have permission to capture screen
    // If we don't, request it.
    if !scap::has_permission() {
        println!("❌ Permission not granted. Requesting permission...");
        if !scap::request_permission() {
            println!("❌ Permission denied");
            return;
        }
    }

    // Initialize the recorder with desired options
    let options = Options {
        fps: 60,
        show_cursor: true,
        show_highlight: true,
        excluded_targets: None,
        output_type: scap::frame::FrameType::BGRAFrame,
        output_resolution: scap::capturer::Resolution::_720p,
        crop_area: Some(Area {
            origin: Point { x: 0.0, y: 0.0 },
            size: Size {
                width: 500.0,
                height: 500.0,
            },
        }),
        ..Default::default()
    };

    let mut recorder = Capturer::build(options).unwrap_or_else(|err| {
        println!("Problem with building Capturer: {}", err);
        std::process::exit(1);
    });

    // Start Capture
    recorder.start_capture();

    // Wait for requests to capture frames
    while let Ok(command) = request_rx.recv() {
        match command {
            RecorderCommand::Capture => match recorder.get_next_frame() {
                Ok(frame) => frame_tx.send(frame).unwrap(),
                Err(err) => println!("Error capturing frame: {}", err),
            },
            RecorderCommand::Stop => break,
        }
    }

    // Stop Capture
    recorder.stop_capture();
}
