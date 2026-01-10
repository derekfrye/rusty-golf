use anyhow::Result;
use clap::Parser;
use rusty_golf_setup::config::{AppMode, Cli, load_config};
use rusty_golf_setup::repl::{run_new_event_one_shot, run_new_event_repl};
use rusty_golf_setup::seed_kv_from_eup;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match load_config(&cli)? {
        AppMode::Seed(config) => seed_kv_from_eup(&config),
        AppMode::NewEvent {
            eup_json,
            output_json,
            one_shot,
            event_id,
            golfers_by_bettor,
        } => {
            if one_shot {
                let event_id = event_id.expect("event_id required for one-shot");
                let output_json = output_json.expect("output_json required for one-shot");
                let golfers_by_bettor =
                    golfers_by_bettor.expect("golfers_by_bettor required for one-shot");
                run_new_event_one_shot(eup_json, &output_json, event_id, golfers_by_bettor)
            } else {
                run_new_event_repl(eup_json, output_json)
            }
        }
    }
}
