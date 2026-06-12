mod config;
mod api;
mod hosts;
mod ui;

use clap::{Parser, Subcommand};
use std::process;
use crate::api::{RegionQueueData, get_api_to_aws};
use crate::config::{get_config_path, load_config, save_config, migrate_json_if_needed};
use crate::hosts::update_hosts;

#[derive(Parser)]
#[command(name = "dbdqueue")]
#[command(about = "Dead by Daylight Queue Times & Region Locker CLI", long_about = None)]
struct Cli {
    #[arg(short, long, value_parser = ["survivor", "killer", "priority", "default"], help = "Sort output by column/rules (persists in config)")]
    sort: Option<String>,

    #[arg(short, long, value_parser = ["standard", "event", "both"], help = "Filter rows by Mode")]
    mode: Option<String>,

    #[arg(short, long, num_args = 0.., help = "Set priority regions in config (comma or space separated, leave empty for interactive menu)")]
    priority: Option<Vec<String>>,

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

pub fn parse_priority_input(words_list: &[String]) -> Vec<String> {
    let raw_str = words_list.join(" ");
    let mut parts = Vec::new();
    for p in raw_str.split(',') {
        let p_clean = p.trim();
        if !p_clean.is_empty() {
            parts.push(p_clean.to_string());
        }
    }
    
    let mut normalized_map = std::collections::HashMap::new();
    normalized_map.insert("sao paulo", "São Paulo");
    normalized_map.insert("sao_paulo", "São Paulo");
    normalized_map.insert("saopaulo", "São Paulo");
    normalized_map.insert("hong kong", "Hong Kong");
    normalized_map.insert("hong_kong", "Hong Kong");
    normalized_map.insert("hongkong", "Hong Kong");
    normalized_map.insert("montreal", "Montréal");
    
    let api_to_aws = crate::api::get_api_to_aws();
    let aws_to_api = crate::api::get_aws_to_api();
    
    let mut resolved = Vec::new();
    
    for part in parts {
        let part_lower = part.to_lowercase();
        
        if let Some(norm) = normalized_map.get(part_lower.as_str()) {
            resolved.push(norm.to_string());
            continue;
        }
        
        if let Some(api_name) = aws_to_api.get(part_lower.as_str()) {
            resolved.push(api_name.to_string());
            continue;
        }
        
        let mut chars = part_lower.chars();
        let part_cap = match chars.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
        };
        
        if api_to_aws.contains_key(part_cap.as_str()) {
            resolved.push(part_cap);
            continue;
        }
        
        let words: Vec<&str> = part.split_whitespace().collect();
        let mut i = 0;
        while i < words.len() {
            let word = words[i].to_lowercase();
            if i + 1 < words.len() {
                let two_words = format!("{} {}", word, words[i + 1].to_lowercase());
                if let Some(norm) = normalized_map.get(two_words.as_str()) {
                    resolved.push(norm.to_string());
                    i += 2;
                    continue;
                }
            }
            
            if let Some(norm) = normalized_map.get(word.as_str()) {
                resolved.push(norm.to_string());
            } else if let Some(api_name) = aws_to_api.get(word.as_str()) {
                resolved.push(api_name.to_string());
            } else {
                let mut c_chars = word.chars();
                let word_cap = match c_chars.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c_chars.as_str(),
                };
                if api_to_aws.contains_key(word_cap.as_str()) {
                    resolved.push(word_cap);
                }
            }
            i += 1;
        }
    }
    
    resolved
}

