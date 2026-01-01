use anyhow::Result;
use clap::Parser;
use rusty_golf_setup::config::{AppMode, Cli, load_config};
use rusty_golf_setup::repl::run_new_event_repl;
use rusty_golf_setup::seed_kv_from_eup;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match load_config(cli)? {
        AppMode::Seed(config) => seed_kv_from_eup(&config),
        AppMode::NewEvent {
            eup_json,
            output_json,
        } => run_new_event_repl(eup_json, output_json),
    }
}
