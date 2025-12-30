use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

#[derive(Debug)]
pub struct SeedOptions {
    pub eup_json: PathBuf,
    pub kv_env: String,
    pub event_id: Option<i64>,
    pub refresh_from_espn: i64,
    pub wrangler_config: PathBuf,
    pub wrangler_env: String,
    pub wrangler_kv_flags: Vec<String>,
    pub wrangler_log_dir: Option<PathBuf>,
    pub wrangler_config_dir: Option<PathBuf>,
    pub kv_binding: Option<String>,
}

pub fn seed_kv_from_eup(options: SeedOptions) -> Result<()> {
    if options.kv_env != "dev" && options.kv_env != "prod" {
        bail!("kv_env must be 'dev' or 'prod'");
    }

    let kv_namespace_id = if options.kv_binding.is_some() {
        None
    } else {
        Some(load_kv_namespace_id(
            &options.wrangler_config,
            &options.kv_env,
        )?)
    };

    let events = load_events(&options.eup_json, options.event_id)?;
    if events.is_empty() {
        bail!("no events found to seed");
    }

    let temp_dir = TempDir::new().context("create temp dir")?;
    for event in &events {
        write_event_files(event, options.refresh_from_espn, temp_dir.path())?;
    }

    for event in &events {
        seed_event_kv(
            event.event,
            temp_dir.path(),
            options.kv_binding.as_deref(),
            kv_namespace_id.as_deref(),
            &options.wrangler_kv_flags,
            options.wrangler_log_dir.as_deref(),
            options.wrangler_config_dir.as_deref(),
        )?;
        println!("Seeded KV for event {}.", event.event);
    }

    println!("KV seed complete for env {}.", options.wrangler_env);
    Ok(())
}

#[derive(Debug, Deserialize)]
struct WranglerConfig {
    env: Option<HashMap<String, WranglerEnv>>,
}

#[derive(Debug, Deserialize)]
struct WranglerEnv {
    kv_namespaces: Option<Vec<KvNamespace>>,
}

