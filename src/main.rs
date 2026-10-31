mod api;
mod app;
mod cache;
mod config;
mod hosts;
mod i18n;
mod ping;
mod tui;
mod ui;

use crate::app::App;
use crate::config::{
    GameMode, SortOrder, TimeFormat, get_config_path, load_config, migrate_json_if_needed,
    save_config,
};
use clap::{Parser, Subcommand};
use std::process;

#[derive(Parser, Debug)]
#[command(name = "dbdq")]
#[command(version)]
#[command(about = "Dead by Daylight Matchmaking Queue Times & Region Locker TUI", long_about = None)]
struct Cli {
    #[arg(
        short,
        long,
        ignore_case = true,
        help = "Sort output by column/rules (persists in config)"
    )]
    sort: Option<SortOrder>,

    #[arg(
        short,
        long,
        ignore_case = true,
        help = "Filter rows by Mode"
    )]
    mode: Option<GameMode>,

    #[arg(
        short = 't',
        long = "time",
        ignore_case = true,
        help = "Time display format (exact, rounded)"
    )]
    time: Option<TimeFormat>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
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

    if let Some(sort) = args.sort {
        config.sort = sort;
        config_changed = true;
    }

    if let Some(mode) = args.mode {
        config.mode = mode;
        config_changed = true;
    }

    if let Some(time) = args.time {
        config.time_format = time;
        config_changed = true;
    }

    if config_changed
        && let Err(e) = save_config(&config_path, &config)
    {
        eprintln!("Failed to save config: {}", e);
        process::exit(1);
    }

    // Process subcommands
    if let Some(ref cmd) = args.command {
        match cmd {
            Commands::Lock { regions } => {
                let resolved_regions = if regions.is_empty() {
                    match hosts::interactive_lock_menu(&config.locked) {
                        Some(regs) => {
                            if regs.is_empty() {
                                eprintln!("Cannot lock: no regions selected.");
                                process::exit(1);
                            }
                            regs
                        }
                        None => {
                            process::exit(1);
                        }
                    }
                } else {
                    match crate::api::resolve_to_aws_codes(regions) {
                        Ok(codes) => {
                            if codes.is_empty() {
                                eprintln!("No valid regions specified to lock.");
                                process::exit(1);
                            }
                            codes
                        }
                        Err(invalid) => {
                            eprintln!("Unknown region(s): {}", invalid.join(", "));
                            process::exit(1);
                        }
                    }
                };

                if resolved_regions.is_empty() {
                    eprintln!("No valid regions specified to lock.");
                    process::exit(1);
                }

                match hosts::update_hosts(Some(&resolved_regions), true) {
                    hosts::UpdateHostsResult::Updated | hosts::UpdateHostsResult::AlreadyUpToDate => {
                        config.locked = resolved_regions;
                        if let Err(e) = save_config(&config_path, &config) {
                            eprintln!("Failed to save config: {}", e);
                            process::exit(1);
                        }
                        process::exit(0);
                    }
                    hosts::UpdateHostsResult::ElevationFailed => {
                        eprintln!("Elevation failed/cancelled.");
                        process::exit(1);
                    }
                    hosts::UpdateHostsResult::Error(e) => {
                        eprintln!("Error: {}", e);
                        process::exit(1);
                    }
                }
            }
            Commands::Unlock => {
                match hosts::update_hosts(None, true) {
                    hosts::UpdateHostsResult::Updated | hosts::UpdateHostsResult::AlreadyUpToDate => {
                        config.locked = vec![];
                        if let Err(e) = save_config(&config_path, &config) {
                            eprintln!("Failed to save config: {}", e);
                            process::exit(1);
                        }
                        process::exit(0);
                    }
                    hosts::UpdateHostsResult::ElevationFailed => {
                        eprintln!("Elevation failed/cancelled.");
                        process::exit(1);
                    }
                    hosts::UpdateHostsResult::Error(e) => {
                        eprintln!("Error: {}", e);
                        process::exit(1);
                    }
                }
            }
        }
    }

    let mut app = App::new(
        config.sort,
        config.mode,
        config.time_format,
        config.locked,
        config.lang,
        config.api_url,
    );

    let cache_path = cache::get_cache_path(&config_path);
    let ping_cache = cache::load_ping_cache(&cache_path);
    app.pings = ping_cache.pings;

    // Initial fetch to show data immediately
    if let Ok((mut queues, updated)) = api::fetch_queue_times() {
        for q in &mut queues {
            q.reformat(app.time_format);
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_version_flag() {
        let res = Cli::try_parse_from(["dbdq", "--version"]);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);

        let res_v = Cli::try_parse_from(["dbdq", "-V"]);
        assert!(res_v.is_err());
        let err_v = res_v.unwrap_err();
        assert_eq!(err_v.kind(), clap::error::ErrorKind::DisplayVersion);
    }

    #[test]
    fn test_cli_sort_order_parsing() {
        let cli = Cli::try_parse_from(["dbdq", "--sort", "killer"]).unwrap();
        assert_eq!(cli.sort, Some(SortOrder::Killer));

        let cli = Cli::try_parse_from(["dbdq", "-s", "survivor"]).unwrap();
        assert_eq!(cli.sort, Some(SortOrder::Survivor));

        let cli = Cli::try_parse_from(["dbdq", "--sort", "ping"]).unwrap();
        assert_eq!(cli.sort, Some(SortOrder::Ping));

        let cli = Cli::try_parse_from(["dbdq", "--sort", "priority"]).unwrap();
        assert_eq!(cli.sort, Some(SortOrder::Ping));

        let cli = Cli::try_parse_from(["dbdq", "--sort", "default"]).unwrap();
        assert_eq!(cli.sort, Some(SortOrder::Default));

        // Case-insensitivity check
        let cli = Cli::try_parse_from(["dbdq", "--sort", "KILLER"]).unwrap();
        assert_eq!(cli.sort, Some(SortOrder::Killer));

        // Invalid sort order rejected
        let res = Cli::try_parse_from(["dbdq", "--sort", "invalid_sort"]);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), clap::error::ErrorKind::InvalidValue);
    }

    #[test]
    fn test_cli_game_mode_parsing() {
        let cli = Cli::try_parse_from(["dbdq", "--mode", "standard"]).unwrap();
        assert_eq!(cli.mode, Some(GameMode::Standard));

        let cli = Cli::try_parse_from(["dbdq", "-m", "event"]).unwrap();
        assert_eq!(cli.mode, Some(GameMode::Event));

        // Legacy "both" alias check
        let cli = Cli::try_parse_from(["dbdq", "--mode", "both"]).unwrap();
        assert_eq!(cli.mode, Some(GameMode::Standard));

        // Case-insensitivity check
        let cli = Cli::try_parse_from(["dbdq", "--mode", "EVENT"]).unwrap();
        assert_eq!(cli.mode, Some(GameMode::Event));

        // Invalid mode rejected
        let res = Cli::try_parse_from(["dbdq", "--mode", "ranked"]);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), clap::error::ErrorKind::InvalidValue);
    }

    #[test]
    fn test_cli_time_format_parsing() {
        let cli = Cli::try_parse_from(["dbdq", "--time", "exact"]).unwrap();
        assert_eq!(cli.time, Some(TimeFormat::Exact));

        let cli = Cli::try_parse_from(["dbdq", "-t", "rounded"]).unwrap();
        assert_eq!(cli.time, Some(TimeFormat::Rounded));

        // Case-insensitivity check
        let cli = Cli::try_parse_from(["dbdq", "-t", "Exact"]).unwrap();
        assert_eq!(cli.time, Some(TimeFormat::Exact));

        // Invalid time format rejected
        let res = Cli::try_parse_from(["dbdq", "-t", "12h"]);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), clap::error::ErrorKind::InvalidValue);
    }

    #[test]
    fn test_cli_subcommands_parsing() {
        let cli = Cli::try_parse_from(["dbdq", "unlock"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Unlock)));

        let cli = Cli::try_parse_from(["dbdq", "lock", "frankfurt", "dublin"]).unwrap();
        match cli.command {
            Some(Commands::Lock { regions }) => {
                assert_eq!(regions, vec!["frankfurt", "dublin"]);
            }
            _ => panic!("Expected Commands::Lock"),
        }

        let cli_empty = Cli::try_parse_from(["dbdq", "lock"]).unwrap();
        match cli_empty.command {
            Some(Commands::Lock { regions }) => {
                assert!(regions.is_empty());
            }
            _ => panic!("Expected Commands::Lock with empty regions"),
        }
    }
}

