use std::io::stdout;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, List, ListItem, Paragraph},
    Terminal,
};
use tokio::sync::mpsc;

use yoink_core::engine::YtDlpEngine;
use yoink_core::format::{format_duration, format_eta, format_speed, shorten_path, truncate, wrap_text};
use yoink_core::progress::ProgressEvent;
use yoink_core::traits::{DownloadRequest, Downloader};

use crate::state::{AppState, Phase};

const LOGO_LINES: [&str; 3] = [
    "█ █  █▀█  ▀█▀  █▀█  █ █  █▀▀",
    "▀█▀  █ █   █   █ █  █▀▄  ▀▀█",
    " ▀   ▀▀▀  ▀▀▀  ▀ ▀  ▀ ▀  ▀▀▀",
];

const TAGLINE: &str = "yoink any video. paste. yoink. done.";
const SITES_CAPTION: &str = "youtube · x · instagram · threads · tiktok · +1800 more";
const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

enum AppEvent {
    Key(crossterm::event::KeyEvent),
    Mouse(crossterm::event::MouseEvent),
    Tick,
    ProbeSuccess(yoink_core::traits::ProbeResult),
    ProbeError(String),
    Progress(ProgressEvent),
    DownloadFinished(PathBuf),
    DownloadError(String),
}

pub async fn run_interactive(initial_url: Option<String>, out_dir: Option<PathBuf>) -> anyhow::Result<Option<PathBuf>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = AppState::new(initial_url.clone());
    let default_out = out_dir.unwrap_or_else(|| {
        directories::UserDirs::new()
            .and_then(|u| u.download_dir().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
    });

    // Monochrome & Minimal Theme
    let theme_colors = [
        Color::White,   // Main
        Color::Gray,    // Secondary
        Color::DarkGray,// Borders
        Color::Black,   // Background
    ];
    let style_normal = Style::default().fg(Color::Gray);
    let style_highlight = Style::default().fg(Color::White).add_modifier(Modifier::BOLD);
    let style_active = Style::default().fg(Color::White).add_modifier(Modifier::BOLD);
    let block_style = Style::default().fg(Color::DarkGray);

    // Function to apply monochrome styling to UI components
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                let trimmed = text.trim();
                if yoink_core::platform::is_probably_url(trimmed) {
                    state.clipboard_url = Some(trimmed.to_string());
                }
            }
        }
    }

    let (event_tx, mut event_rx) = mpsc::channel::<AppEvent>(100);
    let engine = std::sync::Arc::new(YtDlpEngine::new().await?);

    // Event polling task
    let ev_tx = event_tx.clone();
    tokio::spawn(async move {
        loop {
            if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                if let Ok(ev) = event::read() {
                    match ev {
                        Event::Key(k) => {
                            if ev_tx.send(AppEvent::Key(k)).await.is_err() {
                                break;
                            }
                        }
                        Event::Mouse(m) => {
                            if ev_tx.send(AppEvent::Mouse(m)).await.is_err() {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            } else {
                let _ = ev_tx.send(AppEvent::Tick).await;
            }
        }
    });

    // If initial URL was given, kick off probe
    if let Some(url) = initial_url {
        let eng = engine.clone();
        let tx = event_tx.clone();
        tokio::spawn(async move {
            match eng.probe(&url).await {
                Ok(res) => {
                    let _ = tx.send(AppEvent::ProbeSuccess(res)).await;
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::ProbeError(e.to_string())).await;
                }
            }
        });
    }

    let mut tick_count: usize = 0;
    let mut active_download_cancel: Option<tokio::task::JoinHandle<()>> = None;

    while !state.should_exit {
        tick_count = tick_count.wrapping_add(1);
        terminal.draw(|f| render_ui(f, &state, tick_count))?;

        if let Some(app_ev) = event_rx.recv().await {
            match app_ev {
                AppEvent::Tick => {}
                AppEvent::Key(key) => {
                    // Global keys
                    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                        state.should_exit = true;
                        break;
                    }
                    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('t') {
                        state.cycle_theme();
                        continue;
                    }

                    match &state.phase {
                        Phase::Input { .. } => match key.code {
                            KeyCode::Enter => {
                                if let Some(url) = state.submit_input() {
                                    let eng = engine.clone();
                                    let tx = event_tx.clone();
                                    tokio::spawn(async move {
                                        match eng.probe(&url).await {
                                            Ok(res) => {
                                                let _ = tx.send(AppEvent::ProbeSuccess(res)).await;
                                            }
                                            Err(e) => {
                                                let _ = tx.send(AppEvent::ProbeError(e.to_string())).await;
                                            }
                                        }
                                    });
                                }
                            }
                            KeyCode::Tab => {
                                if let Some(clip) = &state.clipboard_url {
                                    state.set_url_input(clip.clone());
                                }
                            }
                            KeyCode::Backspace => {
                                if !state.url_input.is_empty() {
                                    state.url_input.pop();
                                }
                            }
                            KeyCode::Char(c) => {
                                state.url_input.push(c);
                            }
                            _ => {}
                        },
                        Phase::Probing { .. } => {
                            if key.code == KeyCode::Esc {
                                state.reset_to_input();
                            }
                        }
                        Phase::Picking { selected_index } => {
                            let total = state
                                .probe_result
                                .as_ref()
                                .map(|p| p.choices.len())
                                .unwrap_or(0);

                            match key.code {
                                KeyCode::Up | KeyCode::Char('k') => {
                                    let new_idx = if *selected_index == 0 {
                                        total.saturating_sub(1)
                                    } else {
                                        selected_index - 1
                                    };
                                    state.phase = Phase::Picking { selected_index: new_idx };
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    let new_idx = if *selected_index + 1 >= total {
                                        0
                                    } else {
                                        selected_index + 1
                                    };
                                    state.phase = Phase::Picking { selected_index: new_idx };
                                }
                                KeyCode::Esc => {
                                    state.reset_to_input();
                                }
                                KeyCode::Enter => {
                                    if let Some(probe) = &state.probe_result {
                                        if let Some(choice) = probe.choices.get(*selected_index) {
                                            let choice_clone = choice.clone();
                                            state.phase = Phase::Downloading {
                                                choice: choice_clone.clone(),
                                                progress: None,
                                                processing: false,
                                            };

                                            let eng = engine.clone();
                                            let tx = event_tx.clone();
                                            let req = DownloadRequest {
                                                url: probe.info.webpage_url.clone().unwrap_or_else(|| state.url_input.clone()),
                                                choice: choice_clone,
                                                out_dir: default_out.clone(),
                                                info_json_path: probe.info_json_path.clone(),
                                            };

                                            let cancel_task = tokio::spawn(async move {
                                                match eng.download(req).await {
                                                    Ok((mut rx, handle)) => {
                                                        let tx_inner = tx.clone();
                                                        tokio::spawn(async move {
                                                            while let Ok(event) = rx.recv().await {
                                                                let _ = tx_inner.send(AppEvent::Progress(event)).await;
                                                            }
                                                        });
                                                        match handle.await {
                                                            Ok(Ok(dest)) => {
                                                                let _ = tx.send(AppEvent::DownloadFinished(dest)).await;
                                                            }
                                                            Ok(Err(e)) => {
                                                                let _ = tx.send(AppEvent::DownloadError(e.to_string())).await;
                                                            }
                                                            Err(e) => {
                                                                let _ = tx.send(AppEvent::DownloadError(e.to_string())).await;
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        let _ = tx.send(AppEvent::DownloadError(e.to_string())).await;
                                                    }
                                                }
                                            });
                                            active_download_cancel = Some(cancel_task);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        Phase::Downloading { .. } => {
                            if key.code == KeyCode::Esc {
                                if let Some(handle) = active_download_cancel.take() {
                                    handle.abort();
                                }
                                state.reset_to_input();
                            }
                        }
                        Phase::Done { .. } => {
                            if key.code == KeyCode::Enter || key.code == KeyCode::Esc {
                                state.reset_to_input();
                            }
                        }
                        Phase::Error { .. } => {
                            if key.code == KeyCode::Enter || key.code == KeyCode::Esc {
                                state.reset_to_input();
                            }
                        }
                    }
                }
                AppEvent::Mouse(mouse) => {
                    if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
                        // Mouse click on Done button or Yoink button
                        match &state.phase {
                            Phase::Input { .. } => {
                                if let Some(url) = state.submit_input() {
                                    let eng = engine.clone();
                                    let tx = event_tx.clone();
                                    tokio::spawn(async move {
                                        match eng.probe(&url).await {
                                            Ok(res) => {
                                                let _ = tx.send(AppEvent::ProbeSuccess(res)).await;
                                            }
                                            Err(e) => {
                                                let _ = tx.send(AppEvent::ProbeError(e.to_string())).await;
                                            }
                                        }
                                    });
                                }
                            }
                            Phase::Done { .. } | Phase::Error { .. } => {
                                state.reset_to_input();
                            }
                            _ => {}
                        }
                    }
                }
                AppEvent::ProbeSuccess(probe) => {
                    state.probe_result = Some(probe);
                    state.phase = Phase::Picking { selected_index: 0 };
                }
                AppEvent::ProbeError(err) => {
                    state.phase = Phase::Error { message: err };
                }
                AppEvent::Progress(ev) => {
                    if let Phase::Downloading { choice, .. } = &state.phase {
                        match &ev {
                            ProgressEvent::Processing { .. } => {
                                state.phase = Phase::Downloading {
                                    choice: choice.clone(),
                                    progress: Some(ev),
                                    processing: true,
                                };
                            }
                            _ => {
                                state.phase = Phase::Downloading {
                                    choice: choice.clone(),
                                    progress: Some(ev),
                                    processing: false,
                                };
                            }
                        }
                    }
                }
                AppEvent::DownloadFinished(dest) => {
                    state.outcome_path = Some(dest.clone());
                    state.phase = Phase::Done { filepath: dest };
                }
                AppEvent::DownloadError(err) => {
                    state.phase = Phase::Error { message: err };
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;

    Ok(state.outcome_path)
}

fn render_ui(frame: &mut ratatui::Frame, state: &AppState, tick: usize) {
    let area = frame.area();
    let theme = &state.theme;

    // Fill obsidian background
    let bg_block = Block::default().style(Style::default().bg(theme.background));
    frame.render_widget(bg_block, area);

    // Centered content column (width ~68 chars)
    let content_w = 68.min(area.width.saturating_sub(4));
    let content_h = 24.min(area.height.saturating_sub(2));

    let horiz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((area.width.saturating_sub(content_w)) / 2),
            Constraint::Length(content_w),
            Constraint::Min(0),
        ])
        .split(area);

    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(content_h)) / 2),
            Constraint::Length(content_h),
            Constraint::Min(0),
        ])
        .split(horiz[1]);

    let card_area = vert[1];

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Logo
            Constraint::Length(1), // Tagline
            Constraint::Length(1), // Sites
            Constraint::Length(1), // Gap
            Constraint::Min(10),   // Active stage
            Constraint::Length(2), // Footer hints
        ])
        .split(card_area);

    // 1. Logo
    let logo_text = LOGO_LINES
        .iter()
        .map(|&l| Line::from(Span::styled(l, Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))))
        .collect::<Vec<_>>();
    let logo_widget = Paragraph::new(logo_text).alignment(Alignment::Center);
    frame.render_widget(logo_widget, rows[0]);

    // 2. Tagline
    let tagline_widget = Paragraph::new(Line::from(Span::styled(
        TAGLINE,
        Style::default().fg(theme.accent_cyan).add_modifier(Modifier::BOLD),
    )))
    .alignment(Alignment::Center);
    frame.render_widget(tagline_widget, rows[1]);

    // 3. Sites Caption
    let sites_widget = Paragraph::new(Line::from(Span::styled(
        SITES_CAPTION,
        Style::default().fg(theme.secondary),
    )))
    .alignment(Alignment::Center);
    frame.render_widget(sites_widget, rows[2]);

    // 5. Active Stage
    let stage_area = rows[4];
    match &state.phase {
        Phase::Input { warning } => {
            render_input_stage(frame, stage_area, state, warning.as_deref());
        }
        Phase::Probing { status } => {
            render_probing_stage(frame, stage_area, state, status, tick);
        }
        Phase::Picking { selected_index } => {
            render_picking_stage(frame, stage_area, state, *selected_index);
        }
        Phase::Downloading { choice, progress, processing } => {
            render_downloading_stage(frame, stage_area, state, choice, progress.as_ref(), *processing, tick);
        }
        Phase::Done { filepath } => {
            render_done_stage(frame, stage_area, state, filepath);
        }
        Phase::Error { message } => {
            render_error_stage(frame, stage_area, state, message);
        }
    }

    // 6. Footer hints
    let footer_area = rows[5];
    render_footer(frame, footer_area, state);
}

fn render_input_stage(frame: &mut ratatui::Frame, area: Rect, state: &AppState, warning: Option<&str>) {
    let theme = &state.theme;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(2)])
        .split(area);

    let input_block = Block::default()
        .title(" Paste a link ")
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.surface));

    let display_text = if state.url_input.is_empty() {
        Span::styled("https://youtube.com/watch?v=…", Style::default().fg(theme.secondary))
    } else {
        Span::styled(&state.url_input, Style::default().fg(theme.primary))
    };

    let p = Paragraph::new(Line::from(vec![
        Span::styled("❯ ", Style::default().fg(theme.accent_cyan)),
        display_text,
    ]))
    .block(input_block);

    frame.render_widget(p, chunks[0]);

    let hint_text = if let Some(w) = warning {
        Line::from(Span::styled(format!("✗ {}", w), Style::default().fg(Color::Red)))
    } else if let Some(clip) = &state.clipboard_url {
        if state.url_input.is_empty() {
            Line::from(Span::styled(
                "link in your clipboard — ⇥ to paste it",
                Style::default().fg(theme.secondary),
            ))
        } else if &state.url_input == clip {
            Line::from(Span::styled(
                "from your clipboard — ↵ to yoink it",
                Style::default().fg(theme.accent_emerald),
            ))
        } else {
            Line::default()
        }
    } else {
        Line::default()
    };

    let hint_widget = Paragraph::new(hint_text).alignment(Alignment::Center);
    frame.render_widget(hint_widget, chunks[1]);
}

