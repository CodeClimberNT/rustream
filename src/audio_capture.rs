use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc;

pub struct AudioCapture {
    stream: Option<cpal::Stream>,
    sender: mpsc::Sender<Vec<f32>>,
}

impl AudioCapture {
    pub fn new() -> (Self, mpsc::Receiver<Vec<f32>>) {
        let (sender, receiver) = mpsc::channel();

        (
            AudioCapture {
                stream: None,
                sender,
            },
            receiver,
        )
    }

    pub fn start(&mut self) -> Result<(), String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device available")?;

        let config = device.default_input_config().map_err(|e| e.to_string())?;

        let sender = self.sender.clone();

        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _: &_| {
                    sender.send(data.to_vec()).unwrap_or_default();
                },
                |err| eprintln!("Audio stream error: {}", err),
                None,
            )
            .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;
        self.stream = Some(stream);

        Ok(())
    }
}
