use std::collections::VecDeque;
use std::env;
use std::io::ErrorKind::WouldBlock;
use std::io::Write;
use std::mem;
use std::net::SocketAddr;
use std::process::{Command, Stdio};
use std::str::from_utf8;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, Mutex, Notify};

use crate::screen_capture::CapturedFrame;

pub const PORT: u16 = 56123;
const MAX_DATAGRAM_SIZE: usize = 65507; //1472
const SEQ_NUM_SIZE: usize = size_of::<u16>(); // Size of sequence number, 2
const FRAME_ID_SIZE: usize = size_of::<u32>(); // Size of frame_id, 4

pub struct Sender {
    socket: Arc<UdpSocket>,
    receivers: Arc<Mutex<Vec<SocketAddr>>>,
    frame_id: Arc<Mutex<u32>>,
    started_sending: bool,
    //serve pure il frame da mandare? o il buffer di frames?
}

impl Sender {
    //initialize caster UdpSocket
    //implementare il fatto che l'ack da mandare dopo aver ricevuto richiesta dal client deve contenere la porta su cui il client deve connettersi (ma dovrebbe già saperla per fare richiesta teoricamente)
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
            frame_id: Arc::new(Mutex::new(0)),
            started_sending: false,
        }
    }

    pub async fn listen_for_receivers(&self) {
        
        let receivers = self.receivers.clone();
        let socket = self.socket.clone();

        tokio::spawn(async move {
            
            loop {
                let mut buf = [0; 1472];
                match socket.recv_from(&mut buf).await {
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
                    },
                    Err(e) => eprintln!("Error receiving connection: {}", e),
                }
            }
        });
    }

    pub async fn send_data(
        &self,
        frame: CapturedFrame,
        frame_id: Arc<Mutex<u32>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let receivers = self.receivers.lock().await;
        if receivers.is_empty() {
            println!("No receivers connected");
            return Ok(()); // Return early if no receivers
        }
        let start = Instant::now();
        let encoded_frame = frame.encode_to_h265()?;
        let encode_time = start.elapsed();
        println!("Encoding time: {:?}", encode_time);
        println!("Frame encoded to h265");

        //loop {

        let mut fid = frame_id.lock().await;
        *fid += 1;
        println!("Frame id: {:?}", *fid);
        let mut seq_num: u16 = 0;
        //encoded_frame size = num elements (len()) * size of element (u8)[1 byte]
        let total_chunks = (encoded_frame.len() as f32
            / (MAX_DATAGRAM_SIZE - 2 * SEQ_NUM_SIZE - FRAME_ID_SIZE) as f32)
            .ceil() as u16;
        println!("Total chunks: {:?}", total_chunks);

        for chunk in encoded_frame.chunks(MAX_DATAGRAM_SIZE - 2 * SEQ_NUM_SIZE - FRAME_ID_SIZE) {
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
                //tokio::time::sleep(Duration::from_micros(100)).await; //sleep for 100 microseconds before sending next chunk
            }
            seq_num += 1;
        }
        drop(encoded_frame);
        Ok(())
    }
}

pub async fn start_streaming(sender: Arc<Mutex<Sender>>, frame: CapturedFrame) -> Result<(), Box<dyn std::error::Error>> {
    // Start listening for new receivers in the background
    let sender_clone = sender.clone();
    //let sender_clone1 = sender.clone();
    let mut sender = sender.lock().await;
    if !sender.started_sending {
        sender.started_sending = true;
        //drop(sender);
        //tokio::spawn(async move {
            //let sender = sender_clone.lock().await;
            sender.listen_for_receivers().await;
            //drop(sender);
        //});       
    }
    //let sender1 = sender_clone.lock().await;
    //println!("Sender1 lock acquired");
    
    let frame_id = sender.frame_id.clone();

    return match sender.send_data(frame, frame_id).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    };
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

pub struct Receiver {
    pub socket: UdpSocket,  //Arc<UdpSocket>
    pub caster: SocketAddr, //Arc<Mutex<SocketAddr>>,
    //pub frames: Arc<Mutex<VecDeque<CapturedFrame>>>, //o va bene solo mutex?
    pub started_receiving: bool,
    //pub frame_rx: Option<mpsc::Receiver<CapturedFrame>>,
}

impl Receiver {
    //create a new receiver, its socket and connect to the caster
    pub async fn new(caster: SocketAddr) -> Self {
        let sock = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        println!("Socket {} ", sock.local_addr().unwrap());
        let buf = "REQ_FRAMES".as_bytes();
        if let Ok(_) = sock.connect(caster).await {
            //connects socket to send/receive only from sender_addr
            match sock.try_send(buf) {
                //send datagram to caster to request the streaming
                Ok(_) => {
                    println!("Connected to sender");
                }
                Err(_) => println!("Failed to send registration request"), //come me lo gestisco questo errore?
            }
        }

        Self {
            socket: sock,   //Arc::new(sock),
            caster: caster, //Arc::new(Mutex::new(caster)),
            //frames: Arc::new(Mutex::new(VecDeque::new())),
            started_receiving: false,
            //frame_rx: None,
        }
    }