fn render_probing_stage(frame: &mut ratatui::Frame, area: Rect, state: &AppState, status: &str, tick: usize) {
    let theme = &state.theme;
    let spinner = SPINNER_FRAMES[tick % SPINNER_FRAMES.len()];
    let platform_label = state
        .probe_result
        .as_ref()
        .map(|p| p.platform.label.as_str())
        .unwrap_or("Probing");

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(2)])
        .split(area);

    let block = Block::default()
        .title(format!(" {} ", platform_label))
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.surface));

    let p = Paragraph::new(Line::from(Span::styled(
        truncate(&state.url_input, 50),
        Style::default().fg(theme.secondary),
    )))
    .block(block);
    frame.render_widget(p, chunks[0]);

    let status_line = Line::from(vec![
        Span::styled(format!("{} ", spinner), Style::default().fg(theme.accent_cyan)),
        Span::styled(status, Style::default().fg(theme.secondary)),
    ]);
    frame.render_widget(Paragraph::new(status_line).alignment(Alignment::Center), chunks[1]);
}

fn render_picking_stage(frame: &mut ratatui::Frame, area: Rect, state: &AppState, selected: usize) {
    let theme = &state.theme;
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    // Left: Metadata
    if let Some(probe) = &state.probe_result {
        let mut meta_lines = Vec::new();
        for line in wrap_text(&probe.info.title, 26) {
            meta_lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(theme.primary).add_modifier(Modifier::BOLD),
            )));
        }
        meta_lines.push(Line::default());

        let mut sub = format!("▸ {}", probe.platform.label);
        if let Some(dur) = probe.info.duration {
            sub.push_str(&format!(" · {}", format_duration(dur)));
        }
        if let Some(uploader) = &probe.info.uploader {
            sub.push_str(&format!(" · {}", truncate(uploader, 16)));
        }
        meta_lines.push(Line::from(Span::styled(sub, Style::default().fg(theme.secondary))));

        let meta_widget = Paragraph::new(meta_lines);
        frame.render_widget(meta_widget, cols[0]);

        // Right: Choices
        let block = Block::default()
            .title(" Download ")
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.surface));

        let items: Vec<ListItem> = probe
            .choices
            .iter()
            .enumerate()
            .map(|(idx, c)| {
                let is_sel = idx == selected;
                let icon = if c.kind == "audio" { "♪ " } else { "▶ " };
                let prefix = if is_sel { "❯ " } else { "  " };

                let style = if is_sel {
                    Style::default().fg(theme.accent_emerald).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.primary)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(prefix, Style::default().fg(theme.accent_emerald)),
                    Span::styled(icon, Style::default().fg(theme.secondary)),
                    Span::styled(&c.label, style),
                ]))
            })
            .collect();

        let list = List::new(items).block(block);
        frame.render_widget(list, cols[1]);
    }
}

