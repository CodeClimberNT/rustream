use std::net::SocketAddr;
use std::str::from_utf8;
use std::sync::Arc;
use std::thread::spawn;
use std::time::Instant;
use tokio::net::UdpSocket;
use tokio::sync::{Mutex, Notify};
use tokio::task::JoinSet;
use tokio::time::sleep;
use tokio::time::Duration;

use crate::screen_capture::CapturedFrame;

pub const PORT: u16 = 56123;
const MAX_DATAGRAM_SIZE: usize = 65507; //1472
const SEQ_NUM_SIZE: usize = size_of::<u16>(); // Size of sequence number, 2
const FRAME_ID_SIZE: usize = size_of::<u32>(); // Size of frame_id, 4

pub struct Sender {
    socket: Arc<UdpSocket>,
    receivers: Arc<Mutex<Vec<SocketAddr>>>,
    frame_id: u32,
    pub started_sending: bool,
}

impl Sender {
    //initialize caster UdpSocket
    pub async fn new() -> Self {
        let addr = format!("0.0.0.0:{}", PORT);
        let sock = UdpSocket::bind(addr).await.unwrap();
        println!("Socket {} ", sock.local_addr().unwrap());
        /*let sock = match sock {
            Ok(socket) => socket,
            Err(_) => {
                println!("Failed to bind socket  to port 50000, binding to default port");
                let default_sock = UdpSocket::bind("0.0.0.0:0").await.unwrap();
                println!("Socket bound to port {}",  default_sock.local_addr().unwrap().port()); //.to_string()?
                default_sock
            }
            //sock = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        };*/
        Self {
            socket: Arc::new(sock),
            receivers: Arc::new(Mutex::new(Vec::new())),
            frame_id: 0,
            started_sending: false,
        }
    }

    // Start listening for new receivers in the background
    pub async fn listen_for_receivers(&self, stop_notify: Arc<Notify>) {
        let receivers = self.receivers.clone();
        let socket = self.socket.clone();

        tokio::spawn(async move {  //implementare meccanismo per stoppare il loop 
            //atomicbool che metto a true quando instnazio il sender e a false in reset_ui
            loop {
                let mut buf = [0; 1472];

                tokio::select! {
                    _ = stop_notify.notified() => {
                        println!("Stop notify received, exiting listener loop.");
                        break;
                    },

                    result = socket.recv_from(&mut buf) => {
                        match result {

                            //recv_from to receive from different clients
                            Ok((_, peer_addr)) => {
                                if let Ok(message) = from_utf8(&buf) {
                                    if message.trim_matches('\0') == "REQ_FRAMES" {
                                        println!("Received connection request from: {}", &peer_addr);
                                        let mut receivers = receivers.lock().await;
                                        if !receivers.contains(&peer_addr) {
                                            receivers.push(peer_addr);
                                        }
                                        println!("New receiver connected: {}", peer_addr);
                                    } else if message.trim_matches('\0') == "CLOSE_CONNECTION" {
                                        println!("Received close connection request from: {}", &peer_addr);
                                        let mut receivers = receivers.lock().await;
                                        if receivers.contains(&peer_addr) {
                                            receivers.retain(|&x| x != peer_addr); // Remove the disconnected receiver
                                            println!("Receiver {} disconnected", peer_addr);
                                        }
                                    }
                                }
                            }

                            Err(ref e) if e.kind() == std::io::ErrorKind::ConnectionReset => {
                                eprintln!("Connection reset by peer: {}", e);
                                // Handle connection reset, possibly retry connection
                                /*let buffer = "TRY_RECONNECTION".as_bytes();
                                if let Err(_)  = socket.try_send(buffer) {
                                    //Ok(_) => Ok(socket),
                                eprintln!("Failed to reconnect to receiver");
                                }*/
                            }
                            Err(e) => eprintln!("Error receiving connection: {}", e),
                        }
                    }
                }
            }
        });
    }

