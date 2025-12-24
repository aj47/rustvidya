mod video;

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
            status_message: String::from("Welcome to RustVidya 🎬 - Select a video file"),
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
        self.video_player.update();
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
                        KeyCode::Char(' ') => app.video_player.toggle_playback(),
                        KeyCode::Enter => app.select(),
                        KeyCode::Up | KeyCode::Char('k') => app.previous(),
                        KeyCode::Down | KeyCode::Char('j') => app.next(),
                        KeyCode::Left | KeyCode::Char('h') => app.video_player.seek_backward(),
                        KeyCode::Right | KeyCode::Char('l') => app.video_player.seek_forward(),
                        KeyCode::Char('s') => app.video_player.stop(),
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
    let title = if app.video_player.has_video() {
        format!("🎬 RustVidya - {} fps, {}x{}",
            app.video_player.fps as u32,
            app.video_player.width,
            app.video_player.height)
    } else {
        "🎬 RustVidya - Terminal Video Player".to_string()
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
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Video ")
        .title_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    f.render_widget(block, area);

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

fn draw_status(f: &mut Frame, area: Rect, app: &App) {
    let controls = vec![
        Span::styled(&app.status_message, Style::default().fg(Color::Green)),
        Span::raw("  │  "),
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
    ];

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
