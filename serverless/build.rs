use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-env-changed=SOURCE_VERSION");
    println!("cargo:rerun-if-env-changed=GIT_COMMIT");

    let git_sha = std::env::var("SOURCE_VERSION")
        .ok()
        .or_else(|| std::env::var("GIT_COMMIT").ok())
        .or_else(read_git_sha)
        .unwrap_or_else(|| "unknown".to_string());

    let short_git_sha = git_sha.chars().take(12).collect::<String>();

    println!("cargo:rustc-env=RUSTY_GOLF_GIT_SHA={git_sha}");
    println!("cargo:rustc-env=RUSTY_GOLF_GIT_SHA_SHORT={short_git_sha}");
}

fn read_git_sha() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let sha = String::from_utf8(output.stdout).ok()?;
    let sha = sha.trim();
    if sha.is_empty() {
        return None;
    }
    Some(sha.to_string())
}
