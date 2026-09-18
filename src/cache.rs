use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct PingCache {
    #[serde(default)]
    pub updated_at: u64,
    #[serde(default)]
    pub pings: HashMap<String, u32>,
}

pub fn get_cache_path(config_path: &Path) -> PathBuf {
    if let Some(parent) = config_path.parent() {
        parent.join("cache.json")
    } else {
        PathBuf::from("cache.json")
    }
}

pub fn load_ping_cache(path: &Path) -> PingCache {
    if path.exists()
        && let Ok(contents) = fs::read_to_string(path)
        && let Ok(cache) = serde_json::from_str::<PingCache>(&contents)
    {
        return cache;
    }
    PingCache::default()
}

pub fn save_ping_cache(
    path: &Path,
    new_pings: &HashMap<String, u32>,
) -> Result<(), std::io::Error> {
    if new_pings.is_empty() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut current = load_ping_cache(path);
    for (region, &ms) in new_pings {
        current.pings.insert(region.clone(), ms);
    }
    current.updated_at = chrono::Utc::now().timestamp().max(0) as u64;

    let json_str = serde_json::to_string_pretty(&current)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    fs::write(path, json_str)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_cache_roundtrip() {
        let mut pings = HashMap::new();
        pings.insert("eu-central-1".to_string(), 42);
        pings.insert("us-east-1".to_string(), 105);

        let cache = PingCache {
            updated_at: 1700000000,
            pings: pings.clone(),
        };

        let json = serde_json::to_string(&cache).unwrap();
        let deserialized: PingCache = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, cache);
    }

    #[test]
    fn test_load_missing_cache() {
        let path = Path::new("non_existent_cache_dir_12345/cache.json");
        let cache = load_ping_cache(path);
        assert_eq!(cache, PingCache::default());
        assert!(cache.pings.is_empty());
    }

    #[test]
    fn test_save_and_load_ping_cache() {
        let temp_dir = std::env::temp_dir().join(format!("dbdq_cache_test_{}", std::process::id()));
        let cache_path = temp_dir.join("cache.json");

        let mut initial_pings = HashMap::new();
        initial_pings.insert("eu-central-1".to_string(), 35);
        save_ping_cache(&cache_path, &initial_pings).unwrap();

        let loaded = load_ping_cache(&cache_path);
        assert_eq!(loaded.pings.get("eu-central-1"), Some(&35));
        assert!(loaded.updated_at > 0);

        // Update with another region and ensure merging works
        let mut update_pings = HashMap::new();
        update_pings.insert("us-east-1".to_string(), 99);
        save_ping_cache(&cache_path, &update_pings).unwrap();

        let loaded2 = load_ping_cache(&cache_path);
        assert_eq!(loaded2.pings.get("eu-central-1"), Some(&35));
        assert_eq!(loaded2.pings.get("us-east-1"), Some(&99));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_get_cache_path() {
        let config_path = Path::new("foo").join("bar").join("config.toml");
        let cache_path = get_cache_path(&config_path);
        assert_eq!(cache_path, Path::new("foo").join("bar").join("cache.json"));
    }

    #[test]
    fn test_startup_loads_cached_ping_and_computes_best_pick_immediately() {
        let temp_dir = std::env::temp_dir().join(format!("dbdq_startup_test_{}", std::process::id()));
        let cache_path = temp_dir.join("cache.json");

        let mut pings = HashMap::new();
        pings.insert("eu-central-1".to_string(), 25);
        pings.insert("us-east-1".to_string(), 120);
        save_ping_cache(&cache_path, &pings).unwrap();

        // Simulate main.rs startup sequence
        let mut app = crate::app::App::new(
            crate::config::SortOrder::Default,
            crate::config::GameMode::Standard,
            crate::config::TimeFormat::Exact,
            vec![],
            crate::config::Language::En,
            None,
        );

        let ping_cache = load_ping_cache(&cache_path);
        app.pings = ping_cache.pings;
        app.queues = vec![
            crate::api::RegionQueueData::new("[DE]", "Frankfurt", "Standard", "10s", "10s"),
            crate::api::RegionQueueData::new("[US]", "Virginia", "Standard", "15s", "15s"),
        ];

        // Best pick is available IMMEDIATELY without waiting for network pings
        let summary = app.summary();
        assert!(summary.killer.is_some());
        assert!(summary.survivor.is_some());
        assert_eq!(summary.killer.unwrap().row.name, "Frankfurt");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
