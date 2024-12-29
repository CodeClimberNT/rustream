use super::{BgraBuffer, CaptureArea, RgbaBuffer};
use image::{ImageBuffer, RgbaImage};
use std::sync::Arc;

#[derive(Debug, Default, Clone)]
pub struct CapturedFrame {
    pub width: usize,
    pub height: usize,
    pub rgba_data: Arc<RgbaImage>,
}

impl CapturedFrame {
    fn bgra_to_rgba(buffer_bgra: BgraBuffer, width: usize, height: usize) -> RgbaBuffer {
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
    }

    fn bgra_to_rgba_with_crop(
        buffer_bgra: BgraBuffer,
        buffer_width: usize,
        buffer_height: usize,
        crop_area: CaptureArea,
    ) -> (RgbaBuffer, usize, usize) {
        // Debug input values
        log::debug!("Initial buffer: {}x{}", buffer_width, buffer_height);
        log::debug!(
            "Requested crop: x={}, y={}, w={}, h={}",
            crop_area.x,
            crop_area.y,
            crop_area.width,
            crop_area.height
        );

        // Ensure crop coordinates are within bounds
        let CaptureArea { x, y, width, height } = CaptureArea::new_with_safeguards(
            crop_area.x,
            crop_area.y,
            crop_area.width,
            crop_area.height,
            buffer_width,
            buffer_height,
        );
        

        log::debug!("Adjusted crop: x={}, y={}, w={}, h={}", x, y, width, height);

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

        log::debug!("Output buffer: {}x{}", width, height);
        (rgba_data, width, height)
    }

    pub fn from_bgra_buffer(
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

        // Create an ImageBuffer from the processed data
        let image: RgbaImage =
            ImageBuffer::from_vec(final_width as u32, final_height as u32, rgba_data)
                .expect("Failed to create image from buffer");
        Self {
            width: final_width,
            height: final_height,
            rgba_data: Arc::new(image),
        }
    }
}