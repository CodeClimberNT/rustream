// use image::{ImageBuffer, RgbaImage};
use image::{GenericImageView, ImageBuffer, RgbaImage};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::common::RgbaBuffer;

#[derive(Debug, Default, Clone)]
pub struct CapturedFrame {
    pub width: usize,
    pub height: usize,
    pub rgba_data: Vec<u8>,
}

impl CapturedFrame {
    /*fn bgra_to_rgba(buffer_bgra: BgraBuffer, width: usize, height: usize) -> RgbaBuffer {
        // Calculate the stride (bytes per row)
        let stride = buffer_bgra.len() / height;

        // Preallocate the entire vector to avoid reallocations
        let mut rgba: Vec<u8> = vec![0u8; width * height * 4];

        // Process the buffer and write directly to `rgba_data`
        for y in 0..height {
            let row_start = y * stride;
            for x in 0..width {
                let i = row_start + x * 4;
                let target = (y * width + x) * 4;
                rgba[target] = buffer_bgra[i + 2]; // B to R
                rgba[target + 1] = buffer_bgra[i + 1]; // G remains the same
                rgba[target + 2] = buffer_bgra[i]; // R to B
                rgba[target + 3] = 255; // Alpha
            }
        }
        rgba
    }*/

    /*fn bgra_to_rgba_with_crop(
        buffer_bgra: BgraBuffer,
        buffer_width: usize,
        buffer_height: usize,
        crop_area: CaptureArea,
    ) -> (RgbaBuffer, usize, usize) {
        // Debug input values
        debug!("Initial buffer: {}x{}", buffer_width, buffer_height);
        debug!(
            "Requested crop: x={}, y={}, w={}, h={}",
            crop_area.x, crop_area.y, crop_area.width, crop_area.height
        );

        // Ensure crop coordinates are within bounds
        let CaptureArea {
            x,
            y,
            width,
            height,
        } = CaptureArea::new_with_safeguards(
            crop_area.x,
            crop_area.y,
            crop_area.width,
            crop_area.height,
            buffer_width,
            buffer_height,
        );

        debug!("Adjusted crop: x={}, y={}, w={}, h={}", x, y, width, height);

        // Allocate output buffer
        let mut rgba_data = vec![0u8; width * height * 4];
        let src_stride = buffer_width * 4;
        let dst_stride = width * 4;

        // Copy and convert pixels
        for row in 0..height {
            let src_row = y + row;
            for col in 0..width {
                let src_col = x + col;

                let src_idx = (src_row * src_stride) + (src_col * 4);
                let dst_idx = (row * dst_stride) + (col * 4);

                if src_idx + 3 < buffer_bgra.len() && dst_idx + 3 < rgba_data.len() {
                    rgba_data[dst_idx] = buffer_bgra[src_idx + 2]; // R
                    rgba_data[dst_idx + 1] = buffer_bgra[src_idx + 1]; // G
                    rgba_data[dst_idx + 2] = buffer_bgra[src_idx]; // B
                    rgba_data[dst_idx + 3] = buffer_bgra[src_idx + 3]; // A
                }
            }
        }

        debug!("Output buffer: {}x{}", width, height);
        (rgba_data, width, height)
    }*/

    /*pub fn from_bgra_buffer(
        buffer_bgra: BgraBuffer,
        buffer_width: usize,
        buffer_height: usize,
        capture_area: Option<CaptureArea>,
    ) -> Self {
        // Default to full buffer if crop_area is None
        let (rgba_data, final_width, final_height) = match capture_area {
            Some(crop) => {
                Self::bgra_to_rgba_with_crop(buffer_bgra, buffer_width, buffer_height, crop)
            }
            None => {
                let rgba_image = Self::bgra_to_rgba(buffer_bgra, buffer_width, buffer_height);
                (rgba_image, buffer_width, buffer_height)
            }
        };

        Self {
            width: final_width,
            height: final_height,
            rgba_data,
        }
    }*/

    pub fn from_rgba_vec(
        buffer_rgba: RgbaBuffer,
        buffer_width: usize,
        buffer_height: usize,
    ) -> Self {
        Self {
            width: buffer_width,
            height: buffer_height,
            rgba_data: buffer_rgba,
        }
    }

    pub fn from_bgra(width: u32, height: u32, mut bgra_buffer: RgbaImage) -> Self {
        // Convert BGRA to RGBA immediately
        for chunk in bgra_buffer.chunks_exact_mut(4) {
            chunk.swap(0, 2); // Swap B and R
        }
        Self {
            width: width as usize,
            height: height as usize,
            rgba_data: bgra_buffer.to_vec(),
        }
    }

    pub fn view(self, x: u32, y: u32, view_width: u32, view_height: u32) -> Option<Self> {
        let image_view: RgbaImage =
            ImageBuffer::from_vec(self.width as u32, self.height as u32, self.rgba_data)
                .expect("Couldn't create image buffer from raw frame");

        let cropped_image: Vec<u8> = image_view
            .view(x, y, view_width, view_height)
            .to_image()
            .to_vec();

        Some(Self {
            width: view_width as usize,
            height: view_height as usize,
            rgba_data: cropped_image,
        })
    }

    pub fn encode_to_h265(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // let platform = env::consts::OS; //detect OS

        // let (gpu_acceleration, encoder) = match platform {
        //     "linux" =>
        //     // On Linux, prefer VAAPI (works with Intel/AMD)
        //     {
        //         (["-hwaccel", "vaapi"], ["-c:v", "hevc_vaapi"])
        //     }
        //     "windows" =>
        //     // On Windows, use CUDA/NVENC (for NVIDIA GPUs)
        //     {
        //         (["-hwaccel", "cuda"], ["-c:v", "hevc_cuda"])
        //     }
        //     "macos" =>
        //     // On macOS, you might rely on software decoding or choose available hardware (e.g., use VideoToolbox)
        //     {
        //         (["-hwaccel", "videotoolbox"], ["-c:v", "hevc_videotoolbox"])
        //     }
        //     _ => (["", ""], ["-c:v", "hevc"]),
        // };

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
                //gpu_acceleration[0], gpu_acceleration[1],
                "-f", "rawvideo", // input is raw video
                "-pixel_format", "rgba",
                "-video_size", &format!("{}x{}", self.width, self.height),
                "-i", "-", // input from stdin
                "-c:v", "hevc", // Codec H.265
                //encoder[0], encoder[1],
                "-preset", "ultrafast",
                "-f", "rawvideo", // output raw
                "-",        // output to stdout
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()) // Stdio::null() Ignora errori di ffmpeg per semplicità
            .spawn()?;

        // write RGBA data in stdin
        ffmpeg.stdin.as_mut().unwrap().write_all(&self.rgba_data)?;

        // read H.265 encoded data from stdout
        let output = ffmpeg.wait_with_output()?;
        if !output.status.success() {
            return Err("FFmpeg encoding failed".into());
        }

        Ok(output.stdout)
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), image::ImageError> {
        let image: RgbaImage = image::ImageBuffer::from_raw(
            self.width as u32,
            self.height as u32,
            self.rgba_data.clone(),
        )
        .expect("Failed to create image buffer");

        image.save(path)
    }
}