fn render_downloading_stage(
    frame: &mut ratatui::Frame,
    area: Rect,
    state: &AppState,
    choice: &yoink_core::format::DownloadChoice,
    progress: Option<&ProgressEvent>,
    processing: bool,
    tick: usize,
) {
    let theme = &state.theme;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2), Constraint::Length(2)])
        .split(area);

    let title = state
        .probe_result
        .as_ref()
        .map(|p| truncate(&p.info.title, 36))
        .unwrap_or_else(|| "Video".into());

    let header_line = Line::from(vec![
        Span::styled(format!("{} · ", title), Style::default().fg(theme.primary)),
        Span::styled(&choice.label, Style::default().fg(theme.secondary)),
    ]);
    frame.render_widget(Paragraph::new(header_line).alignment(Alignment::Center), chunks[0]);

    let spinner = SPINNER_FRAMES[tick % SPINNER_FRAMES.len()];

    let (percent, meta_str) = if processing {
        (
            1.0,
            format!("{} processing…", spinner),
        )
    } else if let Some(ProgressEvent::Downloading {
        downloaded_bytes,
        total_bytes,
        speed_bps,
        eta_seconds,
        part,
        total_parts,
    }) = progress
    {
        let pct = if let Some(tot) = total_bytes {
            if *tot > 0 {
                (*downloaded_bytes as f64 / *tot as f64).min(1.0)
            } else {
                0.0
            }
        } else {
            0.0
        };

        let part_str = if *total_parts > 1 {
            format!("part {}/{}  ", part + 1, total_parts)
        } else {
            String::new()
        };

        let spd = speed_bps.map(format_speed).unwrap_or_default();
        let eta = eta_seconds.map(|e| format!("{} left", format_eta(e))).unwrap_or_default();
        let meta = format!("{}{:>10}  {:<12}", part_str, spd, eta);
        (pct, meta)
    } else {
        (0.0, format!("{} starting download…", spinner))
    };

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::NONE))
        .gauge_style(Style::default().fg(theme.accent_emerald).bg(theme.border))
        .ratio(percent);
    frame.render_widget(gauge, chunks[1]);

    let meta_p = Paragraph::new(Line::from(Span::styled(meta_str, Style::default().fg(theme.secondary))))
        .alignment(Alignment::Center);
    frame.render_widget(meta_p, chunks[2]);
}

