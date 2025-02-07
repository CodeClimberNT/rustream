use crate::common::CaptureArea;
use crate::config::Config;
use crate::frame_grabber::{CapturedFrame, FrameGrabber};
use crate::video_recorder::VideoRecorder;
use crate::data_streaming::{Sender, Receiver, start_streaming, start_receiving, PORT};
use tokio::sync::oneshot::{channel, error::TryRecvError};
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use std::collections::VecDeque;

use std::collections::HashMap;
use std::rc;
use std::sync::{Arc, Mutex, mpsc};

use eframe::egui;
use egui::{
    CentralPanel, Color32, ColorImage, ComboBox, Context, FontId, Pos2, Rect, RichText,
    TextureHandle, TopBottomPanel, Window, Stroke, Ui, 
};

use log::debug;

const NUM_TEXTURES: usize = 3;

#[derive(Default)]
pub struct RustreamApp {
    config: Arc<Mutex<Config>>, // Wrap in Mutex for interior mutability
    frame_grabber: FrameGrabber,
    video_recorder: VideoRecorder,
    page: PageView,                           // Enum to track modes
    display_texture: Option<TextureHandle>,   // Texture for the screen capture
    textures: HashMap<String, TextureHandle>, // List of textures
    cropped_frame: Option<CapturedFrame>,     // Cropped image to send
    address_text: String,  
    caster_addr: Option<SocketAddr>,                   // Text input for the receiver mode
    //preview_active: bool,
    streaming_active: bool,
    recv_started: bool,
    is_selecting: bool,
    drag_start: Option<Pos2>,
    capture_area: Option<CaptureArea>, // Changed from tuple to CaptureArea
    new_capture_area: Option<Rect>,
    show_config: bool, // Add this field
    sender: Option<Arc<Sender>>,
    receiver: Option<Arc<tokio::sync::Mutex<Receiver>>>,
    sender_rx: Option<tokio::sync::oneshot::Receiver<Arc<Sender>>>,
    receiver_rx: Option<tokio::sync::oneshot::Receiver<Receiver>>,
    socket_created: bool,
    frame_rx: Option<tokio::sync::oneshot::Receiver<CapturedFrame>>,
}

#[derive(Default, Debug)]
pub enum PageView {
    #[default]
    HomePage,
    Caster,
    Receiver,
}

