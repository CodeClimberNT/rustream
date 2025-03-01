use std::collections::VecDeque;
use std::io::ErrorKind::{self, ConnectionReset, WouldBlock};
use std::io::Write;
use std::net::SocketAddr;
use std::process::{Command, Stdio};
use std::str::from_utf8;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, Mutex, Notify};
use tokio::time::{interval, Duration};

use crate::screen_capture::CapturedFrame;

pub struct Receiver {
    pub socket: UdpSocket, //Arc<UdpSocket>
    pub started_receiving: bool,
    caster: SocketAddr,
}

impl Receiver {
    //create a new receiver, its socket and connect to the caster
    pub async fn new(caster: SocketAddr) -> Self {
        //Result<Self, std::io::Error>
        let sock = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        println!("Socket {} ", sock.local_addr().unwrap());
        let buf = "REQ_FRAMES".as_bytes();

        if let Ok(_) = sock.connect(caster).await {
            //connects socket to send/receive only from sender_addr
            match sock.send(buf).await {
                //send datagram to caster to request the streaming
                Ok(_) => {
                    println!("Connected to sender");
                }
                Err(e) if e.kind() == ErrorKind::ConnectionReset => {
                    eprintln!("Destination unreachable: {}", e);
                    //return Err(e);
                }
                Err(e) => {
                    println!("Failed to send registration request: {}", e);
                    //return Err(e);
                }
            }
        }

        Self {
            socket: sock,
            started_receiving: false,
            caster,
        }
    }

    pub async fn recv_data(
        &mut self,
        tx: mpsc::Sender<Vec<u8>>,
        stop_notify: Arc<Notify>,
        stream_ended: Arc<AtomicBool>,
    ) -> Result<(), std::io::Error> {
        //let mut buf =  vec![0; MAX_DATAGRAM_SIZE]; //[0; 1024]; //aggiustare dimesione buffer, troppo piccola per datagramma
        let mut frame_chunks: Vec<(u16, Vec<u8>)> = Vec::new();
        let mut frame: Vec<u8> = Vec::new();
        let mut fid: u32 = 1;
        let mut received_chunks = std::collections::HashSet::new();

        const MAX_DATAGRAM_SIZE: usize = 65507;

        loop {
            let mut buf = vec![0; MAX_DATAGRAM_SIZE];

            tokio::select! {
                _ = stop_notify.notified() => {
                    println!("Received stop signal, exiting recv_data");
                    break; // Gracefully exit when `notify_waiters()` is called
                }

                res = self.socket.recv_from(&mut buf) => { // Keep listening for UDP packets
                    match res {
                        Ok((len, _)) => {

                            // if received END_STREAM message, return earlier
                            if let Ok(message) = from_utf8(&buf) {
                                if message.trim_matches('\0') == "END_STREAM" {
                                    println!("Received END_STREAM message");
                                    stream_ended.store(true, Ordering::SeqCst);
                                    //return Ok(());
                                    continue;
                                }
                            }

                            if  len > 8 {  // Ensure it's a frame and not an empty message
                                stream_ended.store(false, Ordering::SeqCst);
                            }

                            //stream_ended.store(false, Ordering::SeqCst);
                            let frame_id = u32::from_ne_bytes(buf[4..8].try_into().unwrap());

                            let seq_num = u16::from_ne_bytes(buf[0..2].try_into().unwrap());
                            println!("Received Frame {:?}, chunk {:?}", frame_id, seq_num);

                            let total_chunks = u16::from_ne_bytes(buf[2..4].try_into().unwrap());

                            //Frame id changed
                            if frame_id != fid {
                                //last chunk of previous frame not received
                                if !frame_chunks.is_empty() { //new frame while previous chunks are not all received, otherwise frame_chunks would have been cleared
                                    frame_chunks.clear();
                                    received_chunks.clear();
                                    frame.clear();
                                    println!("Wrong frame id: {:?}, previous frame discarded", frame_id);
                                }

                                fid = frame_id;

                            }

                            let chunk_data = buf[8..len].to_vec();
                            received_chunks.insert(seq_num);
                            frame_chunks.push((seq_num, chunk_data));
                            //println!("Frame_chunks len: {:?}", frame_chunks.len());

                            if seq_num == total_chunks - 1 {
                                //If all chunks of frame are received, sort chunks and decode frame
                                if received_chunks.len() == total_chunks as usize {
                                    frame_chunks.sort_by(|a, b| a.0.cmp(&b.0));
                                    println!("Frame_chunks sorted, index order: {:?}", frame_chunks.iter().map(|(i, _)| i).collect::<Vec<_>>());

                                    for (_, ref chunk) in &frame_chunks {
                                        frame.append(&mut chunk.clone());
                                    }

                                    let encoded_frame = frame.clone();
                                    if let Err(e) = tx.send(encoded_frame).await {
                                        eprintln!("❌ Error sending decoded frame to start_receiving: {}", e);
                                    }
                                }

                                // Clear frame_chunks and received_chunks for the next frame
                                frame_chunks.clear();
                                received_chunks.clear();
                                frame.clear();
                            }
                        },
                        Err(e) if e.kind() == WouldBlock => {
                            continue;
                        },
                        Err(e) if e.kind() == ConnectionReset => {
                            println!("Host unreachable");
                            //self.host_unreachable = true;
                            return Err(e);
                        },
                        Err(e) => {
                            println!("Error in receiving data {:?}", e);
                            return Err(e);
                        }
                    }
                }
            }
        }
        Ok(()) //when not receiving return
    }

