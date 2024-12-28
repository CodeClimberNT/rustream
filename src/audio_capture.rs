// // src/audio_capture.rs
// use crate::config::AudioConfig;
// use std::sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex};
// use cpal::{
//     traits::{DeviceTrait, HostTrait, StreamTrait},
//     Stream, StreamConfig,
// };

// pub struct AudioCapturer {
//     config: Arc<Mutex<AudioConfig>>,
//     is_capturing: Arc<AtomicBool>,
//     stream: Option<Stream>,
//     callback: Box<dyn Fn(&[f32]) + Send + 'static>,
// }

// impl AudioCapturer {
//     pub fn new(
//         config: Arc<Mutex<AudioConfig>>, 
//         callback: impl Fn(&[f32]) + Send + 'static
//     ) -> Self {
//         Self {
//             config,
//             is_capturing: Arc::new(AtomicBool::new(false)),
//             stream: None,
//             callback: Box::new(callback),
//         }
//     }

//     pub fn start(&mut self) -> Result<(), String> {
//         if self.is_capturing.load(Ordering::SeqCst) {
//             return Ok(());
//         }

//         let host =