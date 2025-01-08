use std::net::SocketAddr;
use tokio::net::UdpSocket;
use std::io::ErrorKind::WouldBlock;
use std::sync::Arc;
use crate::frame_grabber::{CapturedFrame};
use tokio::sync::Mutex;
use std::mem;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

pub const PORT: u16 = 56123;
const MAX_DATAGRAM_SIZE: usize = 65507; //1472
const SEQ_NUM_SIZE: usize = size_of::<u16>(); // Size of sequence number, 2
const FRAME_ID_SIZE: usize = size_of::<u32>(); // Size of frame_id, 4

//#[derive(Clone)]      
pub struct Sender {
    socket: Arc<UdpSocket>,
    receivers: Arc<Mutex<Vec<SocketAddr>>>,
    frame_id: Arc<Mutex<u32>>,
    //serve pure il frame da mandare? o il buffer di frames?
}

impl Sender {
    //initialize caster UdpSocket 
    //implementare il fatto che l'ack da mandare dopo aver ricevuto richiesta dal client deve contenere la porta su cui il client deve connettersi (ma dovrebbe già saperla per fare richiesta teoricamente)
    pub async fn new() -> Self {
        let addr = format!("0.0.0.0:{}", PORT);
        let sock = UdpSocket::bind(addr).await.unwrap(); 
        println!("Socket {} ",  sock.local_addr().unwrap());
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
            frame_id: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn listen_for_receivers(&self) {
        let mut buf = [0; 1472];
        let receivers = self.receivers.clone();
        let socket = self.socket.clone();
        
        tokio::spawn(async move {
            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((_, peer_addr)) => {
                        println!("Received connection request from: {}", &peer_addr);
                        let mut receivers = receivers.lock().await;
                        if !receivers.contains(&peer_addr) {
                            receivers.push(peer_addr);
                            
                        }
                        println!("New receiver connected: {}", peer_addr);
                    }
                    Err(e) => eprintln!("Error receiving connection: {}", e),
                }
            }
        });
    }

    pub async fn send_data(&self, frame: CapturedFrame, frame_id: Arc<Mutex<u32>>) -> Result<(), Box<dyn std::error::Error>> {
    
        let encoded_frame= frame.encode_to_h265()?;
        println!("Frame encoded to h265");
        let receivers = self.receivers.lock().await;
        
        if receivers.is_empty() {
            println!("No receivers connected");
            return Ok(());  // Return early if no receivers
        }
        //loop {

        let mut fid = frame_id.lock().await;
        *fid += 1;
        println!("Frame id modified in: {:?}", *fid);
        let mut seq_num: u16 = 0;   
        //encoded_frame size = num elements (len()) * size of element (u8)[1 byte] 
        let total_chunks = (encoded_frame.len() as f32 / (MAX_DATAGRAM_SIZE - 2*SEQ_NUM_SIZE - FRAME_ID_SIZE) as f32).ceil() as u16;
        println!("Total chunks: {:?}", total_chunks);
        
        for chunk in encoded_frame.chunks(MAX_DATAGRAM_SIZE - 2*SEQ_NUM_SIZE - FRAME_ID_SIZE) {
      
            let mut pkt = Vec::new();
            pkt.extend_from_slice(&seq_num.to_ne_bytes()); //&seq_num.to_ne_bytes()
            pkt.extend_from_slice(&total_chunks.to_ne_bytes());
            pkt.extend_from_slice(&fid.to_ne_bytes());
            pkt.extend_from_slice(chunk);

            for &peer in receivers.iter() {
                if let Err(e) = self.socket.send_to(&pkt, peer).await {
                    eprintln!("Error sending to {}: {}", peer, e);
                }
                println!("Sent chunk {:?} to peer {}", seq_num, peer);
                tokio::time::sleep(Duration::from_micros(100)).await;
            }
            seq_num += 1;
        }
        Ok(())
        
    }
}