impl RustreamApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx: &Context = &cc.egui_ctx;
        egui_extras::install_image_loaders(ctx);

        let mut textures = HashMap::with_capacity(NUM_TEXTURES);
        RustreamApp::add_texture_to_map(
            &mut textures,
            ctx,
            "error",
            include_bytes!("../assets/icons/error.svg"),
            None,
        );

        RustreamApp::add_texture_to_map(
            &mut textures,
            ctx,
            "home_icon",
            include_bytes!("../assets/icons/home.svg"),
            None,
        );

        RustreamApp::add_texture_to_map(
            &mut textures,
            ctx,
            "quit_icon",
            include_bytes!("../assets/icons/quit.svg"),
            None,
        );

        assert_eq!(
            textures.len(),
            NUM_TEXTURES,
            r"Numbers of Textures Declared: {} | Actual number of textures: {}, 
            Check: 
                1. If the `NUM_TEXTURES` is correct
                2. If the texture name is unique
                3. Try again and pray to the Rust gods",
            NUM_TEXTURES,
            textures.len()
        );

        let config = Arc::new(Mutex::new(Config::default()));
        let frame_grabber = FrameGrabber::new(config.clone());
        let video_recorder = VideoRecorder::new(config.clone());

        RustreamApp {
            config,
            frame_grabber,
            video_recorder,
            textures,
            sender: None,
            sender_rx: None,
            streaming_active: false,
            socket_created: false,
            frame_rx: None,
            recv_started: false,
            receiver: None,
            receiver_rx: None,
            ..Default::default()
        }
    }

    fn get_preview_screen_rect(&self, ui: &egui::Ui) -> Rect {
        // Adjust this based on how your preview is laid out
        // For example, occupy the full available space
        ui.available_rect_before_wrap()
    }

    fn reset_ui(&mut self) {
        // Reset the application when rertuning to the home page
        self.frame_grabber.reset_capture();
        self.page = PageView::default();
        self.address_text.clear();
        self.sender = None;
        self.socket_created = false;
        self.streaming_active = false;
        self.frame_rx = None;
    }

    fn set_page(&mut self, page: PageView) {
        self.page = page;
    }

    fn home_page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);

                if ui.button("CAST NEW STREAMING").clicked() {
                    self.set_page(PageView::Caster);
                }

                ui.add_space(30.0);

                if ui.button("VIEW STREAMING").clicked() {
                    self.set_page(PageView::Receiver);
                }
            });
        });
    }

    fn render_config_window(&mut self, ctx: &Context) {
        let mut show_config = self.show_config;

        Window::new("Configuration")
            .open(&mut show_config)
            .resizable(false)
            .movable(true)
            .frame(
                egui::Frame::window(&ctx.style())
                    .outer_margin(0.0)
                    .inner_margin(10.0),
            )
            .show(ctx, |ui| {
                let mut config = self.config.lock().unwrap().clone();

                // Manual capture area input
                ui.heading("Capture Area");
                ui.horizontal(|ui| {
                    let mut area = self.capture_area.unwrap_or(CaptureArea::new(0, 0, 0, 0));

                    ui.vertical(|ui| {
                        // TODO: fix 0 value error
                        ui.label("X:");
                        let mut x_str = area.x.to_string();
                        if ui.text_edit_singleline(&mut x_str).changed() {
                            if let Ok(x) = x_str.parse() {
                                area.x = x;
                                self.capture_area = Some(area);
                            }
                        }

                        ui.label("Y:");
                        let mut y_str = area.y.to_string();
                        if ui.text_edit_singleline(&mut y_str).changed() {
                            if let Ok(y) = y_str.parse() {
                                area.y = y;
                                self.capture_area = Some(area);
                            }
                        }
                    });

                    ui.vertical(|ui| {
                        ui.label("Width:");
                        let mut width_str = area.width.to_string();
                        if ui.text_edit_singleline(&mut width_str).changed() {
                            if let Ok(width) = width_str.parse() {
                                area.width = width;
                                self.capture_area = Some(area);
                            }
                        }

                        ui.label("Height:");
                        let mut height_str = area.height.to_string();
                        if ui.text_edit_singleline(&mut height_str).changed() {
                            if let Ok(height) = height_str.parse() {
                                area.height = height;
                                self.capture_area = Some(area);
                            }
                        }
                    });
                });

                // Update config when capture area changes
                if let Some(area) = self.capture_area {
                    config.capture.capture_area = Some(area);
                }

                ui.heading("Streaming Settings");
                ui.separator();

                // TODO: Select capture area
                self.is_selecting ^= ui.button("Select Capture Area").clicked();

                if self.is_selecting {
                    // TODO: Select capture area
                    // display a rectangle to show the selected area
                    let response = ui.allocate_rect(ctx.available_rect(), egui::Sense::drag());

                    // display a rectangle to show the selected area
                    if response.drag_started() {
                        self.drag_start = Some(response.interact_pointer_pos().unwrap());
                    }

                    if let Some(start) = self.drag_start {
                        if let Some(current) = response.interact_pointer_pos() {
                            self.new_capture_area = Some(egui::Rect::from_two_pos(start, current));
                            // Draw the selection rectangle
                            if let Some(rect) = self.new_capture_area {
                                ui.painter().rect_filled(
                                    rect,
                                    0.0,
                                    egui::Color32::from_rgba_premultiplied(0, 255, 0, 100),
                                );
                                ui.painter().rect_stroke(
                                    rect,
                                    0.0,
                                    egui::Stroke::new(2.0, egui::Color32::GREEN),
                                );
                            }
                        }
                    }

                    // OK button to confirm selection
                    if self.new_capture_area.is_some() && ui.button("OK").clicked() {
                        let rect = self.new_capture_area.unwrap();
                        self.capture_area = Some(CaptureArea::new(
                            rect.min.x as u32,
                            rect.min.y as u32,
                            rect.width() as u32,
                            rect.height() as u32,
                        ));
                        log::debug!(
                            "Capture Area: x:{}, y:{}, width:{}, height:{}",
                            self.capture_area.unwrap().x,
                            self.capture_area.unwrap().y,
                            self.capture_area.unwrap().width,
                            self.capture_area.unwrap().height
                        );
                        self.is_selecting = false;
                        self.drag_start = None;
                        self.new_capture_area = None;
                    }
                    // Cancel selection
                    if ui.button("Cancel").clicked() {
                        self.is_selecting = false;
                        self.new_capture_area = None;
                        self.drag_start = None;
                    }
                }

                // Update capture area in config when it changes
                if let Some(area) = self.capture_area {
                    let mut config = self.config.lock().unwrap();
                    config.capture.capture_area =
                        Some(CaptureArea::new(area.x, area.y, area.width, area.height));
                }

                ui.heading("Recording Settings");
                ui.separator();

                // Monitor selection
                let selected_monitor = &mut config.capture.selected_monitor;
                ComboBox::from_label("Monitor")
                    .selected_text(format!("Monitor {}", selected_monitor))
                    .show_ui(ui, |ui| {
                        self.frame_grabber
                            .get_monitors()
                            .iter()
                            .enumerate()
                            .for_each(|(i, m)| {
                                ui.selectable_value(selected_monitor, i, m);
                            });
                    });

                // Output path configuration
                ui.horizontal(|ui| {
                    ui.label("Output path:");
                    let mut recording_path =
                        config.video.output_path.to_string_lossy().into_owned();
                    ui.text_edit_singleline(&mut recording_path)
                        .on_hover_text(recording_path.clone());
                    if ui.button("📂").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title("Save recording as...")
                            .set_file_name("recording.mp4")
                            .add_filter("MP4 video", &["mp4"])
                            .save_file()
                        {
                            // let mut config = self.config.lock().unwrap();
                            config.video.output_path = path;
                        }
                    }
                });

                // FPS settings
                ui.horizontal(|ui| {
                    ui.label("Target FPS:");
                    // let mut config = self.config.lock().unwrap();
                    ComboBox::from_label("")
                        .selected_text(format!("{} FPS", config.video.fps))
                        .show_ui(ui, |ui| {
                            for &fps in &[24, 25, 30, 50, 60] {
                                ui.selectable_value(
                                    &mut config.video.fps,
                                    fps,
                                    format!("{} FPS", fps),
                                );
                            }
                        });
                });

                // Audio settings
                ui.heading("Audio Settings");
                ui.separator();

                // let mut config = self.config.lock().unwrap();
                ui.checkbox(&mut config.audio.enabled, "Enable Audio");
                if config.audio.enabled {
                    ui.add(
                        egui::Slider::new(&mut config.audio.volume, 0.0..=1.0)
                            .text("Volume")
                            .step_by(0.1),
                    );
                }

                // Apply changes if the config has changed
                let has_config_changed: bool = self.config.lock().unwrap().clone() != config;
                if has_config_changed {
                    log::debug!("Config changed: {:?}", config);
                    self.config.lock().unwrap().update(config);
                    self.frame_grabber.reset_capture();
                }
            });
        self.show_config = show_config;
    }

    fn render_caster_page(&mut self, ui: &mut egui::Ui, ctx: &Context, _frame: &mut eframe::Frame) {
        // show the selected monitor as continuous feedback of frames
        ui.heading("Monitor Feedback");
        ui.separator();
        ui.vertical_centered(|ui| {
            ui.horizontal(|ui| {
                //rivedi, cambiare pulsante in start streaming, la preview è automatica
                if ui.button(if self.streaming_active {
                    "Stop Streaming"
                } else {
                    "Start Streaming"
                }).clicked() {
                    self.streaming_active = !self.streaming_active; // Toggle streaming
                    if !self.streaming_active {
                        //se stoppo e riprendo lo streaming, mi dà errore: AddrInUse, message: "Di norma è consentito un solo utilizzo di ogni indirizzo di socket (protocollo/indirizzo di rete/porta)
                        self.sender = None; // Clear sender when stopping streaming
                        self.socket_created = false;
                    }
                }

                if ui.button("⚙ Settings").clicked() {
                    self.show_config = true;
                }

                if self.video_recorder.is_recording() {
                    if ui.button("⏹ Stop Recording").clicked() && self.video_recorder.stop() {
                        debug!("Recording stopped and saved successfully");
                    }
                } else if ui.button("⏺ Start Recording").clicked() {
                    self.video_recorder.start();
                }
            });
        });

        // Render the config window if it's open
        self.render_config_window(ctx);

        ui.vertical_centered(|ui| {
            
            if let Some(screen_image) = self.frame_grabber.capture_frame() {

                let screen_clone = screen_image.clone(); // Clone the screen image for streaming

                let image: ColorImage = if let Some(area) = self.capture_area {
                    // Apply cropping if we have a capture area
                    if let Some(cropped) =
                        screen_image
                            .clone()
                            .view(area.x, area.y, area.width, area.height)
                    {
                        egui::ColorImage::from_rgba_unmultiplied(
                            [cropped.width as usize, cropped.height as usize],
                            &cropped.rgba_data, // Changed from frame_data to rgba_data
                        )
                    } else {
                        // Fallback to full image if crop parameters are invalid
                        egui::ColorImage::from_rgba_unmultiplied(
                            [screen_image.width as usize, screen_image.height as usize],
                            &screen_image.rgba_data, // Changed from frame_data to rgba_data
                        )
                    }
                } else {
                    // No crop area selected, show full image
                    egui::ColorImage::from_rgba_unmultiplied(
                        [screen_image.width as usize, screen_image.height as usize],
                        &screen_image.rgba_data, // Changed from frame_data to rgba_data
                    )
                };

                // Store the active frame for network transmission if needed
                self.cropped_frame = if let Some(area) = self.capture_area {
                    screen_image.view(area.x, area.y, area.width, area.height)
                } else {
                    Some(screen_image)
                };

                // Update texture
                if let Some(ref mut texture) = self.display_texture {
                    texture.set(image, egui::TextureOptions::default());
                } else {
                    self.display_texture = Some(ctx.load_texture(
                        "display_texture",
                        image,
                        egui::TextureOptions::default(),
                    ));
                }
                
            
                if self.streaming_active {
                    // Initialize sender if it doesn't exist
                    /*if s.is_none() {
                        let runtime = tokio::runtime::Runtime::new().unwrap();
                        runtime.block_on(async {
                            let sender = Arc::new(Sender::new().await);
                            self.sender = Some(sender);
                        });
                    }*/
                    
                    // Initialize sender if it doesn't exist
                    if self.sender.is_none() && !self.socket_created {
                        let  (tx, rx) = channel();
                        //let tx_clone = tx.clone();
                        self.socket_created = true;

                        tokio::spawn(async move {
                            let sender = Sender::new().await;
                            let _ = tx.send(Arc::new(sender));
                        });
                        //store rx to poll it later to see if initialization completed, since the channel sender is async
                        self.sender_rx = Some(rx);   
                    }

                    // Check if we have a pending sender initialization
                    if let Some(mut rx) = self.sender_rx.take() {  //take consumes the sender_rx
                        // Try to receive the sender
                        if let Ok(sender) = rx.try_recv() {
                            self.sender = Some(sender);
                        }
                        else {
                            // Put the receiver back if we haven't received yet
                            self.sender_rx = Some(rx);
                        }
                    } 

                    // Send frame if we have a sender
                    if let Some(sender) = &self.sender { //i redo the check to extract the sender from Option<Sender>
                        let sender_clone = sender.clone();
                        tokio::spawn(async move {
                            if let Err(e) = start_streaming(sender_clone, screen_clone).await {
                                eprintln!("Error sending frame: {}", e);
                            }
                        });
                    }
                }
            }

            let texture = self
                    .display_texture
                    .as_ref()
                    .unwrap_or(self.textures.get("error").unwrap());
                ui.add(egui::Image::new(texture).max_size(self.get_preview_screen_rect(ui).size()));

            // ui.horizontal(|ui| {
            //     ui.label("Output path:");
            //     let mut recording_path = self
            //         .config
            //         .lock()
            //         .unwrap()
            //         .video
            //         .output_path
            //         .to_string_lossy()
            //         .into_owned();
            //     // ? Beware: if the path use unicode characters, it may not be displayed correctly
            //     ui.text_edit_singleline(&mut recording_path)
            //         // Show a tooltip with the full path when hovering over the text field (useful if it's too long)
            //         .on_hover_text(recording_path);
            //     if ui.button("📂 Browse").clicked() {
            //         if let Some(path) = rfd::FileDialog::new()
            //             .set_title("Save recording as...")
            //             .set_file_name("recording.mp4")
            //             .add_filter("MP4 video", &["mp4"])
            //             .save_file()
            //         {
            //             let mut config = self.config.lock().unwrap();
            //             config.video.output_path = path;
            //         }
            //     }
            // });

            // Record frame and audio if we're recording
            if self.video_recorder.is_recording() {
                if let Some((frame, audio)) = self.frame_grabber.capture_frame_with_audio() {
                    if let Some(frame) = frame {
                        self.video_recorder.record_frame(&frame);
                    }
                    self.video_recorder.record_audio(&audio);
                }
            }
        });
    }

    pub fn render_receiver_mode(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        //ui.disable();
        ui.heading("Receiver Mode");
        ui.vertical_centered(|ui| {
            ui.label("Enter the Sender's IP Address");
            ui.add_space(10.0);

            ui.text_edit_singleline(&mut self.address_text);
            ui.add_space(20.0);

            if ui.add_enabled(!self.address_text.trim().is_empty(), egui::Button::new("Connect")).clicked(){

                //check if inserted address is valid
                if let Ok(addr) = self.address_text.parse::<Ipv4Addr>() {
                    let caster_addr = SocketAddr::new(IpAddr::V4(addr), PORT);
                    self.caster_addr = Some(caster_addr); 
                    
                    
                    //let  (tx, rx) = mpsc::sync_channel(1);
                      
                    
                    // Initialize receiver if it doesn't exist
                    if self.receiver.is_none()  {  //&& !self.socket_created
                        let  (tx, rx) = channel();
                        
                        //self.socket_created = true; //socket is created correctly only after the execution of async code, should i do this there??
                        
                        tokio::spawn(async move {
                            let receiver = Receiver::new(caster_addr).await;

                            let _ = tx.send(receiver);
                        });

                        //store rx to poll it later to see if initialization completed, since the channel sender is async
                        self.receiver_rx = Some(rx); 
                        
                    }

                    //let tx_clone = tx.clone();
                    //tokio::spawn(async move {
                        /*
                        if !self.socket_created {
                        }
                        let socket = connect_to_sender(caster_addr).await;
                        match socket {
                            Ok(socket) => {
                                
                                println!("Connected to Sender");

                                match recv_data(socket).await {
                                    Ok(frame) => {
                                        print!("Frame received from receiver");
                                        let _ = tx_clone.send(frame);
                                        drop(tx_clone);
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to receive data: {}", e);
                                    }
                                }
                                /*if let Err(e) = recv_data(caster_addr, socket).await {
                                    eprintln!("Failed to receive data: {}", e);
                                }*/
                            }
                            Err(e) => {
                                println!("No data received");
                                println!("{}", e);
                            }
                        }*/
                    //});

                    /*match rx.try_recv() { //eseguita anche se la socket non si è connessa
                        Ok(frame) => {
                            println!("Frame received from channel");
                            
                            let image: ColorImage =  egui::ColorImage::from_rgba_unmultiplied(
                                [frame.width as usize, frame.height as usize],
                                &frame.rgba_data );

                            // Update texture
                            if let Some(ref mut texture) = self.display_texture {
                                texture.set(image, egui::TextureOptions::default());
                            } else {
                                self.display_texture = Some(ctx.load_texture(
                                    "display_texture",
                                    image,
                                    egui::TextureOptions::default(),
                                ));
                            }

                            let texture = self
                                .display_texture
                                .as_ref()
                                .unwrap_or(self.textures.get("error").unwrap());
                            ui.add(egui::Image::new(texture).max_size(self.get_preview_screen_rect(ui).size()));
                            
                        },
                        Err(_) => println!("the sender dropped or no data received"),
                    }*/
                }
                else {
                    ui.label(RichText::new("Invalid IP Address").color(Color32::RED));
                    //come faccio a farla comparire per più tempo?? scompare in un secondo
                }

                /*// Add a loading indicator while waiting for receiver initialization
                if self.socket_created && self.receiver.is_none() {
                    ui.spinner(); // Show a spinner while connecting
                    ui.label("Connecting to sender...");
                }*/

                

                
                //check if inserted address is valid
                /*if let Ok(addr) = self.address_text.parse::<Ipv4Addr>() {
                    let caster_addr = SocketAddr::new(IpAddr::V4(addr), PORT);    
                    
                    let (tx, rx) = channel();
                    self.frame_rx = Some(rx);

                    //let mut addr_vec: Vec<&str> = self.address_text.split(".").collect();
                    ///let port  = addr_vec[3].split(":").collect::<Vec<&str>>()[1];
                    ///addr_vec[3] = addr_vec[3].split(":").collect::<Vec<&str>>()[0];

                    /*let caster_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(
                        addr_vec[0].parse::<u8>().unwrap(), 
                        addr_vec[1].parse::<u8>().unwrap(), 
                        addr_vec[2].parse::<u8>().unwrap(), 
                        addr_vec[3].parse::<u8>().unwrap())), 
                        PORT);  //port.parse::<u16>().unwrap()*/
                    
                    //let  (tx, mut rx) = channel();
                    
                    tokio::spawn(async move {
                        let socket = connect_to_sender(caster_addr).await;
                        match socket {
                            Ok(socket) => {
                                println!("Connected to Sender");

                                match recv_data(socket).await {
                                    Ok(frame) => {
                                        println!("Received data");
                                        //let _ = tx.send(frame);
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to receive data: {}", e);
                                    }
                                }
                                /*if let Err(e) = recv_data(caster_addr, socket).await {
                                    eprintln!("Failed to receive data: {}", e);
                                }*/
                            }
                            Err(_) => {
                                println!("No data received");
                            }
                        }
                    });
                } 
                else {
                    ui.label(RichText::new("Invalid IP Address").color(Color32::RED));
                }*/    
            }

            // Check if we have a pending receiver initialization
            if let Some(mut rx) = self.receiver_rx.take() {  //take consumes the receiver_rx
                
                // Try to receive the receiver
                match rx.try_recv() {
                    Ok(receiver) => {
                        println!("Receiver initialized successfully");
                        self.receiver = Some(Arc::new(tokio::sync::Mutex::new(receiver)));
                        
                    }
                    Err(TryRecvError::Empty) => {
                        // Put the channel receiver back if we haven't received yet
                        self.receiver_rx = Some(rx);
                    }
                    Err(TryRecvError::Closed) => {
                        ui.label(RichText::new("Failed to receive receiver from channel").color(Color32::RED));
                        self.socket_created = false;
                        self.receiver_rx = None;
                    }
                }
            }

            // Start/continue receiving if we have a receiver
            if let Some(receiver) = &mut self.receiver { //i redo the check to extract the sender from Option<Sender>
                let receiver_clone = receiver.clone();
                let rcv_clone = receiver.clone();

                let  (tx, mut rx) = channel(); //oneshot
                self.frame_rx = Some(rx);
                

                if let Some(caster_addr) = self.caster_addr {
                
                    tokio::spawn(async move {
                        
                        let receiver_clone2 = Arc::clone(&receiver_clone);
                        let mut receiver = receiver_clone.lock().await; // ✅ Use `.await` instead of `.unwrap()`
                        let mut frame;
                        if receiver.caster != caster_addr {
                            receiver.caster = caster_addr;
                            drop(receiver);
                            println!("Caster address changed, reconnecting to new sender");
                            //connect the socket to the new address and start receiving
                            frame = start_receiving(receiver_clone2, false).await;
                        } else {
                            //drop the lock before starting the receiving task
                            drop(receiver);
                            //println!("Receiving from the same sender");
                            frame = start_receiving(receiver_clone2, true).await;
                        }
                        println!("After start_receiving completed");
                        if let Some(frame) = frame {
                            println!("Frame received from start_receiving");
                            match tx.send(frame) {
                                Ok(_) => println!("🔍 Frame successfully sent to channel"),
                                Err(_) => println!("❌ Failed to send frame to channel"),
                            }
                        }
                        /*let rcv = rcv_clone.lock().await;
                        println!("last Lock on receiver acquired");
                        let mut frame_vec = rcv.frames.lock().await; 
                        println!("Lock on frames acquired");
                        println!("Frame queue length: {}", frame_vec.len());
                        if let Some(frame) = frame_vec.pop_front(){ // retrieve the oldest frame first  
                            println!("Frame popped from queue");
                            drop(frame_vec);
                            //let _ = tx_clone.send(frame);
                        }*/
                    }); 

                    while let Some(rx) = &mut self.frame_rx {
                    
                        if let Ok(frame) = rx.try_recv() {
                                self.frame_rx = None;
                                println!("Frame received from channel");
                                let image = egui::ColorImage::from_rgba_unmultiplied(
                                    [frame.width as usize, frame.height as usize], 
                                    &frame.rgba_data
                                );
                                println!("image created");
                                    
                                    
                                // Update texture
                                if let Some(ref mut texture) = self.display_texture {
                                    texture.set(image, egui::TextureOptions::default());
                                    println!("texture updated");
                                } else {
                                    self.display_texture = Some(ctx.load_texture(
                                        "display_texture",
                                        image,
                                        egui::TextureOptions::default(),
                                    ));
                                    println!("texture loaded");
                                }
                            
                        }                    
                    }
                } 

                
                /*tokio::spawn(async move {
                    let rcv = rcv_clone.lock().await;
                    let mut frame_vec = rcv.frames.lock().await; 
                    if let Some(frame) = frame_vec.pop_front(){ // retrieve the oldest frame first  
                        println!("Frame popped from queue {:?}", frame);
                        //let _ = tx_clone.send(frame);
                    }
                });  */

                
            }
            
            

            let texture = self
                .display_texture
                .as_ref()
                .unwrap_or(self.textures.get("error").unwrap());
            ui.add(egui::Image::new(texture).max_size(self.get_preview_screen_rect(ui).size()));
                        
        });
    }

    /// Add a texture to the texture map
    /// If the texture fails to load, an error texture is loaded instead
    /// The error texture is a red square
    /// The error texture is loaded only once and is reused for all errors
    /// # Arguments
    /// * `textures` - A mutable reference to the texture map
    /// * `ctx` - A reference to the egui context
    /// * `name` - The name of the texture
    /// * `img_bytes` - The bytes of the image to load
    /// * `texture_options` - Optional texture options
    ///
    /// # Panics
    /// If the error texture is not found
    ///
    /// # Example
    /// ```rust
    /// let mut textures = HashMap::new();
    /// let ctx = egui::Context::new();
    /// let img_bytes = include_bytes!("../assets/icons/home.svg");
    /// add_texture_to_map(&mut textures, &ctx, "home_icon", img_bytes, None);
    /// ```
    fn add_texture_to_map(
        textures: &mut HashMap<String, TextureHandle>,
        ctx: &Context,
        name: &str,
        img_bytes: &[u8],
        texture_options: Option<egui::TextureOptions>,
    ) {
        let image: ColorImage = match egui_extras::image::load_svg_bytes(img_bytes) {
            Ok(img) => img,
            Err(e) => {
                log::warn!("Failed to load image: {}", e);
                if let Some(error_texture) = textures.get("error") {
                    textures.insert(name.to_string(), error_texture.clone());
                    return;
                } else {
                    log::warn!("Error Texture not found. Loading RED SQUARE as error texture");
                    ColorImage::new([50, 50], egui::Color32::RED)
                }
            }
        };

        let texture = ctx.load_texture(name, image, texture_options.unwrap_or_default());
        textures.insert(name.to_string(), texture.clone());
    }
}

impl eframe::App for RustreamApp {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    // Home button
                    if ui
                        .add_sized(
                            [80., 30.],
                            egui::Button::image_and_text(
                                &self.textures.get("home_icon").unwrap().clone(),
                                "🏠 Home",
                            ),
                        )
                        .clicked()
                    {
                        self.reset_ui();
                    }

                    // Quit button
                    if ui
                        .add_sized(
                            [80., 30.],
                            egui::Button::image_and_text(
                                &self.textures.get("quit_icon").unwrap().clone(),
                                "🚪 Quit",
                            ),
                        )
                        .clicked()
                    {
                        std::process::exit(0);
                    }
                });

                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("Welcome to RUSTREAM!")
                            .font(FontId::proportional(40.0))
                            .color(Color32::GOLD),
                    );
                });
            });
        });
        CentralPanel::default().show(ctx, |ui| match self.page {
            PageView::HomePage => self.home_page(ui),

            PageView::Caster => self.render_caster_page(ui, ctx, frame),

            PageView::Receiver => self.render_receiver_mode(ui, ctx),
        });
    }
}
