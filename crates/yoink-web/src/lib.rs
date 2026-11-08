pub mod config;
pub mod jobs;
pub mod sweeper;

use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../web/"]
pub struct WebAssets;
