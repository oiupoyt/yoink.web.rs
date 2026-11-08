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
    println!("yoink CLI: {:?}", args);
    Ok(())
}
