mod capturer;
mod frame;

pub use capturer::{CaptureError, ScreenCapture};
pub use frame::CapturedFrame;

// Common types used by both modules
pub(crate) use crate::common::{BgraBuffer, CaptureArea, RgbaBuffer};
