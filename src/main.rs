mod api;
mod app;
mod config;
mod hosts;
mod i18n;
mod ping;
mod tui;
mod ui;

use crate::app::App;
use crate::config::{
    GameMode, SortOrder, get_config_path, load_config, migrate_json_if_needed, save_config,
};
use clap::{Parser, Subcommand};
use std::process;

#[derive(Parser)]
#[command(name = "dbdq")]
#[command(about = "Dead by Daylight Matchmaking Queue Times & Region Locker TUI", long_about = None)]
struct Cli {
    #[arg(short, long, value_parser = ["survivor", "killer", "ping", "priority", "default"], help = "Sort output by column/rules (persists in config)")]
    sort: Option<String>,

    #[arg(short, long, value_parser = ["standard", "event"], help = "Filter rows by Mode")]
    mode: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Lock regions (blocking all others)")]
    Lock {
        #[arg(num_args = 0.., help = "Regions to whitelist (leave empty for interactive menu)")]
        regions: Vec<String>,
    },
    #[command(about = "Unlock all regions")]
    Unlock,
}

fn main() {
    let args = Cli::parse();

    let config_path = get_config_path();
    migrate_json_if_needed(&config_path);

    let mut config = load_config(&config_path);
    let mut config_changed = false;

    if let Some(ref s) = args.sort {
        let sort_val = match s.to_lowercase().as_str() {
            "killer" => SortOrder::Killer,
            "survivor" => SortOrder::Survivor,
            "ping" | "priority" => SortOrder::Ping,
            _ => SortOrder::Default,
        };
        config.sort = sort_val;
        config_changed = true;
    }

    if let Some(ref m) = args.mode {
        let mode_val = match m.to_lowercase().as_str() {
            "event" => GameMode::Event,
            _ => GameMode::Standard,
        };
        config.mode = mode_val;
        config_changed = true;
    }

    if config_changed && let Err(e) = save_config(&config_path, &config) {
        eprintln!("Failed to save config: {}", e);
    }

    // Process subcommands
    if let Some(ref cmd) = args.command {
        match cmd {
            Commands::Lock { regions } => {
                let resolved_regions = if regions.is_empty() {
                    match hosts::interactive_lock_menu(&config.locked) {
                        Some(regs) => regs,
                        None => process::exit(0),
                    }
                } else {
                    crate::api::resolve_to_aws_codes(regions)
                };

                config.locked = resolved_regions.clone();
                if let Err(e) = save_config(&config_path, &config) {
                    eprintln!("Failed to save config: {}", e);
                }
                hosts::update_hosts(Some(&resolved_regions), true);
                process::exit(0);
            }
            Commands::Unlock => {
                config.locked = vec![];
                if let Err(e) = save_config(&config_path, &config) {
                    eprintln!("Failed to save config: {}", e);
                }
                hosts::update_hosts(None, true);
                process::exit(0);
            }
        }
    }

    let mut app = App::new(
        config.sort,
        config.mode,
        config.locked,
        config.lang,
        config.api_url,
    );

    // Initial fetch to show data immediately
    if let Ok((queues, updated)) = api::fetch_queue_times() {
        app.queues = queues;
        app.api_last_updated = updated;
        app.is_fetching = false;
        app.clamp_selection();
    }

    if let Err(e) = tui::run_app(app, config_path) {
        eprintln!("Error running TUI: {}", e);
        process::exit(1);
    }
}
