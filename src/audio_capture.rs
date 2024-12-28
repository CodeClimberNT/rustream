use crate::config::Config;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Stream, StreamConfig,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

// pub struct AudioStream {
//     buffer: RingBuffer<f32>,
//     encoder: AudioEncoder,
//     network_sender: NetworkSender,
// }

pub struct AudioCapturer {
    // config: Arc<Mutex<Config>>,
    is_capturing: Arc<AtomicBool>,
    stream: Option<Stream>,
    audio_buffer: Vec<f32>,
    thread_buffer: Arc<Mutex<Vec<f32>>>,
}

impl AudioCapturer {
    pub fn new(/*config: Arc<Mutex<Config>>*/) -> Self {
        Self {
            // config,
            is_capturing: Arc::new(AtomicBool::new(false)),
            stream: None,
            audio_buffer: Vec::new(),
            thread_buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn take_audio_buffer(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.audio_buffer)
    }

    pub fn start(&mut self) -> Result<(), String> {
        if self.is_capturing.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.audio_buffer.clear();
        if let Ok(mut buffer) = self.thread_buffer.lock() {
            buffer.clear();
        }

        let host = cpal::default_host();
        let device = host.default_input_device().ok_or("No input device found")?;

        // Get supported config
        let supported_config = device.default_input_config().map_err(|e| e.to_string())?;
        let config: StreamConfig = supported_config.into();

        let is_capturing = self.is_capturing.clone();

        let buffer_clone = self.thread_buffer.clone();

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &_| {
                    if is_capturing.load(Ordering::SeqCst) {
                        if let Ok(mut buffer) = buffer_clone.lock() {
                            buffer.extend_from_slice(data);
                        }
                    }
                },
                move |err| log::error!("Audio stream error: {}", err),
                None,
            )
            .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;
        self.stream = Some(stream);
        self.is_capturing.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(stream) = self.stream.take() {
            drop(stream);
        }
        self.is_capturing.store(false, Ordering::SeqCst);

        // Transfer data from thread buffer to main buffer
        if let Ok(mut buffer) = self.thread_buffer.lock() {
            self.audio_buffer.append(&mut buffer);
            buffer.clear();
        }
    }
}