#[derive(Debug, Deserialize)]
struct KvNamespace {
    id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EupEvent {
    event: i64,
    name: String,
    score_view_step_factor: serde_json::Value,
    data_to_fill_if_event_and_year_missing: Vec<EupDataFill>,
}

#[derive(Debug, Deserialize)]
struct EupDataFill {
    golfers: Vec<EupGolfer>,
    event_user_player: Vec<EupEventUserPlayer>,
}

#[derive(Debug, Deserialize)]
struct EupGolfer {
    espn_id: i64,
    name: String,
}

#[derive(Debug, Deserialize)]
struct EupEventUserPlayer {
    bettor: String,
    golfer_espn_id: i64,
    score_view_step_factor: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct EventDetails<'a> {
    event_name: &'a str,
    score_view_step_factor: &'a serde_json::Value,
    refresh_from_espn: i64,
}

#[derive(Debug, Serialize)]
struct GolferOut<'a> {
    eup_id: i64,
    espn_id: i64,
    golfer_name: &'a str,
    bettor_name: &'a str,
    group: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    score_view_step_factor: Option<&'a serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct PlayerFactor<'a> {
    golfer_espn_id: i64,
    bettor_name: &'a str,
    step_factor: &'a serde_json::Value,
}

fn load_kv_namespace_id(config_path: &Path, kv_env: &str) -> Result<String> {
    let contents = fs::read_to_string(config_path)
        .with_context(|| format!("read wrangler config {}", config_path.display()))?;
    let config: WranglerConfig = toml::from_str(&contents)
        .with_context(|| format!("parse wrangler config {}", config_path.display()))?;

    let envs = config
        .env
        .ok_or_else(|| anyhow!("no env section in {}", config_path.display()))?;
    let env = envs
        .get(kv_env)
        .ok_or_else(|| anyhow!("no env '{}' in {}", kv_env, config_path.display()))?;
    let namespaces = env.kv_namespaces.as_ref().ok_or_else(|| {
        anyhow!(
            "no kv_namespaces found for env '{}' in {}",
            kv_env,
            config_path.display()
        )
    })?;
    let namespace = namespaces
        .first()
        .ok_or_else(|| anyhow!("kv_namespaces empty for env '{}'", kv_env))?;
    let id = namespace.id.as_ref().ok_or_else(|| {
        anyhow!(
            "missing kv_namespaces[0].id for env '{}' in {}",
            kv_env,
            config_path.display()
        )
    })?;
    Ok(id.clone())
}

fn load_events(eup_path: &Path, event_id_filter: Option<i64>) -> Result<Vec<EupEvent>> {
    if !eup_path.is_file() {
        bail!("missing file {}", eup_path.display());
    }
    let contents =
        fs::read_to_string(eup_path).with_context(|| format!("read {}", eup_path.display()))?;
    let mut events: Vec<EupEvent> = serde_json::from_str(&contents)
        .with_context(|| format!("parse {}", eup_path.display()))?;

    if let Some(event_id) = event_id_filter {
        events.retain(|event| event.event == event_id);
    }
    Ok(events)
}

fn write_event_files(event: &EupEvent, refresh_from_espn: i64, root: &Path) -> Result<()> {
    let data_to_fill = event
        .data_to_fill_if_event_and_year_missing
        .get(0)
        .ok_or_else(|| anyhow!("no data_to_fill_if_event_and_year_missing for {}", event.event))?;

    let event_dir = root.join(event.event.to_string());
    fs::create_dir_all(&event_dir)
        .with_context(|| format!("create {}", event_dir.display()))?;

    let details = EventDetails {
        event_name: &event.name,
        score_view_step_factor: &event.score_view_step_factor,
        refresh_from_espn,
    };
    write_json(event_dir.join("event_details.json"), &details)?;

    let mut golfers_by_id = HashMap::new();
    for golfer in &data_to_fill.golfers {
        golfers_by_id.insert(golfer.espn_id, golfer.name.as_str());
    }

    let mut bettor_counts: HashMap<&str, usize> = HashMap::new();
    let mut golfers_out = Vec::new();
    let mut eup_id = 1_i64;
    for entry in &data_to_fill.event_user_player {
        let count = bettor_counts.entry(entry.bettor.as_str()).or_insert(0);
        *count += 1;

        let golfer_name = golfers_by_id.get(&entry.golfer_espn_id).ok_or_else(|| {
            anyhow!(
                "missing golfer_espn_id {} in golfers list for event {}",
                entry.golfer_espn_id,
                event.event
            )
        })?;

        golfers_out.push(GolferOut {
            eup_id,
            espn_id: entry.golfer_espn_id,
            golfer_name,
            bettor_name: entry.bettor.as_str(),
            group: *count,
            score_view_step_factor: entry.score_view_step_factor.as_ref(),
        });
        eup_id += 1;
    }

    write_json(event_dir.join("golfers.json"), &golfers_out)?;

    let player_factors: Vec<PlayerFactor<'_>> = data_to_fill
        .event_user_player
        .iter()
        .filter_map(|entry| {
            entry.score_view_step_factor.as_ref().map(|factor| PlayerFactor {
                golfer_espn_id: entry.golfer_espn_id,
                bettor_name: entry.bettor.as_str(),
                step_factor: factor,
            })
        })
        .collect();
    write_json(event_dir.join("player_factors.json"), &player_factors)?;

    Ok(())
}

fn write_json<T: Serialize>(path: PathBuf, data: &T) -> Result<()> {
    let file = fs::File::create(&path).with_context(|| format!("create {}", path.display()))?;
    serde_json::to_writer(&file, data).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn seed_event_kv(
    event_id: i64,
    root: &Path,
    kv_binding: Option<&str>,
    namespace_id: Option<&str>,
    wrangler_kv_flags: &[String],
    wrangler_log_dir: Option<&Path>,
    wrangler_config_dir: Option<&Path>,
) -> Result<()> {
    let event_dir = root.join(event_id.to_string());
    let entries = [
        (
            format!("event:{event_id}:details"),
            event_dir.join("event_details.json"),
        ),
        (
            format!("event:{event_id}:golfers"),
            event_dir.join("golfers.json"),
        ),
        (
            format!("event:{event_id}:player_factors"),
            event_dir.join("player_factors.json"),
        ),
    ];

    for (key, path) in entries {
        let mut command = Command::new("wrangler");
        command
            .arg("kv")
            .arg("key")
            .arg("put")
            .args(wrangler_kv_flags);

        if let Some(binding) = kv_binding {
            command.arg("--binding").arg(binding);
        } else if let Some(id) = namespace_id {
            command.arg("--namespace-id").arg(id);
        } else {
            bail!("missing kv binding or namespace id");
        }

        command.arg(key).arg("--path").arg(path);

        if let Some(dir) = wrangler_log_dir {
            command.env("WRANGLER_LOG_DIR", dir);
        }
        if let Some(dir) = wrangler_config_dir {
            command.env("XDG_CONFIG_HOME", dir);
        }

        let status = command.status().context("run wrangler kv key put")?;
        if !status.success() {
            bail!("wrangler failed with status {}", status);
        }
    }

    Ok(())
}
