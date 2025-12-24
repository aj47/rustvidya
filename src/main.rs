mod video;
mod webcam;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dotmax::BrailleGrid;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};
use std::path::PathBuf;
use std::{io, time::Duration};
use video::VideoPlayer;
use webcam::WebcamPlayer;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Video,
    Webcam,
}

#[derive(Debug, Clone)]
struct FileEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
}

struct App {
    files: Vec<FileEntry>,
    selected: usize,
    current_dir: PathBuf,
    video_player: VideoPlayer,
    webcam_player: WebcamPlayer,
    mode: Mode,
    status_message: String,
    scroll_offset: usize,
}

impl App {
    fn new() -> Result<Self> {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut app = Self {
            files: Vec::new(),
            selected: 0,
            current_dir: current_dir.clone(),
            video_player: VideoPlayer::new(80, 20)?,
            webcam_player: WebcamPlayer::new(80, 20)?,
            mode: Mode::Video,
            status_message: String::from("Welcome to RustVidya 🎬 - [w] Webcam | Select a video file"),
            scroll_offset: 0,
        };
        app.load_directory(&current_dir);
        Ok(app)
    }

    fn load_directory(&mut self, path: &PathBuf) {
        self.files.clear();
        self.selected = 0;
        self.scroll_offset = 0;

        if let Some(parent) = path.parent() {
            self.files.push(FileEntry {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                is_dir: true,
            });
        }

        if let Ok(entries) = std::fs::read_dir(path) {
            let mut entries: Vec<_> = entries
                .filter_map(|e| e.ok())
                .map(|e| {
                    let path = e.path();
                    let is_dir = path.is_dir();
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    FileEntry { name, path, is_dir }
                })
                .filter(|e| {
                    if e.is_dir {
                        !e.name.starts_with('.')
                    } else {
                        Self::is_video_file(&e.path)
                    }
                })
                .collect();

            entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            });

            self.files.extend(entries);
        }

        self.current_dir = path.clone();
    }

    fn is_video_file(path: &std::path::Path) -> bool {
        let extensions = ["mp4", "mkv", "avi", "webm", "mov", "wmv", "flv", "m4v"];
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| extensions.contains(&e.to_lowercase().as_str()))
            .unwrap_or(false)
    }

    fn next(&mut self) {
        if !self.files.is_empty() {
            self.selected = (self.selected + 1).min(self.files.len() - 1);
        }
    }

    fn previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn select(&mut self) {
        if let Some(entry) = self.files.get(self.selected) {
            if entry.is_dir {
                let path = entry.path.clone();
                self.load_directory(&path);
            } else {
                let path = entry.path.clone();
                self.status_message = format!("Loading: {}", entry.name);
                match self.video_player.load_video(&path) {
                    Ok(()) => {
                        self.status_message = format!("Playing: {} ({} frames)", entry.name, self.video_player.total_frames);
                        self.video_player.play();
                    }
                    Err(e) => {
                        self.status_message = format!("Error: {}", e);
                    }
                }
            }
        }
    }

    fn update(&mut self) {
        match self.mode {
            Mode::Video => self.video_player.update(),
            Mode::Webcam => self.webcam_player.update(),
        }
    }
}

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()?;

    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| draw_ui(f, app))?;

        if event::poll(Duration::from_millis(16))? { // ~60fps polling
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('w') => {
                            // Toggle webcam mode
                            if app.mode == Mode::Webcam {
                                app.webcam_player.stop_streaming();
                                app.mode = Mode::Video;
                                app.status_message = "Video mode - [w] Webcam".to_string();
                            } else {
                                app.video_player.stop();
                                app.mode = Mode::Webcam;
                                if let Err(e) = app.webcam_player.start_streaming() {
                                    app.status_message = format!("Webcam error: {}", e);
                                } else {
                                    app.status_message = format!("📷 Webcam: {} - [n] Next device | [w] Video mode",
                                        app.webcam_player.current_device_name());
                                }
                            }
                        }
                        KeyCode::Char('n') if app.mode == Mode::Webcam => {
                            app.webcam_player.next_device();
                            app.status_message = format!("📷 {} | Mode: {} | Color: {}",
                                app.webcam_player.current_device_name(),
                                app.webcam_player.render_mode.name(),
                                app.webcam_player.color_mode.name());
                        }
                        KeyCode::Char('m') if app.mode == Mode::Webcam => {
                            // Cycle render mode
                            app.webcam_player.cycle_render_mode();
                            app.status_message = format!("Render mode: {} | [m] Next mode | [c] Color",
                                app.webcam_player.render_mode.name());
                        }
                        KeyCode::Char('c') if app.mode == Mode::Webcam => {
                            // Cycle color mode
                            app.webcam_player.cycle_color_mode();
                            app.status_message = format!("Color: {} | [c] Next color | [m] Mode",
                                app.webcam_player.color_mode.name());
                        }
                        KeyCode::Char('d') if app.mode == Mode::Webcam => {
                            // Cycle dithering mode
                            app.webcam_player.cycle_dithering_mode();
                            app.status_message = format!("Dithering: {} | [d] Next dither | [m] Mode",
                                app.webcam_player.dithering_mode.name());
                        }
                        KeyCode::Char('+') | KeyCode::Char('=') if app.mode == Mode::Webcam => {
                            // Increase brightness threshold
                            app.webcam_player.increase_threshold();
                            app.status_message = format!("Threshold: {} | [+/-] Adjust",
                                app.webcam_player.brightness_threshold);
                        }
                        KeyCode::Char('-') if app.mode == Mode::Webcam => {
                            // Decrease brightness threshold
                            app.webcam_player.decrease_threshold();
                            app.status_message = format!("Threshold: {} | [+/-] Adjust",
                                app.webcam_player.brightness_threshold);
                        }
                        KeyCode::Char(' ') => {
                            if app.mode == Mode::Webcam {
                                let _ = app.webcam_player.toggle_streaming();
                            } else {
                                app.video_player.toggle_playback();
                            }
                        }
                        KeyCode::Enter => {
                            if app.mode == Mode::Video {
                                app.select();
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => app.previous(),
                        KeyCode::Down | KeyCode::Char('j') => app.next(),
                        KeyCode::Left | KeyCode::Char('h') => {
                            if app.mode == Mode::Video {
                                app.video_player.seek_backward();
                            }
                        }
                        KeyCode::Right | KeyCode::Char('l') => {
                            if app.mode == Mode::Video {
                                app.video_player.seek_forward();
                            }
                        }
                        KeyCode::Char('s') => {
                            if app.mode == Mode::Video {
                                app.video_player.stop();
                            } else {
                                app.webcam_player.stop_streaming();
                            }
                        }
                        KeyCode::Backspace => {
                            if let Some(parent) = app.current_dir.parent() {
                                let path = parent.to_path_buf();
                                app.load_directory(&path);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        app.update();
    }
}

fn draw_ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Main content (video + file list)
            Constraint::Length(3),  // Progress bar
            Constraint::Length(3),  // Status bar
        ])
        .split(f.area());

    draw_header(f, chunks[0], app);
    draw_main(f, chunks[1], app);
    draw_progress(f, chunks[2], app);
    draw_status(f, chunks[3], app);
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let title = match app.mode {
        Mode::Webcam => {
            if app.webcam_player.is_streaming {
                format!("📷 {} | Mode: {} | Dither: {} | Color: {} | Thresh: {} | {}fps",
                    app.webcam_player.current_device_name(),
                    app.webcam_player.render_mode.name(),
                    app.webcam_player.dithering_mode.name(),
                    app.webcam_player.color_mode.name(),
                    app.webcam_player.brightness_threshold,
                    app.webcam_player.frame_count)
            } else {
                format!("📷 RustVidya - Webcam: {} (paused)",
                    app.webcam_player.current_device_name())
            }
        }
        Mode::Video => {
            if app.video_player.has_video() {
                format!("🎬 RustVidya - {} fps, {}x{}",
                    app.video_player.fps as u32,
                    app.video_player.width,
                    app.video_player.height)
            } else {
                "🎬 RustVidya - Terminal Video Player".to_string()
            }
        }
    };

    let header = Paragraph::new(title)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Magenta)));

    f.render_widget(header, area);
}

