use anyhow::{anyhow, Result};
use dotmax::BrailleGrid;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

pub struct VideoPlayer {
    pub grid: BrailleGrid,
    pub is_playing: bool,
    pub current_frame: usize,
    pub total_frames: usize,
    pub fps: f64,
    pub width: u32,
    pub height: u32,
    frames: Vec<Vec<u8>>,
    last_frame_time: Instant,
    frame_duration_ms: u64,
    temp_dir: Option<String>,
}

impl VideoPlayer {
    pub fn new(grid_width: usize, grid_height: usize) -> Result<Self> {
        let grid = BrailleGrid::new(grid_width, grid_height)?;
        Ok(Self {
            grid,
            is_playing: false,
            current_frame: 0,
            total_frames: 0,
            fps: 30.0,
            width: 160,
            height: 96,
            frames: Vec::new(),
            last_frame_time: Instant::now(),
            frame_duration_ms: 33,
            temp_dir: None,
        })
    }

    pub fn load_video(&mut self, path: &Path) -> Result<()> {
        self.frames.clear();
        self.current_frame = 0;
        self.is_playing = false;

        let temp_dir = format!("/tmp/rustvidya_{}", std::process::id());
        fs::create_dir_all(&temp_dir)?;
        self.temp_dir = Some(temp_dir.clone());

        // Get video info
        let probe = Command::new("ffprobe")
            .args(["-v", "error", "-select_streams", "v:0",
                   "-show_entries", "stream=r_frame_rate",
                   "-of", "csv=p=0",
                   path.to_str().ok_or_else(|| anyhow!("Invalid path"))?])
            .output()?;

        let fps_str = String::from_utf8_lossy(&probe.stdout);
        if let Some(fps_part) = fps_str.trim().split(',').next() {
            let parts: Vec<&str> = fps_part.split('/').collect();
            if parts.len() == 2 {
                if let (Ok(n), Ok(d)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                    if d > 0.0 { self.fps = n / d; }
                }
            }
        }
        if self.fps <= 0.0 || self.fps > 120.0 { self.fps = 30.0; }
        self.frame_duration_ms = (1000.0 / self.fps) as u64;

        let (grid_w, grid_h) = self.grid.dimensions();
        let target_w = grid_w * 2;
        let target_h = grid_h * 4;

        // Extract frames
        let status = Command::new("ffmpeg")
            .args(["-i", path.to_str().unwrap(),
                   "-vf", &format!("scale={}:{},format=gray", target_w, target_h),
                   "-t", "120", "-r", &format!("{}", self.fps.min(30.0)),
                   &format!("{}/frame_%05d.png", temp_dir)])
            .output()?;

        if !status.status.success() {
            return Err(anyhow!("ffmpeg failed"));
        }

        let mut files: Vec<_> = fs::read_dir(&temp_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |x| x == "png"))
            .collect();
        files.sort_by_key(|e| e.path());

        for entry in files.iter().take(3600) {
            let img = image::open(entry.path())?;
            self.frames.push(img.to_luma8().into_raw());
        }

        self.total_frames = self.frames.len();
        self.width = target_w as u32;
        self.height = target_h as u32;
        self.cleanup_temp();

