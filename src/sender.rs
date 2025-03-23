use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{self, AsyncWriteExt, Interest};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, Notify, RwLock};

use crate::screen_capture::CapturedFrame;

pub const PORT: u16 = 56123;
const MAX_DATAGRAM_SIZE: usize = 65507; //1472
const SEQ_NUM_SIZE: usize = size_of::<u16>(); // Size of sequence number, 2
const FRAME_ID_SIZE: usize = size_of::<u32>(); // Size of frame_id, 4

pub struct Sender {
    //socket: Arc<UdpSocket>,
    receivers: Arc<RwLock<HashMap<SocketAddr, Arc<TcpStream>>>>,
    frame_id: u32,
    pub started_sending: bool,
}

impl Sender {
    //initialize caster UdpSocket
    pub async fn new() -> Self {
        /*let addr = format!("0.0.0.0:{}", PORT);
        let sock = UdpSocket::bind(addr).await.unwrap();
        println!("Socket {} ", sock.local_addr().unwrap());*/
        
        Self {
            //socket: Arc::new(sock),
            receivers: Arc::new(RwLock::new(HashMap::new())),
            frame_id: 0,
            started_sending: false,
        }
    }

    // Start listening for new receivers in the background
    pub async fn listen_for_receivers(&self, stop_notify: Arc<Notify>) {
        let receivers = self.receivers.clone();
        //let socket = self.socket.clone();
        let listener = TcpListener::bind(format!("0.0.0.0:{}", PORT)).await.expect("Failed to bind TCP socket");
        println!("TCP Server listening on port {}", PORT);

        tokio::spawn(async move {  //implementare meccanismo per stoppare il loop 
            //atomicbool che metto a true quando instnazio il sender e a false in reset_ui
            loop {

                tokio::select! {
                    _ = stop_notify.notified() => {
                        println!("Stop notify received, exiting listener loop.");
                        break;
                    },

                    //result = socket.recv_from(&mut buf) => {
                    Ok((socket, peer_addr)) = listener.accept() => {
                        //match result {
                        receivers.write().await.insert(peer_addr, Arc::new(socket));
                        //receivers.push(socket);
                        println!("New receiver connected: {}", peer_addr);


                            //recv_from to receive from different clients
                            //Ok((_, peer_addr)) => {
                                /*if let Ok(message) = from_utf8(&buf) {
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
                                }*/
                            //}

                            /*Err(ref e) if e.kind() == std::io::ErrorKind::ConnectionReset => {
                                eprintln!("Connection reset by peer: {}", e);
                                // Handle connection reset, possibly retry connection
                                /*let buffer = "TRY_RECONNECTION".as_bytes();
                                if let Err(_)  = socket.try_send(buffer) {
                                    //Ok(_) => Ok(socket),
                                eprintln!("Failed to reconnect to receiver");
                                }*/
                            }
                            Err(e) => eprintln!("Error receiving connection: {}", e),*/
                        //}
                    }
                }
            }
        });
    }

    pub async fn send_data(&mut self, frame: CapturedFrame) -> Result<(), Box<dyn std::error::Error>> {
        let recv = self.receivers.clone();
        let receivers = self.receivers.read().await;
        
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
        drop(receivers);
        
        let disconnected_peers = Arc::new(Mutex::new(Vec::new()));
        
        //for &peer in receivers.iter() {
        let recv = recv.read().await;
        for (_, stream) in recv.iter() {
            
            let disc_peers = disconnected_peers.clone();
            let encoded_frame1 = encoded_frame.clone();
            let stream1 = stream.clone();
            
            tokio::spawn( async move {
                
                loop{
                    //let mut stream = stream1.lock().await;
                    let ready = stream1.ready(Interest::WRITABLE).await.unwrap();

                    if ready.is_writable() {
                        // Try to write data, this may still fail with `WouldBlock`
                        // if the readiness event is a false positive.
                        let frame_size = (encoded_frame1.len() as u32).to_ne_bytes();
                        let mut pkt = Vec::new();
                        pkt.extend_from_slice(&frame_size);
                        pkt.extend_from_slice(&encoded_frame1); 
                        
                        /*if let Err(e) = stream.write_all(&encoded_frame).await {
                            match e.kind() {
                                ErrorKind::BrokenPipe | ErrorKind::ConnectionReset => {
                                    eprintln!("Connection closed by receiver.");
                                }
                                _ => {
                                    eprintln!("Failed to send frame data: {:?}", e);
                                }
                            }
                            return Err(e);
                        }*/

                        match stream1.try_write(&pkt) {
                            
                            Ok(0) => {
                                // If 0 bytes are written, the connection was likely closed.
                                eprintln!("Connection closed by peer");

                                //Add peer to disconnected_peers
                                let mut disconnected_peers = disc_peers.lock().await;
                                disconnected_peers.push(stream1.peer_addr().unwrap());
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
                            Err(ref e) if e.kind() == io::ErrorKind::BrokenPipe 
                                || e.kind() == io::ErrorKind::ConnectionReset
                                || e.kind() == io::ErrorKind::ConnectionAborted => {
                                // Connection was closed by the peer
                                eprintln!("Connection closed: {:?}", e);

                                //Add peer to disconnected_peers
                                let mut disconnected_peers = disc_peers.lock().await;
                                disconnected_peers.push(stream1.peer_addr().unwrap());
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
                /*if stream1.write_all(&encoded_frame1).await.is_err(){
                    eprintln!("Error sending frame to receiver");
                    //disconnected_peers.push(stream.peer_addr().unwrap());

                }*/ //

                /*let mut seq_num: u16 = 0;

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
                    
                    seq_num += 1;
                }*/
            }); 
        }
        
        // Remove disconnected peers
        let disconnected_peers = disconnected_peers.lock().await;

        if !disconnected_peers.is_empty() {
            let mut receivers = self.receivers.write().await;
            
            receivers.retain(|peer, _| {
                let keep_peer = !disconnected_peers.contains(peer);  //check if actual peer is in disconnected_peers
                if !keep_peer {
                    println!("Receiver {} disconnected", peer);
                }
                keep_peer   //condition to keep or remove the peer in the receivers list
            });
        }
        
        /*for peer in disconnected_peers.iter() {
            receivers.remove(peer);  //the Drop trait for TcpStream will close the connection
            println!("Receiver {} disconnected", peer);
        }*/
        Ok(())
    }

    // Send end of stream messageto all receivers
    pub async fn end_stream(&self) {
        //let recv = self.receivers.clone();
        let receivers = self.receivers.clone();
        let receivers = receivers.read().await;

        for (peer, stream) in receivers.iter() {
            let stream1 = stream.clone();
            let peer1 = *peer;
            
            tokio::spawn( async move {
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