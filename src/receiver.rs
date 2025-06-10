use crate::screen_capture::{decode_from_h265_to_rgba, CapturedFrame};

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::str::from_utf8;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex, Notify};

pub struct Receiver {
    pub socket: TcpStream,
    pub started_receiving: bool,
}

impl Receiver {
    //create a new receiver, its socket and connect to the caster
    pub async fn new(caster: SocketAddr) -> Result<Self, std::io::Error> {
        match TcpStream::connect(caster).await {
            Ok(stream) => {
                println!("Connected to sender at {}", caster);

                Ok(Self {
                    socket: stream,
                    started_receiving: false,
                })
            }
            Err(e) => {
                eprintln!("Failed to connect to sender: {}", e);
                Err(e)
            }
        }
    }

    pub async fn recv_data(
        &mut self,
        tx: mpsc::Sender<Vec<u8>>,
        stop_notify: Arc<Notify>,
        stream_ended: Arc<AtomicBool>,
    ) -> Result<(), std::io::Error> {
        loop {
            let mut buf = vec![0; 4]; // Buffer to read frame size

            tokio::select! {
                _ = stop_notify.notified() => {
                    println!("Received stop signal, exiting recv_data");
                    break; // Gracefully exit when `notify_waiters()` is called
                }
                result = self.socket.read_exact(&mut buf) => {  //read frame size or end message
                    match result {
                        Ok(0) => {
                            println!("Connection closed by sender");
                            stream_ended.store(true, Ordering::SeqCst);
                            break;
                        }
                        Ok(_) => {
                            // if received END message, return earlier
                            if let Ok(message) = from_utf8(&buf) {
                                if message.trim_matches('\0') == "END" {
                                    println!("Received END message");
                                    stream_ended.store(true, Ordering::SeqCst);
                                    break;
                                }
                                else if message.trim_matches('\0') == "BLNK" {
                                    println!("Received BLNK message");
                                    if let Err(e) = tx.send(message.as_bytes().to_vec()).await {
                                        eprintln!("Error sending encoded frame to start_receiving: {}", e);
                                    }
                                    continue;
                                }
                            }

                            let frame_size = u32::from_ne_bytes(buf.try_into().unwrap());
                            let mut frame = vec![0; frame_size as usize];
                            println!("Frame size: {:?}", frame_size);

                            // Read the complete frame
                            match self.socket.read_exact(&mut frame).await {
                                Ok(0) => {
                                    println!("Connection closed by sender");
                                    stream_ended.store(true, Ordering::SeqCst);
                                    break;
                                }
                                Ok(_) => {
                                    println!("Frame received");
                                    if let Err(e) = tx.send(frame).await {
                                        eprintln!("Error sending encoded frame to start_receiving: {}", e);
                                    }

                                }
                                Err(e) => {
                                    eprintln!("Error receiving frame: {}", e);
                                    return Err(e);
                                }
                            }

                        }
                        Err(e) => {
                            eprintln!("Error receiving frame size: {}", e);
                            return Err(e);
                        }
                    }
                }
            }
        }
        Ok(()) //when not receiving return
    }
}

async fn process_frame(frames_vec: Arc<std::sync::Mutex<VecDeque<CapturedFrame>>>, frame: Vec<u8>) {
    let start = Instant::now();
    let decoded_frame = decode_from_h265_to_rgba(frame);
    let decode_time = start.elapsed();
    println!("Decoding time: {:?}", decode_time);
    match decoded_frame {
        Ok(frame) => {
            let mut frames = frames_vec.lock().unwrap();
            frames.push_back(frame);
        }
        Err(e) => {
            eprintln!("Error decoding frame: {}", e);
        }
    };
}

pub async fn start_receiving(
    frames_vec: Arc<std::sync::Mutex<VecDeque<CapturedFrame>>>,
    receiver: Arc<Mutex<Receiver>>,
    stop_notify: Arc<Notify>,
    host_unreachable: Arc<AtomicBool>,
    stream_ended: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
) {
    let stop_notify1 = stop_notify.clone();
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(100);

    tokio::spawn(async move {
        let mut recv = receiver.lock().await;
        println!("Calling recv_data");

        if recv
            .recv_data(tx, stop_notify1, stream_ended)
            .await
            .is_err()
        {
            host_unreachable.store(true, Ordering::SeqCst);
        }

        drop(recv);
    });

    loop {
        let frames_vec1 = frames_vec.clone();
        tokio::select! {
            _ = stop_notify.notified() => {
                println!("Received stop signal, exiting start_receiving");
                break; // Gracefully exit when `notify_waiters()` is called
            }

            Some(frame) = rx.recv() => {

                if !is_paused.load(Ordering::SeqCst) {

                    if let Ok(message) = from_utf8(&frame) {
                        //blank screen
                        if message.trim_matches('\0') == "BLNK" {

                            let mut blank_frame = Vec::with_capacity(1920 * 1080 * 4);
                            for _ in 0..(1920 * 1080) {
                                blank_frame.extend_from_slice(&[0, 0, 0, 255]); // BGRA o RGBA nero opaco
                            }
                            let mut frames = frames_vec1.lock().unwrap();

                            let frame = CapturedFrame::from_rgba_vec(
                                blank_frame,
                                1920,
                                1080,
                            );
                            frames.push_back(frame);
                        }
                    }
                    else {
                        tokio::spawn(async move {
                            println!("Calling process_frame");
                            process_frame(frames_vec1, frame).await;
                            // frames_vec is the vector of frames to share with ui
                        });
                    }
                }
            }
        }
    }
}
