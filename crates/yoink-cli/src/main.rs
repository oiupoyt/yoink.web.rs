use clap::Parser;
use std::path::PathBuf;
use yoink_core::YtDlpEngine;
use yoink_core::traits::Downloader;

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    
    if args.url.is_some() {
        run_flag_mode(args).await?;
    } else {
        run_tui()?;
    }
    
    Ok(())
}

async fn run_flag_mode(args: Args) -> anyhow::Result<()> {
    let url = args.url.clone().unwrap();
    let engine = YtDlpEngine::new().await?;
    let probe_res = engine.probe(&url).await?;
    
    // Simple logic to pick the best format
    let choice = if args.audio {
        probe_res.choices.iter().find(|c| c.kind == "audio").unwrap_or(&probe_res.choices[0])
    } else {
        &probe_res.choices[0]
    };

    let req = yoink_core::traits::DownloadRequest {
        url,
        choice: choice.clone(),
        out_dir: args.out.unwrap_or_else(|| PathBuf::from(".")),
        info_json_path: probe_res.info_json_path,
    };

    let (mut rx, handle) = engine.download(req).await?;
    
    if args.json {
        while let Ok(event) = rx.recv().await {
            println!("{}", serde_json::to_string(&event)?);
        }
    } else {
        while let Ok(event) = rx.recv().await {
            println!("{:?}", event);
        }
    }

    handle.await??;
    Ok(())
}

fn run_tui() -> anyhow::Result<()> {
    Ok(())
}
