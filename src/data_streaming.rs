use std::net::SocketAddr;
use tokio::net::UdpSocket;
use std::io::ErrorKind::WouldBlock;
use std::sync::Arc;
use crate::frame_grabber::{CapturedFrame};
use tokio::sync::Mutex;
use std::mem;
use std::io::Write;
use std::process::{Command, Stdio};

pub const PORT: u16 = 56123;
const MAX_DATAGRAM_SIZE: usize = 65507; //1472
const SEQ_NUM_SIZE: usize = std::mem::size_of::<u8>(); // Size of sequence number

//#[derive(Clone)]      
pub struct Sender {
    socket: Arc<UdpSocket>,
    receivers: Arc<Mutex<Vec<SocketAddr>>>,
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
            receivers: Arc::new(Mutex::new(Vec::new())) 
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

    pub async fn send_data(&self, frame: CapturedFrame) -> Result<(), Box<dyn std::error::Error>> {
    
        let encoded_frame= frame.encode_to_h264()?;
        println!("Frame encoded to h264");
        let receivers = self.receivers.lock().await;
        
        if receivers.is_empty() {
            println!("No receivers connected");
            return Ok(());  // Return early if no receivers
        }
        //loop {
        //const frame_size = std::mem::size_of::<encoded_frame>();
        //let num_pkts_per_frame = ( as f32 / (MAX_DATAGRAM_SIZE - SEQ_NUM_SIZE) as f32).ceil();
        
        let mut seq_num: u16 = 0;   
        //267
        //total_chunks = 412 with MAX_DATAGRAM_SIZE = 1024, 273with MAX_DATAGRAM_SIZE = 1472
        for chunk in encoded_frame.chunks(MAX_DATAGRAM_SIZE - SEQ_NUM_SIZE) {
      
            let mut pkt = Vec::new();
            pkt.push(seq_num as u8); //&seq_num.to_ne_bytes()
            pkt.extend_from_slice(chunk);

            for &peer in receivers.iter() {
                if let Err(e) = self.socket.send_to(&pkt, peer).await {
                    eprintln!("Error sending to {}: {}", peer, e);
                }
                println!("Sent chunk {:?} to peer {}", seq_num, peer);
            }
            seq_num += 1;
        }
        Ok(())
        
    }
}

pub async fn start_streaming(sender: Arc<Sender>, frame: CapturedFrame) -> Result<(), Box<dyn std::error::Error>> {
    // Start listening for new receivers in the background
    sender.listen_for_receivers().await;
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

pub async fn recv_data(sender_addr: SocketAddr, socket: UdpSocket) -> Result<(), Box<dyn std::error::Error>>{
    
    
    //let mut buf =  vec![0; MAX_DATAGRAM_SIZE]; //[0; 1024]; //aggiustare dimesione buffer, troppo piccola per datagramma
    let mut frame_chunks: Vec<(u8, Vec<u8>)> = Vec::new();

    loop {

        socket.readable().await?;
        let mut buf =  vec![0; MAX_DATAGRAM_SIZE];

        match socket.try_recv_from(&mut buf) {
            Ok((len, _)) => {

                let seq_num = buf[0];
                let chunk_data = buf[1..len].to_vec();
                frame_chunks.push((seq_num, chunk_data));

                //riordinare chunks e decodificare da h264
                println!("Received chunk {:?} from sender:", seq_num);
   
            }, //reconstruct chunks and decode from h264
            Err(ref e) if e.kind() == WouldBlock => {
                continue;
            },
            Err(e) => println!("Error in receiving data {:?}", e), //dà WouldBlock, non trova dati da leggere
        }
    }
}

fn decode_from_h264_to_rgba(frame: Vec<u8>) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>{
    
    let (width, height) = get_h264_dimensions(frame.clone())?;

    let mut ffmpeg = Command::new("ffmpeg")
            .args([
                "-f", "h264",           // input format is H.
                "-i", "-", // input from stdin
                "-preset", "ultrafast",
                "-f", "rawvideo", // output raw
                "-pixel_format", "rgba", // convert to rgba
                "-video_size", &format!("{}x{}", width, height), 
                "-", // output to stdout
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null()) // Ignora errori di ffmpeg per semplicità
            .spawn()?;

        // write encoded frame in stdin
        ffmpeg.stdin.as_mut().unwrap().write_all(&frame)?;

        // read H.264 encoded data from stdout
        let output = ffmpeg.wait_with_output()?;
        if !output.status.success() {
            return Err("FFmpeg encoding failed".into());
        }

        Ok(output.stdout)
    
}

fn get_h264_dimensions(frame: Vec<u8>) -> Result<(u32, u32), Box<dyn std::error::Error + Send + Sync>> {
    // Run ffprobe to extract the width and height
    let mut ffprobe = Command::new("ffprobe")
        .args([
            "-i", "-", // input from stdin
            "-v", "error", // Suppress unnecessary output
            "-select_streams", "v:0", // Select the first video stream
            "-show_entries", "stream=width,height", // Show width and height
            "-of", "csv=p=0", // Format output as CSV (plain text)
            "-", // output to stdout
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null()) // Ignora errori di ffmpeg per semplicità
        .spawn()?;
        //.output()?; // Execute the command and capture the output

    /*if !output.status.success() {
        return Err(format!("ffprobe failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }*/

    // write encoded frame in stdin
    ffprobe.stdin.as_mut().unwrap().write_all(&frame)?;

    // read H.264 encoded data from stdout
    let output = ffprobe.wait_with_output()?;
    if !output.status.success() {
        return Err("FFmpeg encoding failed".into());
    }

    // Parse the width and height from the output
    let output_str = String::from_utf8(output.stdout)?;
    let dims: Vec<&str> = output_str.trim().split(',').collect();
    if dims.len() != 2 {
        return Err("Unexpected output format from ffprobe".into());
    }

    let width: u32 = dims[0].parse()?;
    let height: u32 = dims[1].parse()?;

    Ok((width, height))
}

