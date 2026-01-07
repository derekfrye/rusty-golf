use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

const KV_BINDING: &str = "djf_rusty_golf_kv";
const R2_BUCKET: &str = "djf-rusty-golf-dev-preview";

#[derive(Clone)]
pub struct WranglerPaths {
    pub config: PathBuf,
    pub log_dir: PathBuf,
    pub config_dir: PathBuf,
}

#[derive(Clone)]
pub struct WranglerFlags {
    pub kv_flags: String,
    pub r2_flags: String,
    pub kv_flag_list: Vec<String>,
}

pub struct CleanupGuard {
    workspace_root: PathBuf,
    wrangler_paths: WranglerPaths,
    wrangler_flags: WranglerFlags,
    event_ids: Vec<i64>,
    include_auth_tokens: bool,
}

impl CleanupGuard {
    pub fn new(
        workspace_root: PathBuf,
        wrangler_paths: WranglerPaths,
        wrangler_flags: WranglerFlags,
        event_ids: Vec<i64>,
        include_auth_tokens: bool,
    ) -> Self {
        Self {
            workspace_root,
            wrangler_paths,
            wrangler_flags,
            event_ids,
            include_auth_tokens,
        }
    }
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        if let Err(err) = cleanup_events(
            &self.workspace_root,
            &self.wrangler_paths,
            &self.wrangler_flags,
            &self.event_ids,
            self.include_auth_tokens,
        ) {
            eprintln!("Cleanup failed: {err}");
        }
    }
}

pub fn cleanup_events(
    workspace_root: &Path,
    wrangler_paths: &WranglerPaths,
    wrangler_flags: &WranglerFlags,
    event_ids: &[i64],
    include_auth_tokens: bool,
) -> Result<(), Box<dyn Error>> {
    for event_id in event_ids {
        cleanup_kv_for_event(
            wrangler_paths,
            wrangler_flags,
            *event_id,
            include_auth_tokens,
        )?;
        cleanup_r2_for_event(
            workspace_root,
            wrangler_paths,
            wrangler_flags,
            *event_id,
        )?;
    }
    Ok(())
}

fn cleanup_kv_for_event(
    wrangler_paths: &WranglerPaths,
    wrangler_flags: &WranglerFlags,
    event_id: i64,
    include_auth_tokens: bool,
) -> Result<(), Box<dyn Error>> {
    let mut keys = vec![
        format!("event:{event_id}:details"),
        format!("event:{event_id}:golfers"),
        format!("event:{event_id}:player_factors"),
        format!("event:{event_id}:details:seeded_at"),
        format!("event:{event_id}:golfers:seeded_at"),
        format!("event:{event_id}:player_factors:seeded_at"),
    ];
    if include_auth_tokens {
        keys.push(format!("event:{event_id}:auth_tokens"));
    }

    for key in keys {
        let mut command = Command::new("wrangler");
        command
            .arg("kv")
            .arg("key")
            .arg("delete")
            .args(&wrangler_flags.kv_flag_list)
            .arg("--binding")
            .arg(KV_BINDING)
            .arg("--force")
            .arg(&key);

        let wrangler_log_dir_str = wrangler_paths.log_dir.to_str().unwrap_or_default();
        let wrangler_config_dir_str = wrangler_paths.config_dir.to_str().unwrap_or_default();
        command
            .env("WRANGLER_LOG_DIR", wrangler_log_dir_str)
            .env("XDG_CONFIG_HOME", wrangler_config_dir_str);

        let status = command.status()?;
        if !status.success() {
            return Err(format!("wrangler kv key delete failed for {key}").into());
        }
    }

    Ok(())
}

fn cleanup_r2_for_event(
    workspace_root: &Path,
    wrangler_paths: &WranglerPaths,
    wrangler_flags: &WranglerFlags,
    event_id: i64,
) -> Result<(), Box<dyn Error>> {
    let r2_keys = [
        format!("{R2_BUCKET}/events/{event_id}/scores.json"),
        format!("{R2_BUCKET}/cache/espn/{event_id}.json"),
    ];
    let wrangler_log_dir_str = wrangler_paths.log_dir.to_str().unwrap_or_default();
    let wrangler_config_dir_str = wrangler_paths.config_dir.to_str().unwrap_or_default();

    for key in r2_keys {
        let mut command = Command::new("wrangler");
        command
            .arg("r2")
            .arg("object")
            .arg("delete")
            .args(wrangler_flags.r2_flags.split_whitespace())
            .arg(&key)
            .current_dir(workspace_root)
            .env("WRANGLER_LOG_DIR", wrangler_log_dir_str)
            .env("XDG_CONFIG_HOME", wrangler_config_dir_str);

        let status = command.status()?;
        if !status.success() {
            return Err(format!("wrangler r2 object delete failed for {key}").into());
        }
    }
    Ok(())
}