    pub async fn send_data(&mut self, frame: CapturedFrame) -> Result<(), Box<dyn std::error::Error>> {
        let receivers = self.receivers.lock().await;
        
        // Return early if no receivers
        if receivers.is_empty() {
            println!("No receivers connected");
            return Ok(()); 
        }

        let start = Instant::now();
        let encoded_frame = frame.encode_to_h265()?;
        let encode_time = start.elapsed();
        println!("Encoding time: {:?}", encode_time);
        println!("Frame encoded to h265");

        // Increase frame_id
        self.frame_id += 1;
        let fid = self.frame_id;
        println!("Frame id: {:?}", fid);

        //let mut seq_num: u16; //  = 0 se for chunk for peer
        //encoded_frame size = num elements (len()) * size of element (u8)[1 byte]
        let total_chunks = (encoded_frame.len() as f32
            / (MAX_DATAGRAM_SIZE - 2 * SEQ_NUM_SIZE - FRAME_ID_SIZE) as f32)
            .ceil() as u16;
        println!("Total chunks: {:?}", total_chunks);

        //let mut tasks = Vec::new();
        let encoded_frame = Arc::new(encoded_frame);
        //let mut set = JoinSet::new();

        for &peer in receivers.iter() {
        
            let socket = self.socket.clone();
            let encoded_frame1 = encoded_frame.clone();
            
            tokio::spawn( async move {
                let mut seq_num: u16 = 0;

                for chunk in &mut encoded_frame1.chunks(MAX_DATAGRAM_SIZE - 2 * SEQ_NUM_SIZE - FRAME_ID_SIZE) {
                
                    let mut pkt = Vec::new();
                    pkt.extend_from_slice(&seq_num.to_ne_bytes()); //&seq_num.to_ne_bytes()
                    pkt.extend_from_slice(&total_chunks.to_ne_bytes());
                    pkt.extend_from_slice(&fid.to_ne_bytes());
                    pkt.extend_from_slice(chunk);

                    if let Err(e) = socket.send_to(&pkt, peer).await {
                        eprintln!("Error sending to {}: {}", peer, e);
                    }
                    println!("Sent chunk {:?} to peer {}", seq_num, peer);
                    
                    /*if let Err(e) = self.socket.send_to(&pkt, peer).await {
                        eprintln!("Error sending to {}: {}", peer, e);
                    }
                    println!("Sent chunk {:?} to peer {}", seq_num, peer);*/
                    // Small delay to avoid flooding the network
                    //sleep(Duration::from_micros(600)).await;
                    seq_num += 1;
                    //tokio::time::sleep(Duration::from_micros(100)).await; //sleep for 100 microseconds before sending next chunk
                }
            });
            //seq_num += 1;
            
        }

        // Wait for all tasks to complete
        /*while let Some(res) = set.join_next().await {
            if let Err(e) = res {
                eprintln!("Task failed: {:?}", e);
            }
        }*/
        Ok(())
    }

    // Send end of stream messageto all receivers
    pub async fn end_stream(&self) {
        let mut receivers = self.receivers.lock().await;

        for &peer in receivers.iter() {
            let socket = self.socket.clone();
            
            tokio::spawn( async move {
                let buf = "END_STREAM".as_bytes();
                if let Err(e) = socket.send_to(&buf, peer).await {
                    eprintln!("Error sending END_STREAM to {}: {}", peer, e);
                }
                println!("Sent END_STREAM to peer {}", peer);
            });
        }
        //receivers.clear();
    }
}

pub async fn start_streaming(
    sender: Arc<Mutex<Sender>>,
    frame: CapturedFrame,
    stop_notify: Arc<Notify>,
) -> Result<(), Box<dyn std::error::Error>> {
    
    let mut sender = sender.lock().await;
    
    if !sender.started_sending {
        sender.started_sending = true;
        
        sender.listen_for_receivers(stop_notify).await;
    }

    return match sender.send_data(frame).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    };
}