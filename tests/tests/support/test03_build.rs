use std::error::Error;
use std::path::Path;
use std::process::Command;

use crate::common::serverless::types::WranglerPaths;

pub(super) fn build_local(
    workspace_root: &Path,
    wrangler_paths: &WranglerPaths,
) -> Result<(), Box<dyn Error>> {
    run_script(
        &workspace_root.join("serverless/scripts/build_local.sh"),
        &[
            (
                "CONFIG_PATH",
                wrangler_paths.config.to_str().unwrap_or_default(),
            ),
            (
                "WRANGLER_LOG_DIR",
                wrangler_paths.log_dir.to_str().unwrap_or_default(),
            ),
            (
                "XDG_CONFIG_HOME",
                wrangler_paths.config_dir.to_str().unwrap_or_default(),
            ),
        ],
        workspace_root,
    )
}

fn run_script(script_path: &Path, envs: &[(&str, &str)], cwd: &Path) -> Result<(), Box<dyn Error>> {
    let output = Command::new("bash")
        .arg(script_path)
        .envs(envs.iter().copied())
        .current_dir(cwd)
        .output()?;
    if output.status.success() {
        return Ok(());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "Script failed: {}\nstdout:\n{}\nstderr:\n{}",
        script_path.display(),
        stdout,
        stderr
    )
    .into())
}