        if self.frames.is_empty() {
            return Err(anyhow!("No frames extracted"));
        }
        Ok(())
    }

    fn cleanup_temp(&mut self) {
        if let Some(ref dir) = self.temp_dir {
            let _ = fs::remove_dir_all(dir);
        }
        self.temp_dir = None;
    }

    pub fn play(&mut self) {
        self.is_playing = true;
        self.last_frame_time = Instant::now();
    }

    pub fn pause(&mut self) { self.is_playing = false; }

    pub fn toggle_playback(&mut self) {
        if self.is_playing { self.pause(); } else { self.play(); }
    }

    pub fn stop(&mut self) {
        self.is_playing = false;
        self.current_frame = 0;
    }

    pub fn seek_forward(&mut self) {
        let skip = (self.fps * 5.0) as usize;
        self.current_frame = (self.current_frame + skip).min(self.total_frames.saturating_sub(1));
    }

    pub fn seek_backward(&mut self) {
        let skip = (self.fps * 5.0) as usize;
        self.current_frame = self.current_frame.saturating_sub(skip);
    }

    pub fn has_video(&self) -> bool { !self.frames.is_empty() }

    pub fn update(&mut self) {
        if !self.is_playing || self.frames.is_empty() { return; }
        let elapsed = self.last_frame_time.elapsed().as_millis() as u64;
        if elapsed >= self.frame_duration_ms {
            self.current_frame += 1;
            if self.current_frame >= self.total_frames { self.current_frame = 0; }
            self.last_frame_time = Instant::now();
            self.render_current_frame();
        }
    }

    pub fn render_current_frame(&mut self) {
        if self.frames.is_empty() || self.current_frame >= self.frames.len() { return; }
        let frame = &self.frames[self.current_frame];
        let (grid_w, grid_h) = self.grid.dimensions();
        let dot_w = grid_w * 2;
        let dot_h = grid_h * 4;

        // Source frame dimensions
        let src_w = self.width as usize;
        let src_h = self.height as usize;

        self.grid.clear();

        // Scale frame to current grid size using nearest-neighbor sampling
        for y in 0..dot_h {
            for x in 0..dot_w {
                // Map current position to source frame position
                let src_x = (x * src_w) / dot_w;
                let src_y = (y * src_h) / dot_h;
                let idx = src_y * src_w + src_x;

                if idx < frame.len() && frame[idx] > 80 {
                    let _ = self.grid.set_dot(x, y);
                }
            }
        }
    }

    pub fn progress(&self) -> f64 {
        if self.total_frames == 0 { 0.0 } else { self.current_frame as f64 / self.total_frames as f64 }
    }

    pub fn duration_str(&self) -> String {
        if self.total_frames == 0 || self.fps == 0.0 { return "0:00 / 0:00".to_string(); }
        let cur = self.current_frame as f64 / self.fps;
        let tot = self.total_frames as f64 / self.fps;
        format!("{}:{:02} / {}:{:02}", cur as u32 / 60, cur as u32 % 60, tot as u32 / 60, tot as u32 % 60)
    }
}

impl Drop for VideoPlayer {
    fn drop(&mut self) { self.cleanup_temp(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_video_loading_and_rendering() {
        // Create a video player
        let mut player = VideoPlayer::new(40, 12).expect("Failed to create player");

        // Load the test video
        let test_video = PathBuf::from("test_video.mp4");
        if test_video.exists() {
            let result = player.load_video(&test_video);
            assert!(result.is_ok(), "Failed to load video: {:?}", result.err());

            // Check that frames were loaded
            assert!(player.total_frames > 0, "No frames loaded");
            assert!(player.has_video(), "has_video() should return true");

            // Render first frame
            player.render_current_frame();

            // Check grid dimensions
            let (w, h) = player.grid.dimensions();
            assert_eq!(w, 40);
            assert_eq!(h, 12);

            // Test playback controls
            player.play();
            assert!(player.is_playing);
            player.pause();
            assert!(!player.is_playing);
            player.toggle_playback();
            assert!(player.is_playing);
            player.stop();
            assert!(!player.is_playing);
            assert_eq!(player.current_frame, 0);

            // Test seeking
            player.current_frame = 50;
            player.seek_forward();
            assert!(player.current_frame > 50);
            player.seek_backward();

            // Test progress
            let progress = player.progress();
            assert!(progress >= 0.0 && progress <= 1.0);

            // Test duration string
            let dur = player.duration_str();
            assert!(dur.contains("/"));

            println!("Video loaded successfully!");
            println!("Total frames: {}", player.total_frames);
            println!("FPS: {}", player.fps);
            println!("Duration: {}", player.duration_str());

            // Render and print the braille output
            player.render_current_frame();
            let chars = player.grid.to_unicode_grid();
            println!("\nBraille output (first frame, first 5 rows):");
            for row in chars.iter().take(5) {
                let line: String = row.iter().collect();
                println!("{}", line);
            }
            println!("...");

            // Test grid resizing - create a new grid with different dimensions
            // and verify rendering still works
            let new_grid = BrailleGrid::new(60, 18).expect("Failed to create new grid");
            player.grid = new_grid;
            player.render_current_frame();
            let (new_w, new_h) = player.grid.dimensions();
            assert_eq!(new_w, 60);
            assert_eq!(new_h, 18);

            let chars = player.grid.to_unicode_grid();
            println!("\nBraille output after resize (60x18, first 3 rows):");
            for row in chars.iter().take(3) {
                let line: String = row.iter().collect();
                println!("{}", line);
            }
        } else {
            println!("Test video not found, skipping test");
        }
    }
}