pub async fn start_streaming(sender: Arc<Sender>, frame: CapturedFrame) -> Result<(), Box<dyn std::error::Error>> {
    // Start listening for new receivers in the background
    sender.listen_for_receivers().await;
    let frame_id = sender.frame_id.clone();
    
    sender.send_data(frame, frame_id).await
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

/*pub struct Receiver {
    socket: Arc<UdpSocket>,
    caster: Arc<Mutex<SocketAddr>>,
}

impl Receiver {

}*/

//send datagram to caster to request the streaming
pub async fn connect_to_sender(sender_addr: SocketAddr) -> Result<UdpSocket, Box<dyn std::error::Error + Send + Sync>> {
    
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    let buf = "REQ_FRAME".as_bytes();
    socket.connect(sender_addr).await?; //connects socket to send/receive only from sender_addr
    match socket.try_send(buf) {
        Ok(_) => Ok(socket), 
        Err(_) => Err("Failed to connect to sender")?,
    }
}

pub async fn recv_data(sender_addr: SocketAddr, socket: UdpSocket) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>{
    
    
    //let mut buf =  vec![0; MAX_DATAGRAM_SIZE]; //[0; 1024]; //aggiustare dimesione buffer, troppo piccola per datagramma
    let mut frame_chunks: Vec<(u16, Vec<u8>)> = Vec::new();
    let mut frame: Vec<u8> = Vec::new();
    let mut fid:u32 = 1;
    let mut received_chunks = std::collections::HashSet::new();

    loop {

        socket.readable().await?;
        let mut buf =  vec![0; MAX_DATAGRAM_SIZE];

        match socket.try_recv_from(&mut buf) {
            Ok((len, _)) => {
                
                let frame_id = u32::from_ne_bytes(buf[4..8].try_into().unwrap());
                println!("Received Frame {:?}", frame_id);
                let seq_num = u16::from_ne_bytes(buf[0..2].try_into().unwrap());
                println!("Received chunk {:?} from sender:", seq_num);

                let total_chunks = u16::from_ne_bytes(buf[2..4].try_into().unwrap());

                //Discard previous frame if a new frame_id arrives before the previous last chunk
                if frame_id != fid { 
                    frame_chunks.clear();
                    received_chunks.clear();
                    println!("Wrong frame id: {:?}, previous frame discarded", frame_id);
                    fid = frame_id;
                    
                }
                
                let chunk_data = buf[8..len].to_vec();
                received_chunks.insert(seq_num);
                frame_chunks.push((seq_num, chunk_data));
                //println!("Frame_chunks len: {:?}", frame_chunks.len());
                
                if seq_num == total_chunks - 1 {
                    //Received all chunks of frame, sort chunks and decode frame
                    if received_chunks.len() == total_chunks as usize {
                        frame_chunks.sort_by(|a, b| a.0.cmp(&b.0));
                        println!("Frame_chunks sorted, index order: {:?}", frame_chunks.iter().map(|(i, _)| i).collect::<Vec<_>>());

                        for (_, ref chunk) in &frame_chunks {
                            frame.append(&mut chunk.clone());
                        }

                        let decoded_frame = decode_from_h265_to_rgba(frame.clone());
                        match decoded_frame {
                            Ok(frame) => Ok(frame),
                            Err(e) =>  {
                                eprintln!("Error decoding frame: {}", e);
                                Err(e)
                            }
                        }?;
                    }
                   

                // Clear frame_chunks and received_chunks for the next frame
                frame_chunks.clear();
                received_chunks.clear();    
                //decodificare da h265
                }  
            }, 
            Err(ref e) if e.kind() == WouldBlock => {
                continue;
            },
            Err(e) => println!("Error in receiving data {:?}", e), //dà WouldBlock, non trova dati da leggere
        }
    }
}

fn decode_from_h265_to_rgba(frame: Vec<u8>) -> Result<CapturedFrame, Box<dyn std::error::Error + Send + Sync>>{
    
    

    let mut ffmpeg = Command::new("ffmpeg")
            .args([
                "-f", "hevc",   // input format is H.265
                "-i", "pipe:0", // input from stdin
                "-c:v", "rawvideo",
                "-preset", "ultrafast",
                "-pix_fmt", "rgba", // convert to rgba
                "-f", "rawvideo", // output raw
                //"-video_size", &format!("{}x{}", width, height), 
                "pipe:1", // output to stdout
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()) // null() Ignora errori di ffmpeg per semplicità
            .spawn()?;

        // write encoded frame in stdin
        if let Some(stdin) = ffmpeg.stdin.as_mut() {
            stdin.write_all(&frame)?;
        } else {
            return Err("Failed to open stdin for ffmpeg".into());
        }

        // read H.264 encoded data from stdout
        let output = ffmpeg.wait_with_output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("FFmpeg error: {}", stderr);
            return Err(format!("FFmpeg encoding failed: {}", stderr).into());
        }
        println!("Frame decoded to rgba");
        
        let rgba_data = output.stdout; 
        
        let (width, height) = get_h265_dimensions(frame.clone())?;
        
        Ok(CapturedFrame {
            width,
            height,
            rgba_data,
        })
    
}

fn get_h265_dimensions(frame: Vec<u8>) -> Result<(u32, u32), Box<dyn std::error::Error + Send + Sync>> {
    let mut ffmpeg = Command::new("ffmpeg")
        .args([
            "-f", "hevc",
            "-i", "pipe:0",
            "-vframes", "1",  // Process only first frame
            "-vf", "scale=iw:ih",  // Force scale filter to report size
            "-f", "null",
            "-"
        ])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())  // FFmpeg reports dimensions to stderr
        .spawn()?;
    
    // Write frame data
    ffmpeg.stdin.take().unwrap().write_all(&frame)?;
    
    let output = ffmpeg.wait_with_output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Parse dimensions from FFmpeg output
    let dim_pattern = regex::Regex::new(r"(\d+)x(\d+)")?.captures(&stderr)
        .ok_or("Could not find dimensions in FFmpeg output")?;
    
    let width = dim_pattern[1].parse()?;
    let height = dim_pattern[2].parse()?;
    
    println!("Dimensions: {}x{}", width, height);
    Ok((width, height))

}

