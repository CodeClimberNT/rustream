use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::io::{self, Interest};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, Notify, RwLock};

use crate::screen_capture::CapturedFrame;

pub const PORT: u16 = 56123;

pub struct Sender {
    receivers: Arc<RwLock<HashMap<SocketAddr, Arc<TcpStream>>>>,
    disconnected_peers: Arc<Mutex<Vec<SocketAddr>>>,
    frame_id: u32,
    pub started_sending: bool,
}

impl Sender {
    //initialize caster UdpSocket
    pub async fn new() -> Self {
        Self {
            receivers: Arc::new(RwLock::new(HashMap::new())),
            disconnected_peers: Arc::new(Mutex::new(Vec::new())),
            frame_id: 0,
            started_sending: false,
        }
    }

    // Start listening for new receivers in background
    pub async fn listen_for_receivers(&self, stop_notify: Arc<Notify>) {
        let receivers = self.receivers.clone();

        let listener = TcpListener::bind(format!("0.0.0.0:{}", PORT))
            .await
            .expect("Failed to bind TCP socket");
        println!("TCP Server listening on port {}", PORT);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = stop_notify.notified() => {
                        println!("Stop notify received, exiting listener loop.");
                        break;
                    },

                    Ok((socket, peer_addr)) = listener.accept() => {
                        //match result {
                        receivers.write().await.insert(peer_addr, Arc::new(socket));
                        println!("New receiver connected: {}", peer_addr);
                    }
                }
            }
        });
    }

    pub async fn send_data(
        &mut self,
        frame: CapturedFrame,
        is_blank_screen: Arc<AtomicBool>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let recv = self.receivers.clone();

        let mut disconnected_peers = self.disconnected_peers.lock().await;

        //Remove disconnected peers before sending data
        if !disconnected_peers.is_empty() {
            for peer in disconnected_peers.iter() {
                self.receivers.write().await.remove(peer); //the Drop trait for TcpStream will close the connection
                println!("Receiver {} disconnected", peer);
            }
            disconnected_peers.clear(); // Clear the disconnected peers after processing
        }
        drop(disconnected_peers);

        let receivers = self.receivers.read().await;

        // Return early if no receivers
        if receivers.is_empty() {
            println!("No receivers connected");
            return Ok(());
        }

        //let start = Instant::now();
        let encoded_frame = frame.encode_to_h265()?;
        //let encode_time = start.elapsed();
        //println!("Encoding time: {:?}", encode_time);
        println!("Frame encoded to h265");

        // Increase frame_id
        self.frame_id += 1;
        let fid = self.frame_id;
        println!("Frame id: {:?}", fid);
        drop(receivers);

        let recv = recv.read().await;

        for (peer_addr, stream) in recv.iter() {
            let disc_peers = self.disconnected_peers.clone();
            let encoded_frame1 = encoded_frame.clone();
            let stream1 = stream.clone();
            let peer_addr = *peer_addr;
            let is_blank_clone = is_blank_screen.clone();

            tokio::spawn(async move {
                loop {
                    // Check if the stream is writable
                    let ready = stream1.ready(Interest::WRITABLE).await.unwrap();

                    if ready.is_writable() {
                        // Try to write data, this may still fail with `WouldBlock`
                        // if the readiness event is a false positive.

                        let mut pkt = Vec::new();

                        if is_blank_clone.load(Ordering::SeqCst) {
                            // If it's a blank screen, send BLNK message
                            let message = b"BLNK";
                            pkt.extend_from_slice(message);
                        } else {
                            let frame_size = (encoded_frame1.len() as u32).to_ne_bytes();
                            // Create a packet with frame size and encoded frame
                            pkt.extend_from_slice(&frame_size);
                            pkt.extend_from_slice(&encoded_frame1);
                        }

                        match stream1.try_write(&pkt) {
                            Ok(0) => {
                                // If 0 bytes are written, the connection was likely closed.
                                eprintln!("Connection closed by peer");

                                //Add peer to disconnected_peers
                                let mut disconnected_peers = disc_peers.lock().await;
                                disconnected_peers.push(peer_addr);
                                println!("Peer added to disconnected_peers: {}", peer_addr);
                                drop(disconnected_peers);
                                return;
                            }
                            Ok(_) => {
                                println!("Sent frame {}", fid);
                                break;
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                // If the readiness event is a false positive, try again
                                continue;
                            }
                            Err(ref e)
                                if e.kind() == io::ErrorKind::BrokenPipe
                                    || e.kind() == io::ErrorKind::ConnectionReset
                                    || e.kind() == io::ErrorKind::ConnectionAborted =>
                            {
                                // Connection was closed by the peer
                                eprintln!("Connection closed: {:?}", e);

                                //Add peer to disconnected_peers
                                let mut disconnected_peers = disc_peers.lock().await;
                                disconnected_peers.push(peer_addr);
                                println!("Peer added to disconnected_peers: {}", peer_addr);
                                drop(disconnected_peers);
                                return;
                            }
                            Err(e) => {
                                // Handle other errors
                                eprintln!("Failed to write to socket: {:?}", e);
                                return;
                            }
                        }
                    }
                }
            });
        }
        Ok(())
    }

    // Send end of stream message to all receivers
    pub async fn end_stream(&self) {
        let receivers = self.receivers.clone();
        let receivers = receivers.read().await;

        for (peer, stream) in receivers.iter() {
            let stream1 = stream.clone();
            let peer1 = *peer;

            tokio::spawn(async move {
                let mut buf = vec![0; 4];
                let message = b"END";
                buf[..message.len()].copy_from_slice(message); // Copy message to buffer, last byte is 0

                loop {
                    let ready = stream1.ready(Interest::WRITABLE).await.unwrap();

                    if ready.is_writable() {
                        match stream1.try_write(&buf) {
                            Ok(_) => {
                                println!("Sent END to peer {}", peer1);
                                break;
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                // If the readiness event is a false positive, try again
                                continue;
                            }
                            Err(e) => {
                                // Handle other errors
                                eprintln!("Error sending END to {}: {}", peer1, e);
                                return;
                            }
                        }
                    }
                }
            });
        }
        let mut receivers = self.receivers.write().await;
        receivers.clear();
    }
}

pub async fn start_streaming(
    sender: Arc<Mutex<Sender>>,
    frame: CapturedFrame,
    stop_notify: Arc<Notify>,
    is_blank_screen: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut sender = sender.lock().await;

    if !sender.started_sending {
        sender.started_sending = true;

        sender.listen_for_receivers(stop_notify).await;
    }

    return match sender.send_data(frame, is_blank_screen).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    };
}