fn main() {
    let args = Cli::parse();
    
    let config_path = get_config_path();
    migrate_json_if_needed(&config_path);
    
    let mut config = load_config(&config_path);
    let mut config_changed = false;
    
    if let Some(ref s) = args.sort {
        config.sort = s.clone();
        config_changed = true;
        println!("\x1b[92mDefault sorting set to:\x1b[0m {}", s);
    }
    
    if let Some(ref p) = args.priority {
        let priorities = if p.is_empty() {
            match ui::interactive_priority_menu(&config.priority) {
                Some(prio) => prio,
                None => process::exit(0),
            }
        } else {
            parse_priority_input(p)
        };
        config.priority = priorities.clone();
        config_changed = true;
        println!("\x1b[92mUpdated priority list to:\x1b[0m {}", priorities.join(", "));
    }
    
    if config_changed
        && let Err(e) = save_config(&config_path, &config) {
            eprintln!("\x1b[91mFailed to save config:\x1b[0m {}", e);
        }
    
    // Process subcommands
    if let Some(ref cmd) = args.command {
        match cmd {
            Commands::Lock { regions } => {
                let resolved_regions = if regions.is_empty() {
                    match ui::interactive_lock_menu(&config.locked) {
                        Some(regs) => regs,
                        None => process::exit(0),
                    }
                } else {
                    let mut resolved = Vec::new();
                    let api_to_aws = get_api_to_aws();
                    let input_resolved = parse_priority_input(regions);
                    for r in input_resolved {
                        if let Some(aws_code) = api_to_aws.get(r.as_str()) {
                            resolved.push(aws_code.to_string());
                        }
                    }
                    resolved
                };
                
                config.locked = resolved_regions.clone();
                if let Err(e) = save_config(&config_path, &config) {
                    eprintln!("\x1b[91mFailed to save config:\x1b[0m {}", e);
                }
                update_hosts(Some(&resolved_regions));
                process::exit(0);
            }
            Commands::Unlock => {
                config.locked = vec![];
                if let Err(e) = save_config(&config_path, &config) {
                    eprintln!("\x1b[91mFailed to save config:\x1b[0m {}", e);
                }
                update_hosts(None);
                process::exit(0);
            }
        }
    }
    
    // Default action: fetch and print queue times
    let mut data = match api::fetch_queue_times() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("\x1b[91m{}\x1b[0m", e);
            process::exit(1);
        }
    };
    
    // Filter by mode
    let active_mode = args.mode.as_ref().unwrap_or(&config.mode);
    if active_mode == "standard" {
        data.retain(|r| r.mode == "Standard");
    } else if active_mode == "event" {
        data.retain(|r| r.mode == "Event");
    }
    
    // Split into priority and other lists
    let priority_names = &config.priority;
    let mut priority_rows: Vec<RegionQueueData> = data.iter()
        .filter(|r| priority_names.contains(&r.name))
        .cloned()
        .collect();
    let mut other_rows: Vec<RegionQueueData> = data.iter()
        .filter(|r| !priority_names.contains(&r.name))
        .cloned()
        .collect();
        
    // Apply sorting
    let active_sort = &config.sort;
    if active_sort == "survivor" {
        priority_rows.sort_by_key(|r| api::parse_time_to_seconds(&r.survivor));
        other_rows.sort_by_key(|r| api::parse_time_to_seconds(&r.survivor));
    } else if active_sort == "killer" {
        priority_rows.sort_by_key(|r| api::parse_time_to_seconds(&r.killer));
        other_rows.sort_by_key(|r| api::parse_time_to_seconds(&r.killer));
    } else if active_sort == "priority" {
        priority_rows.sort_by_key(|r| priority_names.iter().position(|name| name == &r.name).unwrap_or(usize::MAX));
        other_rows.sort_by_key(|r| r.name.clone());
    } else {
        // default
        priority_rows.sort_by_key(|r| r.name.clone());
        other_rows.sort_by_key(|r| r.name.clone());
    }
    
    let mut all_rows = priority_rows;
    all_rows.extend(other_rows);
    
    if !all_rows.is_empty() {
        ui::draw_table(&all_rows, priority_names);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_priority() {
        let input1 = vec!["Frankfurt,Dublin".to_string()];
        let parsed1 = parse_priority_input(&input1);
        assert_eq!(parsed1, vec!["Frankfurt", "Dublin"]);
        
        let input2 = vec!["sao paulo, montreal, virginia".to_string()];
        let parsed2 = parse_priority_input(&input2);
        assert_eq!(parsed2, vec!["São Paulo", "Montréal", "Virginia"]);
        
        let input3 = vec!["us-east-1".to_string(), "eu-central-1".to_string()];
        let parsed3 = parse_priority_input(&input3);
        assert_eq!(parsed3, vec!["Virginia", "Frankfurt"]);
    }
}

