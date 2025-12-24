use anyhow::{anyhow, Result};
use dotmax::BrailleGrid;
use std::io::BufReader;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

pub struct WebcamPlayer {
    pub grid: BrailleGrid,
    pub is_streaming: bool,
    pub device_index: usize,
    pub available_devices: Vec<String>,
    frame_receiver: Option<Receiver<Vec<u8>>>,
    ffmpeg_process: Option<Child>,
    last_frame_time: Instant,
    pub width: usize,
    pub height: usize,
    pub frame_count: usize,
    pub last_frame_size: usize,  // For debugging
    pub last_frame_avg: u8,      // For debugging
}

impl WebcamPlayer {
    pub fn new(grid_width: usize, grid_height: usize) -> Result<Self> {
        let grid = BrailleGrid::new(grid_width, grid_height)?;
        let available_devices = Self::list_devices()?;
        
        Ok(Self {
            grid,
            is_streaming: false,
            device_index: 0,
            available_devices,
            frame_receiver: None,
            ffmpeg_process: None,
            last_frame_time: Instant::now(),
            width: grid_width * 2,
            height: grid_height * 4,
            frame_count: 0,
            last_frame_size: 0,
            last_frame_avg: 0,
        })
    }

    pub fn list_devices() -> Result<Vec<String>> {
        let output = Command::new("ffmpeg")
            .args(["-f", "avfoundation", "-list_devices", "true", "-i", ""])
            .output()?;
        
        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut devices = Vec::new();
        let mut in_video = false;
        
        for line in stderr.lines() {
            if line.contains("AVFoundation video devices:") {
                in_video = true;
                continue;
            }
            if line.contains("AVFoundation audio devices:") {
                break;
            }
            if in_video && line.contains("] [") {
                if let Some(name) = line.split("] ").last() {
                    devices.push(name.to_string());
                }
            }
        }
        
        if devices.is_empty() {
            devices.push("Default Camera".to_string());
        }
        
        Ok(devices)
    }

    pub fn start_streaming(&mut self) -> Result<()> {
        if self.is_streaming {
            return Ok(());
        }

        let (grid_w, grid_h) = self.grid.dimensions();
        let width = grid_w * 2;
        let height = grid_h * 4;
        self.width = width;
        self.height = height;
        self.frame_count = 0;

        // Start ffmpeg to capture webcam and output raw grayscale frames
        // Use 1280x720 capture resolution (widely supported) then scale down
        let mut child = Command::new("ffmpeg")
            .args([
                "-f", "avfoundation",
                "-framerate", "30",
                "-video_size", "1280x720",
                "-i", &format!("{}:none", self.device_index),
                "-vf", &format!("scale={}:{},format=gray", width, height),
                "-f", "rawvideo",
                "-pix_fmt", "gray",
                "-"
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())  // Capture stderr for debugging
            .spawn()?;

        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to capture stdout"))?;
        let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
        
        let frame_size = width * height;
        
        // Spawn thread to read frames
        thread::spawn(move || {
            let mut reader = BufReader::with_capacity(frame_size * 2, stdout);
            let mut buffer = vec![0u8; frame_size];
            
            loop {
                match std::io::Read::read_exact(&mut reader, &mut buffer) {
                    Ok(_) => {
                        if tx.send(buffer.clone()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        self.ffmpeg_process = Some(child);
        self.frame_receiver = Some(rx);
        self.is_streaming = true;
        self.last_frame_time = Instant::now();
        
        Ok(())
    }

    pub fn stop_streaming(&mut self) {
        self.is_streaming = false;
        self.frame_receiver = None;
        
        if let Some(mut child) = self.ffmpeg_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    pub fn toggle_streaming(&mut self) -> Result<()> {
        if self.is_streaming {
            self.stop_streaming();
            Ok(())
        } else {
            self.start_streaming()
        }
    }

    #[allow(dead_code)]
    pub fn has_devices(&self) -> bool {
        !self.available_devices.is_empty()
    }

    pub fn next_device(&mut self) {
        if !self.available_devices.is_empty() {
            self.device_index = (self.device_index + 1) % self.available_devices.len();
            if self.is_streaming {
                self.stop_streaming();
                let _ = self.start_streaming();
            }
        }
    }

    pub fn update(&mut self) {
        if !self.is_streaming {
            return;
        }

        if let Some(ref rx) = self.frame_receiver {
            // Get the latest frame (drain channel to get most recent)
            let mut latest_frame = None;
            while let Ok(frame) = rx.try_recv() {
                latest_frame = Some(frame);
            }

            if let Some(frame) = latest_frame {
                self.last_frame_size = frame.len();
                self.last_frame_avg = if frame.is_empty() { 0 } else {
                    (frame.iter().map(|&b| b as u64).sum::<u64>() / frame.len() as u64) as u8
                };
                self.render_frame(&frame);
                self.last_frame_time = Instant::now();
                self.frame_count += 1;
            }
        }
    }

    fn render_frame(&mut self, frame: &[u8]) {
        let (grid_w, grid_h) = self.grid.dimensions();
        let dot_w = grid_w * 2;
        let dot_h = grid_h * 4;

        // Debug: skip if frame size mismatch
        let expected_size = self.width * self.height;
        if frame.len() != expected_size {
            return;
        }

        self.grid.clear();

        for y in 0..dot_h {
            for x in 0..dot_w {
                // Scale to source frame
                let src_x = (x * self.width) / dot_w;
                let src_y = (y * self.height) / dot_h;
                let idx = src_y * self.width + src_x;

                if idx < frame.len() && frame[idx] > 80 {
                    let _ = self.grid.set_dot(x, y);
                }
            }
        }
    }

    pub fn current_device_name(&self) -> &str {
        self.available_devices
            .get(self.device_index)
            .map(|s| s.as_str())
            .unwrap_or("No device")
    }
}

impl Drop for WebcamPlayer {
    fn drop(&mut self) {
        self.stop_streaming();
    }
}

