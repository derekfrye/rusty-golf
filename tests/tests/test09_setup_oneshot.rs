use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use rusty_golf_setup::espn::EspnClient;
use rusty_golf_setup::repl::run_new_event_one_shot_with_client;

#[path = "support/test09_support.rs"]
mod test09_support;
use test09_support::{
    FixtureEspnClient, assert_output_matches_expected, build_one_shot_inputs_from_dbprefill,
    first_two_event_ids, load_dbprefill_event,
};

#[test]
fn test09_setup_oneshot() -> Result<()> {
    let fixture_root = fixture_root();
    let dbprefill_path = fixture_root
        .join("../test05_dbprefill.json")
        .canonicalize()
        .unwrap_or_else(|_| fixture_root.join("../test05_dbprefill.json"));
    let event_ids = first_two_event_ids(&dbprefill_path)?;

    let client: Arc<dyn EspnClient> = Arc::new(FixtureEspnClient::new(fixture_root.clone()));
    for event_id in event_ids {
        let expected_entry = load_dbprefill_event(&dbprefill_path, event_id)?;
        let golfers_by_bettor =
            build_one_shot_inputs_from_dbprefill(&fixture_root, event_id, &expected_entry)?;
        let output_path = output_path(event_id);
        run_new_event_one_shot_with_client(
            Some(dbprefill_path.clone()),
            Some(&output_path),
            false,
            event_id,
            golfers_by_bettor,
            Some(Arc::clone(&client)),
        )?;
        assert_output_matches_expected(&output_path, event_id, &expected_entry)?;
        let _ = fs::remove_file(&output_path);
    }

    Ok(())
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test09")
}

fn output_path(event_id: i64) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    path.push(format!("setup_oneshot_{event_id}_{nanos}.json"));
    path
}