    //send datagram to caster to request the streaming
    pub async fn connect_to_sender(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        //let socket = UdpSocket::bind("0.0.0.0:0").await?;
        let buf = "REQ_FRAME".as_bytes();
        self.socket.connect(self.caster).await?; //connects socket to send/receive only from sender_addr
        match self.socket.try_send(buf) {
            Ok(_) => {
                println!("Connected to sender");
                Ok(())
            }
            Err(_) => Err("Failed to connect to sender")?,
        }
    }

    pub async fn recv_data(
        &mut self,
        tx: mpsc::Sender<Vec<u8>>,
        stop_notify: Arc<Notify>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        //let mut buf =  vec![0; MAX_DATAGRAM_SIZE]; //[0; 1024]; //aggiustare dimesione buffer, troppo piccola per datagramma
        let mut frame_chunks: Vec<(u16, Vec<u8>)> = Vec::new();
        let mut frame: Vec<u8> = Vec::new();
        let mut fid: u32 = 1;
        let mut received_chunks = std::collections::HashSet::new();

        //while self.started_receiving { //implementare in app pulsante stop che mette started_receiving a false
        loop {
            //self.socket.readable().await?;
            let mut buf = vec![0; MAX_DATAGRAM_SIZE];

            tokio::select! {
                _ = stop_notify.notified() => {
                    println!("Received stop signal, exiting recv_data");
                    break; // Gracefully exit when `notify_waiters()` is called
                }

                res = self.socket.recv_from(&mut buf) => { // Keep listening for UDP packets
                    match res {
            //match self.socket.recv_from(&mut buf).await {
                        Ok((len, _)) => {

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
                                    //println!("Frame sent to process_frame");


                                    //let frames = frames_vec.clone();
                                    /*tokio::spawn(async move{
                                        process_frame(enc_frames, frames).await;
                                    });*/
                                    //return Ok(frame);
                                }

                                // Clear frame_chunks and received_chunks for the next frame
                                frame_chunks.clear();
                                received_chunks.clear();
                                frame.clear();
                            }
                        },
                        Err(ref e) if e.kind() == WouldBlock => {
                            continue;
                        },
                        Err(e) => println!("Error in receiving data {:?}", e), //dà WouldBlock, non trova dati da leggere
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

async fn process_frame(frames_vec: Arc<std::sync::Mutex<VecDeque<CapturedFrame>>>, frame: Vec<u8>) {  // mut rx: mpsc::Receiver<Vec<u8>>
        
   // while let Some(frame) = rx.recv().await { //while self.started_receiving
        //let mut frames = enc_frames.lock().await;
        //if let Some(frame) = frames.pop_front() {
            //println!("Frame received in process_frame");
            //drop(frames);
            let start = Instant::now();
            let decoded_frame = decode_from_h265_to_rgba(frame);
            let decode_time = start.elapsed();
            println!("Decoding time: {:?}", decode_time);
            match decoded_frame {
                Ok(frame) => {
                    let mut frames = frames_vec.lock().unwrap();
                    frames.push_back(frame);
                    //println!("Frame pushed to frames_vec");
                },
                Err(e) =>  {
                    eprintln!("Error decoding frame: {}", e);
                }
            };

    //tokio::time::sleep(Duration::from_millis(5)).await;
    //}
    //}
    /*let decoded_frame = decode_from_h265_to_rgba(frame);
    //return decoded_frame;
    match decoded_frame {
        Ok(frame) => {
            println!("Frame received in process_frame");
            let mut frames = frames_vec.lock().unwrap();
            frames.push_back(frame);
            println!("Frame pushed to frames_vec");

            /*if tx.is_closed() {
                eprintln!("❌ Error: Channel is closed, cannot send frame");
                return;
            }
            if let Err(e) = tx.send(frame).await {
                eprintln!("❌ Error sending decoded frame to start_receiving: {}", e);
            } */
        },
        Err(e) =>  {
            eprintln!("Error decoding frame: {}", e);
        }
     };*/
}

pub async fn start_receiving(
    frames_vec: Arc<std::sync::Mutex<VecDeque<CapturedFrame>>>,
    receiver: Arc<Mutex<Receiver>>,
    stop_notify: Arc<Notify>,
) {
    //println!("Inside start_receiving");

    let mut recv = receiver.lock().await;
    let stop_notify1 = stop_notify.clone();
     //println!("Receiver lock acquired");

    //if receiver changed caster address
    /*if !connected {
        println!("Connecting to the new sender");
        if let Err(e) = recv.connect_to_sender().await{
            println!("Error connecting to sender: {}", e);

        };
    }*/

    /*if channel_rx.is_none() { //if rx has not been initialized before
        let (tx, mut rx) = mpsc::channel::<CapturedFrame>(100);
        let tx_clone = tx.clone();
        recv.frame_rx = Some(rx);
        recv.frame_tx = Some(tx);
        //channel_rx = Some(rx);
        drop(recv); //release the lock
    }

    let recv_clone = receiver.clone();
    //wait to receive the decoded frame
    let handle = tokio::spawn(async move {
        println!("Waiting for processed frame");
        let mut recv = recv_clone.lock().await;
        if let Some(ref mut rx) = recv.frame_rx {
             println!("Receiver lock acquired, waiting for processed frame");
            if let Some(frame) = rx.recv().await { //release the lock
                println!("Received processed frame");
                drop(recv);
                Some(frame)
            } else {
                println!("Error receiving frame from processing (channel closed)");
                drop(recv);
                None
            }

        } else {
            drop(recv);
            println!("No rx");
            None
        }

    });*/

    //let recv_clone = receiver.clone();
    if !recv.started_receiving {
        recv.started_receiving = true;
        drop(recv);

        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(100);

        let recv_clone = receiver.clone();
        tokio::spawn(async move {
            //scope to release the lock
            let mut recv = recv_clone.lock().await;

            //let mut ready = false;
            //let rc = receiver.clone();

            //launch recv_data thread only when starting receiving a stream (vedere come implementare la cosa, i thread danno problemi per async)
            //if !recv.started_receiving {
                //recv.started_receiving = true;
                //println!("inside is !started receiving");
                
                //drop(recv); //release the lock
       
                //tokio::spawn(async move { //let handle =
                    //println!("inside tokio spawn");
                    //let mut recv = rc.lock().await;
    
                   
                    println!("Calling recv_data");
                    //if let Some(tx) = recv.frame_tx.clone() {
                        if let Err(e) = recv.recv_data(tx, stop_notify1).await{
                            println!("Error receiving frame: {}", e);
                        }
                        //if recv_data never ends recv is never dropped?
                        drop(recv);
                    
                    /*match recv.recv_data().await{ //recv data deve diventare un thread?
                        Ok(frame) => {
                            //println!("Frame received from recv_data");                    
                            return Some(frame);
                        },
                        Err(e) => {
                            println!("Error receiving frame: {}", e);
                            return None;
                        }
                    }*/
                //});
                    //}
            //}
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
}

fn decode_from_h265_to_rgba(
    frame: Vec<u8>,
) -> Result<CapturedFrame, Box<dyn std::error::Error + Send + Sync>> {
    //println!("Dimension of encoded frame: {}", frame.len());

    let platform = env::consts::OS; //detect OS

    let (gpu_acceleration, decoder) = match platform {
        "linux" =>
        // On Linux, prefer VAAPI (works with Intel/AMD)
        {
            (["-hwaccel", "vaapi"], ["-c:v", "hevc_vaapi"])
        }

        "windows" => 
         // On Windows, use CUDA/NVENC (for NVIDIA GPUs)
         (["-hwaccel", "auto"], ["-c:v", "hevc_cuda"]),

        "macos" =>
        // On macOS, you might rely on software decoding or choose available hardware (e.g., use VideoToolbox)
        {
            (["-hwaccel", "videotoolbox"], ["-c:v", "hevc_videotoolbox"])
        }

        _ => (["", ""], ["-c:v", "hevc"]),
    };

    let mut ffmpeg = Command::new("ffmpeg")
        .args([
            //"-y",  // Overwrite output files
            //gpu_acceleration[0], gpu_acceleration[1],
            //decoder[0], decoder[1],
            "-f",
            "hevc", // input format is H.265
            "-i",
            "pipe:0", // input from stdin
            "-c:v",
            "rawvideo",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "rgba", // convert to rgba
            "-f",
            "rawvideo", // output raw
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
        //stdin.flush()?;  // Ensure all data is written
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
    //println!("Frame decoded to rgba");

    let rgba_data = output.stdout;
    //println!("Decoded RGBA size: {}", rgba_data.len());

    let (width, height) = get_h265_dimensions(frame.clone())?;

    Ok(CapturedFrame::from_rgba_vec(
        rgba_data,
        width as usize,
        height as usize,
    ))
}

fn get_h265_dimensions(
    frame: Vec<u8>,
) -> Result<(u32, u32), Box<dyn std::error::Error + Send + Sync>> {
    let mut ffmpeg = Command::new("ffmpeg")
        .args([
            "-f", "hevc",
            "-i", "pipe:0",
            "-vframes", "1", // Process only first frame
            "-vf", "scale=iw:ih", // Force scale filter to report size
            "-f", "null",
            "-",
        ])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped()) // FFmpeg reports dimensions to stderr
        .spawn()?;

    // Write frame data
    ffmpeg.stdin.take().unwrap().write_all(&frame)?;

    let output = ffmpeg.wait_with_output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Parse dimensions from FFmpeg output
    let dim_pattern = regex::Regex::new(r"(\d+)x(\d+)")?
        .captures(&stderr)
        .ok_or("Could not find dimensions in FFmpeg output")?;

    let width = dim_pattern[1].parse()?;
    let height = dim_pattern[2].parse()?;

    println!("Dimensions: {}x{}", width, height);
    Ok((width, height))
}
