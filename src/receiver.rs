use std::collections::VecDeque;
use std::io::Write;
use std::process::{Command, Stdio};
use std::str::from_utf8;
use tokio::sync::{mpsc, Mutex, Notify};
use std::sync::atomic::{AtomicBool, Ordering};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt};

use crate::screen_capture::CapturedFrame;

pub struct Receiver {
    pub socket: TcpStream,
    pub started_receiving: bool
}

impl Receiver {
    //create a new receiver, its socket and connect to the caster
    pub async fn new(caster: SocketAddr) -> Result<Self, std::io::Error> {
        
        match TcpStream::connect(caster).await {
            Ok(stream) => {
                println!("Connected to sender at {}", caster);

                Ok(Self {
                    socket: stream,
                    started_receiving: false
                })
            }
            Err(e) => {
                eprintln!("Failed to connect to sender: {}", e);
                Err(e)
            }
        }       
    }

    pub async fn recv_data(&mut self, tx: mpsc::Sender<Vec<u8>>, stop_notify: Arc<Notify>, stream_ended: Arc<AtomicBool> ) -> Result<(), std::io::Error> {

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
        },
        Err(e) =>  {
            eprintln!("Error decoding frame: {}", e);
        }
    };
}

pub async fn start_receiving(
    frames_vec: Arc<std::sync::Mutex<VecDeque<CapturedFrame>>>,
    receiver: Arc<Mutex<Receiver>>,
    stop_notify: Arc<Notify>,
    host_unreachable: Arc<AtomicBool>,
    stream_ended: Arc<AtomicBool>
) {
 
    let stop_notify1 = stop_notify.clone();    
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(100);
        
    tokio::spawn(async move {        
        let mut recv = receiver.lock().await;        
        println!("Calling recv_data");
        
        if let Err(_) = recv.recv_data(tx, stop_notify1, stream_ended).await{
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
                            1920 as usize,
                            1080 as usize,
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

fn decode_from_h265_to_rgba(
    frame: Vec<u8>,
) -> Result<CapturedFrame, Box<dyn std::error::Error + Send + Sync>> {
    //println!("Dimension of encoded frame: {}", frame.len());

    let mut command = Command::new("ffmpeg");

    // Platform-specific configuration to hide window
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut ffmpeg = command
        .args([
            "-f", "hevc", // input format is H.265
            "-i", "pipe:0", // input from stdin
            "-c:v", "rawvideo",
            "-preset", "ultrafast",
            "-pix_fmt", "rgba", // convert to rgba
            "-f", "rawvideo", // output raw
            "pipe:1", // output to stdout
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
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

    let rgba_data = output.stdout;

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
    let mut command = Command::new("ffmpeg");

    // Platform-specific configuration to hide window
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut ffmpeg = command
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

    //println!("Dimensions: {}x{}", width, height);
    Ok((width, height))
}