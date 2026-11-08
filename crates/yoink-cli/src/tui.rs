use ratatui::{backend::CrosstermBackend, Terminal};
use crossterm::{terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}, execute};
use std::io::stdout;

pub fn run_tui() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Simple TUI loop: render KAT Square Minimalist theme
    loop {
        terminal.draw(|f| {
            let area = f.area();
            f.render_widget(
                ratatui::widgets::Paragraph::new("Yoink TUI (KAT Square Minimalist)")
                    .block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL).border_type(ratatui::widgets::BorderType::Plain)),
                area
            );
        })?;
        
        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                if key.code == crossterm::event::KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
