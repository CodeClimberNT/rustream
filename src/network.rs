use crate::{annotations, capture, hotkeys, recording};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use futures::FutureExt;

#[derive(Serialize, Deserialize)]
struct StreamData {
    frame: Vec<u8>,
    width: u32,
    height: u32,
}

pub async fn start_streaming(
    area: Option<capture::ScreenArea>,
    hotkey_config: Arc<hotkeys::HotkeyConfig>,
    annotation_state: Arc<Mutex<annotations::AnnotationState>>,
) {
    let listener = TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("Failed to bind");
    println!("Streaming on port 8080");

    loop {
        let (mut socket, _) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Failed to accept connection: {}", e);
                continue;
            }
        };
        let area = area.clone();
        let hotkey_config = Arc::clone(&hotkey_config);
        let annotation_state = Arc::clone(&annotation_state);
        tokio::spawn(async move {
            loop {
                // Check hotkey state
                if hotkeys::should_terminate(&hotkey_config) {
                    break;
                }
                if hotkeys::is_paused(&hotkey_config) {
                    continue;
                }

                // Capture screen data
                let mut screen_data = capture::capture_screen(area.clone());

                let state = annotation_state.lock().await;
                if state.active {
                    annotations::apply_annotations(&mut screen_data, &state);
                }

                // Serialize screen data
                let data = StreamData {
                    frame: screen_data.data,
                    width: screen_data.width,
                    height: screen_data.height,
                };
                let serialized = serde_json::to_vec(&data).expect("Failed to serialize");

                // Send data
                if let Err(e) = socket.write_all(&serialized).await {
                    println!("Failed to send data: {}", e);
                    break;
                }
            }
        });
    }
}

pub async fn start_receiving(address: &str, enable_recording: bool) {
    match TcpStream::connect(address).await {
        Ok(mut stream) => {
            let mut buffer = Vec::new();
            let (recorder_tx, mut recorder_rx) = tokio::sync::mpsc::channel::<(Vec<u8>, u32, u32)>(32);
            let recorder_shutdown = Arc::new(tokio::sync::Notify::new());
            let shutdown_signal = recorder_shutdown.clone();

            if enable_recording {
                std::thread::spawn(move || {
                    // Wait for the first frame to get dimensions
                    let (first_frame, width, height) = recorder_rx.blocking_recv().unwrap();
                    let mut recorder = recording::start_recording("received_output.mp4", width, height);
                    // Record the first frame
                    recorder.record_frame(&first_frame, width, height);
                    loop {
                        if recorder_shutdown.notified().now_or_never().is_some() {
                            recording::stop_recording(&mut recorder);
                            break;
                        }
                        if let Some(frame_data) = recorder_rx.blocking_recv() {
                            let (frame, width, height) = frame_data;
                            recorder.record_frame(&frame, width, height);
                        }
                    }
                });
            }

            // Add cleanup on connection drop
            tokio::spawn(async move {
                tokio::signal::ctrl_c().await.ok();
                shutdown_signal.notify_one();
            });

            loop {
                let mut temp_buffer = vec![0; 65536]; // Adjust buffer size as needed
                match stream.read(&mut temp_buffer).await {
                    Ok(0) => break,
                    Ok(n) => buffer.extend_from_slice(&temp_buffer[..n]),
                    Err(e) => {
                        eprintln!("Failed to read from stream: {}", e);
                        break;
                    }
                }
                // Deserialize screen data
                if let Ok(data) = serde_json::from_slice::<StreamData>(&buffer) {
                    // Handle received screen data
                    if enable_recording {
                        recorder_tx.send((data.frame, data.width, data.height)).await.unwrap();
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to {}: {}", address, e)
        }
    }
}