fn render_done_stage(frame: &mut ratatui::Frame, area: Rect, state: &AppState, filepath: &PathBuf) {
    let theme = &state.theme;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2), Constraint::Length(3)])
        .split(area);

    let done_line = Line::from(vec![
        Span::styled("✓ yoinked! ", Style::default().fg(theme.accent_emerald).add_modifier(Modifier::BOLD)),
        Span::styled("find your file in:", Style::default().fg(theme.primary)),
    ]);
    frame.render_widget(Paragraph::new(done_line).alignment(Alignment::Center), chunks[0]);

    let homedir = std::env::var("HOME").unwrap_or_default();
    let pretty_path = shorten_path(&filepath.to_string_lossy(), &homedir, 50);
    let path_line = Line::from(Span::styled(pretty_path, Style::default().fg(theme.accent_cyan)));
    frame.render_widget(Paragraph::new(path_line).alignment(Alignment::Center), chunks[1]);

    let button_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(theme.accent_emerald))
        .style(Style::default().bg(theme.surface));

    let btn = Paragraph::new(Line::from(Span::styled(
        "↵ yoink another",
        Style::default().fg(theme.accent_emerald).add_modifier(Modifier::BOLD),
    )))
    .alignment(Alignment::Center)
    .block(button_block);

    frame.render_widget(btn, chunks[2]);
}

