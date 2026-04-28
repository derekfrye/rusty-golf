pub mod admin;
pub mod fixtures;
pub mod locks;
pub mod runtime;
pub mod types;

use std::env;
use std::error::Error;
use std::path::PathBuf;

pub fn shared_wrangler_dirs() -> Option<(PathBuf, PathBuf)> {
    let miniflare_root = PathBuf::from("/miniflare_work/.wrangler");
    let miniflare_logs = miniflare_root.join("logs");
    let miniflare_config = miniflare_root.join("config");
    if miniflare_logs.is_dir() && miniflare_config.is_dir() {
        return Some((miniflare_logs, miniflare_config));
    }
    let home = env::var("HOME").ok()?;
    let base = PathBuf::from(home).join(".local/share/rusty-golf-miniflare");
    Some((base.join("logs"), base.join("config")))
}

pub fn is_local_miniflare(url: &str) -> bool {
    let host = url
        .split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

pub fn event_id_i32(event_id: i64) -> Result<i32, Box<dyn Error>> {
    i32::try_from(event_id).map_err(|_| format!("event_id out of range: {event_id}").into())
}
