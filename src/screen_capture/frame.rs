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
                "-f",
                "rawvideo", // input is raw video
                "-pixel_format",
                "rgba",
                "-video_size",
                &format!("{}x{}", self.width, self.height),
                "-i",
                "-", // input from stdin
                "-c:v",
                "hevc", // Codec H.265
                "-preset",
                "ultrafast",
                "-f",
                "rawvideo", // output raw
                "-",        // output to stdout
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()) // Stdio::null() Ignora errori di ffmpeg
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

pub fn decode_from_h265_to_rgba(
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
            "pipe:1",   // output to stdout
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
            "-f",
            "hevc",
            "-i",
            "pipe:0",
            "-vframes",
            "1", // Process only first frame
            "-vf",
            "scale=iw:ih", // Force scale filter to report size
            "-f",
            "null",
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
