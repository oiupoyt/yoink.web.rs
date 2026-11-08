use clap::Parser;
use std::path::PathBuf;
use yoink_cli::tui::run_interactive;
use yoink_core::traits::{DownloadRequest, Downloader};
use yoink_core::YtDlpEngine;

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
    pub tui: bool,

    #[arg(long)]
    pub json: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let is_tty = atty::is(atty::Stream::Stdout) && atty::is(atty::Stream::Stdin);

    if (args.url.is_none() && is_tty && !args.json) || args.tui {
        let outcome = run_interactive(args.url.clone(), args.out).await?;
        if let Some(dest) = outcome {
            println!("✓ yoinked → {}", dest.display());
        }
    } else if let Some(url) = args.url.clone() {
        run_flag_mode(args, url).await?;
    } else {
        // Run interactive even if piped or requested
        let outcome = run_interactive(None, args.out).await?;
        if let Some(dest) = outcome {
            println!("✓ yoinked → {}", dest.display());
        }
    }

    Ok(())
}

async fn run_flag_mode(args: Args, url: String) -> anyhow::Result<()> {
    let engine = YtDlpEngine::new().await?;
    let probe_res = engine.probe(&url).await?;

    let choice = if args.audio {
        probe_res
            .choices
            .iter()
            .find(|c| c.kind == "audio")
            .unwrap_or(&probe_res.choices[0])
    } else if let Some(q) = &args.quality {
        probe_res
            .choices
            .iter()
            .find(|c| c.label.to_lowercase().contains(&q.to_lowercase()))
            .unwrap_or(&probe_res.choices[0])
    } else {
        &probe_res.choices[0]
    };

    let req = DownloadRequest {
        url,
        choice: choice.clone(),
        out_dir: args.out.unwrap_or_else(|| {
            directories::UserDirs::new()
                .and_then(|u| u.download_dir().map(|d| d.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from("."))
        }),
        info_json_path: probe_res.info_json_path,
    };

    let (mut rx, handle) = engine.download(req).await?;

    while let Ok(event) = rx.recv().await {
        if args.json {
            println!("{}", serde_json::to_string(&event)?);
        } else {
            match event {
                yoink_core::progress::ProgressEvent::Downloading {
                    downloaded_bytes,
                    total_bytes,
                    speed_bps,
                    eta_seconds,
                    part,
                    total_parts,
                } => {
                    let total_str = total_bytes
                        .map(yoink_core::format::format_bytes)
                        .unwrap_or_else(|| "? MB".to_string());
                    let spd_str = speed_bps
                        .map(yoink_core::format::format_speed)
                        .unwrap_or_default();
                    let eta_str = eta_seconds
                        .map(yoink_core::format::format_eta)
                        .unwrap_or_default();
                    let pct = if let Some(tot) = total_bytes {
                        if tot > 0 {
                            (downloaded_bytes * 100 / tot).min(100)
                        } else {
                            0
                        }
                    } else {
                        0
                    };
                    let part_str = if total_parts > 1 {
                        format!(" [part {}/{}]", part + 1, total_parts)
                    } else {
                        String::new()
                    };
                    eprint!(
                        "\ryoink: {}% of {} at {} ({} left){}",
                        pct, total_str, spd_str, eta_str, part_str
                    );
                }
                yoink_core::progress::ProgressEvent::Processing { step } => {
                    eprint!("\ryoink: processing ({})…                     ", step);
                }
                yoink_core::progress::ProgressEvent::Finished { filepath } => {
                    eprintln!();
                    println!("✓ yoinked → {}", filepath.display());
                }
                yoink_core::progress::ProgressEvent::Error { message } => {
                    eprintln!();
                    eprintln!("✗ error: {}", message);
                }
                _ => {}
            }
        }
    }

    let final_path = handle.await??;
    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "type": "Finished",
                "data": { "filepath": final_path.to_string_lossy() }
            })
        );
    }

    Ok(())
}