    pub fn stop_receiving(&self) {
        let buf = "CLOSE_CONNECTION".as_bytes();
        match self.socket.try_send(buf) {
            Ok(_) => {
                println!("Close connection request sent to sender");
            }
            Err(_) => println!("Failed to close connection"),
        }
    }
}

impl Drop for Receiver {
    fn drop(&mut self) {
        self.stop_receiving();
    }
}

async fn process_frame(frames_vec: Arc<std::sync::Mutex<VecDeque<CapturedFrame>>>, frame: Vec<u8>) {
    // mut rx: mpsc::Receiver<Vec<u8>>

    let start = Instant::now();
    let decoded_frame = decode_from_h265_to_rgba(frame);
    let decode_time = start.elapsed();
    println!("Decoding time: {:?}", decode_time);
    match decoded_frame {
        Ok(frame) => {
            let mut frames = frames_vec.lock().unwrap();
            frames.push_back(frame);
            //println!("Frame pushed to frames_vec");
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
) {
    //println!("Inside start_receiving");

    let stop_notify1 = stop_notify.clone();
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(100);

    tokio::spawn(async move {
        let mut recv = receiver.lock().await;
        println!("Calling recv_data");

        if let Err(_) = recv.recv_data(tx, stop_notify1, stream_ended).await {
            //println!("Error receiving frame: {}", e);
            host_unreachable.store(true, Ordering::SeqCst);

            //let mut timer = interval(Duration::from_secs(5)); // Runs every 5 seconds

            //Try to reconnect every 5 seconds until reconnected
            /*loop {
                timer.tick().await; // Wait for the next tick
                println!("Trying to reconnect to sender");
                if let Ok(_) = recv.reconnect_to_sender().await { // non funziona, dà sempre ok anche se sender non è raggiungibile
                    break;
                }
            }*/
        }
        drop(recv);
    });

    //while let Some(frame) = rx.recv().await {
    loop {
        let frames_vec1 = frames_vec.clone();
        tokio::select! {
            _ = stop_notify.notified() => {
                println!("Received stop signal, exiting start_receiving");
                break; // Gracefully exit when `notify_waiters()` is called
            }

            Some(frame) = rx.recv() => {

                tokio::spawn(async move {
                    //println!("Calling process_frame");
                    process_frame(frames_vec1, frame).await;
                    // frames_vec is the vector of frames to share with ui
                });
            }
        }
    }
}

fn decode_from_h265_to_rgba(
    frame_data: Vec<u8>,
) -> Result<CapturedFrame, Box<dyn std::error::Error + Send + Sync>> {
    let (rgba_data, width, height) = crate::ffmpeg_utils::decode_from_h265(&frame_data)?;
    Ok(CapturedFrame::from_rgba_vec(rgba_data, width, height))
}