fn render_error_stage(frame: &mut ratatui::Frame, area: Rect, state: &AppState, message: &str) {
    let theme = &state.theme;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(2)])
        .split(area);

    let err_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::Red))
        .style(Style::default().bg(theme.surface));

    let p = Paragraph::new(Line::from(vec![
        Span::styled("✗ ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::styled(message, Style::default().fg(theme.primary)),
    ]))
    .block(err_block);
    frame.render_widget(p, chunks[0]);

    let btn = Paragraph::new(Line::from(Span::styled(
        "[↵ try again]",
        Style::default().fg(theme.secondary),
    )))
    .alignment(Alignment::Center);
    frame.render_widget(btn, chunks[1]);
}

fn render_footer(frame: &mut ratatui::Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let hints = match &state.phase {
        Phase::Input { .. } => vec![("↵", "yoink"), ("^c", "quit")],
        Phase::Probing { .. } => vec![("esc", "cancel"), ("^c", "quit")],
        Phase::Picking { .. } => vec![("↑↓", "choose"), ("↵", "yoink"), ("esc", "back"), ("^c", "quit")],
        Phase::Downloading { .. } => vec![("esc", "cancel"), ("^c", "quit")],
        Phase::Done { .. } => vec![("↵", "yoink another"), ("^c", "quit")],
        Phase::Error { .. } => vec![("↵", "try again"), ("^c", "quit")],
    };

    let theme_mode_str = match theme.mode {
        crate::theme::ThemeMode::Auto => "auto",
        crate::theme::ThemeMode::Dark => "dark",
        crate::theme::ThemeMode::Light => "light",
    };

    let mut spans = Vec::new();
    for (i, (key, label)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ·  ", Style::default().fg(theme.border)));
        }
        spans.push(Span::styled(*key, Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)));
        spans.push(Span::styled(format!(" {}", label), Style::default().fg(theme.secondary)));
    }
    spans.push(Span::styled("  ·  ", Style::default().fg(theme.border)));
    spans.push(Span::styled("^t", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)));
    spans.push(Span::styled(format!(" theme:{}", theme_mode_str), Style::default().fg(theme.secondary)));

    let footer = Paragraph::new(Line::from(spans)).alignment(Alignment::Center);
    frame.render_widget(footer, area);
}
