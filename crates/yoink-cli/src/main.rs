use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about = "yoink - A native Rust spiritual fork of yoinks")]
pub struct Args {
    pub url: Option<String>,

    #[arg(short, long)]
    pub audio: bool,

    #[arg(short, long)]
    pub quality: Option<String>,

    #[arg(short, long)]
    pub out: Option<PathBuf>,

    #[arg(long)]
    pub playlist: Option<String>,

    #[arg(long)]
    pub json: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    
    // TUI Mode detection
    let is_tty = atty::is(atty::Stream::Stdout);
    let interactive = is_tty && args.url.is_none() && !args.json;

    if interactive {
        run_tui()?;
    } else {
        println!("yoink CLI: {:?}", args);
    }
    
    Ok(())
}

fn run_tui() -> anyhow::Result<()> {
    use ratatui::{backend::CrosstermBackend, Terminal};
    use crossterm::{terminal::{enable_raw_mode, EnterAlternateScreen}, execute};
    use std::io::stdout;

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Simple TUI loop (for now)
    loop {
        terminal.draw(|f| {
            f.render_widget(
                ratatui::widgets::Paragraph::new("Yoink TUI (KAT Square Minimalist)"),
                f.area()
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

    Ok(())
}