fn draw_main(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(40), Constraint::Length(35)])
        .split(area);

    draw_video(f, chunks[0], app);
    draw_file_list(f, chunks[1], app);
}

fn draw_video(f: &mut Frame, area: Rect, app: &mut App) {
    let (title, border_color) = match app.mode {
        Mode::Webcam => (" 📷 Webcam ", Color::Green),
        Mode::Video => (" 🎬 Video ", Color::Yellow),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title)
        .title_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    match app.mode {
        Mode::Webcam => {
            // Resize webcam grid if needed - must restart streaming to match new size
            let (grid_w, grid_h) = app.webcam_player.grid.dimensions();
            if grid_w != inner.width as usize || grid_h != inner.height as usize {
                if let Ok(new_grid) = BrailleGrid::new(inner.width as usize, inner.height as usize) {
                    let was_streaming = app.webcam_player.is_streaming;
                    if was_streaming {
                        app.webcam_player.stop_streaming();
                    }
                    app.webcam_player.grid = new_grid;
                    if was_streaming {
                        let _ = app.webcam_player.start_streaming();
                    }
                }
            }

            if !app.webcam_player.is_streaming {
                let msg = Paragraph::new("Webcam paused\n\nPress [Space] to start streaming")
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center);
                f.render_widget(msg, inner);
                return;
            }

            // Render webcam grid (braille or density characters with optional colors)
            let (grid_w, grid_h) = app.webcam_player.grid.dimensions();
            let mut lines: Vec<Line> = Vec::new();

            for row in 0..grid_h {
                let mut spans = Vec::new();
                for col in 0..grid_w {
                    let ch = app.webcam_player.grid.get_char(col, row);
                    let color = if let Some(dotmax_color) = app.webcam_player.grid.get_color(col, row) {
                        // Convert dotmax::Color to ratatui::Color
                        Color::Rgb(dotmax_color.r, dotmax_color.g, dotmax_color.b)
                    } else {
                        Color::White  // Default to white for no color
                    };
                    spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
                }
                lines.push(Line::from(spans));
            }

            let para = Paragraph::new(lines);
            f.render_widget(para, inner);
        }
        Mode::Video => {
            if !app.video_player.has_video() {
                let msg = Paragraph::new("No video loaded\n\nSelect a video file from the list →")
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center);
                f.render_widget(msg, inner);
                return;
            }

            // Resize grid if needed
            let (grid_w, grid_h) = app.video_player.grid.dimensions();
            if grid_w != inner.width as usize || grid_h != inner.height as usize {
                if let Ok(new_grid) = BrailleGrid::new(inner.width as usize, inner.height as usize) {
                    app.video_player.grid = new_grid;
                    app.video_player.render_current_frame();
                }
            }

            // Render braille grid
            let (grid_w, grid_h) = app.video_player.grid.dimensions();
            let mut lines: Vec<Line> = Vec::new();

            for row in 0..grid_h {
                let mut spans = Vec::new();
                for col in 0..grid_w {
                    let ch = app.video_player.grid.get_char(col, row);
                    spans.push(Span::styled(ch.to_string(), Style::default().fg(Color::White)));
                }
                lines.push(Line::from(spans));
            }

            let para = Paragraph::new(lines);
            f.render_widget(para, inner);
        }
    }
}

