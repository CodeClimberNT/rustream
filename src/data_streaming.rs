use std::{net::SocketAddr};
use tokio::net::UdpSocket;
use std::sync::{Arc};
use crate::frame_grabber::{CapturedFrame};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Sender {
    socket: Arc<UdpSocket>,
    receivers: Arc<Mutex<Vec<SocketAddr>>>,
    //serve pure il frame da mandare? o il buffer di frames?
}

impl Sender {
    //initialize caster UdpSocket 
    //implementare il fatto che l'ack da mandare dopo aver ricevuto richiesta dal client deve contenere la porta su cui il client deve connettersi (ma dovrebbe già saperla per fare richiesta teoricamente)
    pub async fn new() -> Self {
        let sock = UdpSocket::bind("0.0.0.0:0").await.unwrap(); 
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
            receivers: Arc::new(Mutex::new(Vec::new())) 
        }
    }

    pub async fn listen_for_receivers(&self) {
        let mut buf = [0; 1024];
        let receivers = self.receivers.clone();
        let socket = self.socket.clone();
        
        tokio::spawn(async move {
            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((_, peer_addr)) => {
                        let mut receivers = receivers.lock().await;
                        if !receivers.contains(&peer_addr) {
                            receivers.push(peer_addr);
                            println!("New receiver connected: {}", peer_addr);
                        }
                    }
                    Err(e) => eprintln!("Error receiving connection: {}", e),
                }
            }
        });
    }

    pub async fn send_data(&self, frame: CapturedFrame) -> Result<(), Box<dyn std::error::Error>> {
    
        let encoded_frame= frame.encode_to_h264()?;
        let receivers = self.receivers.lock().await;
        
        if receivers.is_empty() {
            return Ok(());  // Return early if no receivers
        }
        //loop {
            
        for chunk in encoded_frame.chunks(1024) {
            for &peer in receivers.iter() {
                if let Err(e) = self.socket.send_to(chunk, peer).await {
                    eprintln!("Error sending to {}: {}", peer, e);
                }
            }
        }
        Ok(())
        
    }
}

pub async fn start_streaming(sender: Arc<Sender>, frame: CapturedFrame) -> Result<(), Box<dyn std::error::Error>> {
    // Start listening for new receivers in the background
    sender.listen_for_receivers().await;
    sender.send_data(frame).await
}

pub async fn send_frame(sender: &Sender, frame: CapturedFrame) -> Result<(), Box<dyn std::error::Error>> {
    sender.send_data(frame).await
}

/*pub async fn cast_streaming(sender: &mut Sender, frame: CapturedFrame) -> Result<(), Box<dyn std::error::Error>> {
    //let mut sender_guard = sender.lock().expect("failed to lock sender");
    //let mut sender_guard = sender.lock().await;
    
        
            sender.listen_for_receivers().await;
            sender.send_data(frame).await?;
            Ok(())
        
    //sender_guard.listen_for_receivers().await?;
    //sender_guard.send_data(frame).await?;
    //Ok(())

    /*if let Some(ref mut s) = *sender_guard {
        s.listen_for_receivers().await?;
        s.send_data(frame).await?;
        Ok(())
    } else {
        Err("No sender available".into())
    }  */
    
}*/

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

