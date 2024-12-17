use std::{net::SocketAddr, thread};
use tokio::net::UdpSocket;

use crate::frame_grabber::{CapturedFrame};

pub struct Sender {
    pub socket: UdpSocket,
    pub receivers: Vec<SocketAddr>,
    //serve pure il frame da mandare? o il buffer di frames?
}

impl Sender {
    //initialize caster UdpSocket 
    pub async fn new() -> Self {
        let sock = UdpSocket::bind("0.0.0.0:8081").await;
        let sock = match sock {
            Ok(socket) => socket,
            Err(_) => {
                println!("Failed to bind socket  to port 8081, binding to default port 0");
                let default_sock = UdpSocket::bind("0.0.0.0:0").await.unwrap();
                println!("Socket bound to port {}",  default_sock.local_addr().unwrap().port()); //.to_string()?
                default_sock
            }
            //sock = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        };
        Self { 
            socket: sock, 
            receivers: Vec::new() 
        }
    }

    pub async fn listen_for_receivers(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = [0; 1024];
        loop {
            let (len, peer_addr) = self.socket.recv_from(&mut buf).await?;
            self.receivers.push(peer_addr);
        }
    }

    pub async fn send_data(&self, frame: CapturedFrame) -> Result<(), Box<dyn std::error::Error>> {
    
        //let socket = self.socket;
       
        let encoded_frame= frame.encode_to_h264();
        
        //loop {
            
        if self.receivers.len() != 0 {
            if let Ok(ref frame) = encoded_frame {
                for chunk in frame.chunks(1024) {
                    for peer in self.receivers.iter() {
                        match  self.socket.try_send_to(chunk, *peer) { 
                            Ok(_) => (),
                            Err(_) => Err("Failed to send data")?,
                            
                        }
                    }
                }
                Ok(())
            }  
            else {
                Err("Error encoding frame to H264")?
            }  
        }
        else {
            Err("No receivers connected")?
        }  
        //}
        
    }
}

pub async fn cast_streaming(frame: CapturedFrame) -> Result<(), Box<dyn std::error::Error>> {
    
    let mut sender = Sender::new().await;
    sender.listen_for_receivers().await?;
    sender.send_data(frame).await?;
    Ok(())
    
}

//send datagram to caster to request the streaming
pub async fn connect_to_sender(sender_addr: SocketAddr) -> Result<UdpSocket, Box<dyn std::error::Error>> {
    
    let socket = UdpSocket::bind("0.0.0.0:8081").await?;
    let buf = "REQ_FRAME".as_bytes();
    socket.connect(sender_addr).await?; //connects socket only to send/receive from sender_Addr
    match socket.try_send(buf) {
        Ok(_) => Ok(socket), 
        Err(_) => Err("Failed to connect to sender")?,
    }
}

pub async fn recv_data(sender_addr: SocketAddr, socket: UdpSocket) -> Result<(), Box<dyn std::error::Error>>{
    
    let mut buf = [0; 1024];
    match socket.try_recv_from(&mut buf) {
        Ok(_) => Ok(()) , //reconstruct chunks and decode from h264
        Err(_) => Err("Failed to receive data")?,
    }
    //vedere se posso fare semplicemete connect e poi verifico nel sendere se la socket è connessa
    

}