fn draw_file_list(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" 📂 {} ", truncate_path(&app.current_dir.to_string_lossy(), 25)))
        .title_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    let visible_height = inner.height as usize;

    // Update scroll offset to keep selected item visible
    if app.selected < app.scroll_offset {
        app.scroll_offset = app.selected;
    } else if app.selected >= app.scroll_offset + visible_height {
        app.scroll_offset = app.selected - visible_height + 1;
    }

    let items: Vec<ListItem> = app
        .files
        .iter()
        .enumerate()
        .skip(app.scroll_offset)
        .take(visible_height)
        .map(|(i, entry)| {
            let icon = if entry.is_dir { "📁 " } else { "🎬 " };
            let style = if i == app.selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else if entry.is_dir {
                Style::default().fg(Color::Blue)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(format!("{}{}", icon, entry.name)).style(style)
        })
        .collect();

    let list = List::new(items);

    f.render_widget(block, area);
    f.render_widget(list, inner);
}

fn draw_progress(f: &mut Frame, area: Rect, app: &App) {
    match app.mode {
        Mode::Webcam => {
            let state = if app.webcam_player.is_streaming { "📷 LIVE" } else { "⏸ PAUSED" };
            let device = app.webcam_player.current_device_name();
            let label = format!("{} - {}", state, device);

            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Green)).title(" Webcam "))
                .gauge_style(Style::default().fg(Color::Green).bg(Color::DarkGray))
                .ratio(if app.webcam_player.is_streaming { 1.0 } else { 0.0 })
                .label(label);

            f.render_widget(gauge, area);
        }
        Mode::Video => {
            let progress = app.video_player.progress();
            let duration = app.video_player.duration_str();
            let state = if app.video_player.is_playing { "▶" } else { "⏸" };

            let label = format!("{} {} {:>3.0}%", state, duration, progress * 100.0);

            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow)).title(" Progress "))
                .gauge_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray))
                .ratio(progress)
                .label(label);

            f.render_widget(gauge, area);
        }
    }
}

fn draw_status(f: &mut Frame, area: Rect, app: &App) {
    let controls = match app.mode {
        Mode::Webcam => vec![
            Span::styled(&app.status_message, Style::default().fg(Color::Green)),
            Span::raw("  │  "),
            Span::styled("w", Style::default().fg(Color::Yellow)),
            Span::raw(" Video "),
            Span::styled("m", Style::default().fg(Color::Cyan)),
            Span::raw(" Mode "),
            Span::styled("c", Style::default().fg(Color::Magenta)),
            Span::raw(" Color "),
            Span::styled("+/-", Style::default().fg(Color::Yellow)),
            Span::raw(" Thresh "),
            Span::styled("n", Style::default().fg(Color::Yellow)),
            Span::raw(" Dev "),
            Span::styled("Space", Style::default().fg(Color::Yellow)),
            Span::raw(" Toggle "),
            Span::styled("q", Style::default().fg(Color::Red)),
            Span::raw(" Quit"),
        ],
        Mode::Video => vec![
            Span::styled(&app.status_message, Style::default().fg(Color::Green)),
            Span::raw("  │  "),
            Span::styled("w", Style::default().fg(Color::Yellow)),
            Span::raw(" Webcam "),
            Span::styled("Space", Style::default().fg(Color::Yellow)),
            Span::raw(" Play/Pause "),
            Span::styled("←→", Style::default().fg(Color::Yellow)),
            Span::raw(" Seek "),
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(" Nav "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" Select "),
            Span::styled("q", Style::default().fg(Color::Red)),
            Span::raw(" Quit"),
        ],
    };

    let status = Paragraph::new(Line::from(controls))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));

    f.render_widget(status, area);
}

fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else {
        format!("...{}", &path[path.len() - max_len + 3..])
    }
}
