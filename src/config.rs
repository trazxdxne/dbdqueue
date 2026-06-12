use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(default)]
    pub priority: Vec<String>,
    #[serde(default)]
    pub locked: Vec<String>,
    #[serde(default = "default_sort")]
    pub sort: String,
    #[serde(default = "default_mode")]
    pub mode: String,
}

fn default_sort() -> String {
    "default".to_string()
}

fn default_mode() -> String {
    "both".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            priority: vec![],
            locked: vec![],
            sort: default_sort(),
            mode: default_mode(),
        }
    }
}

pub fn get_config_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
            .join(".config")
            .join("dbdqueue")
            .join("config.toml")
    } else {
        PathBuf::from("config.toml")
    }
}

pub fn load_config(path: &Path) -> AppConfig {
    if path.exists()
        && let Ok(contents) = fs::read_to_string(path)
            && let Ok(config) = toml::from_str::<AppConfig>(&contents) {
                return config;
            }
    AppConfig::default()
}

pub fn save_config(path: &Path, config: &AppConfig) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let toml_str = toml::to_string_pretty(config)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    fs::write(path, toml_str)?;
    Ok(())
}

pub fn migrate_json_if_needed(toml_path: &Path) {
    if toml_path.exists() {
        return;
    }
    let json_path = toml_path.with_file_name("config.json");
    if json_path.exists()
        && let Ok(contents) = fs::read_to_string(&json_path)
            && let Ok(config) = serde_json::from_str::<AppConfig>(&contents)
                && save_config(toml_path, &config).is_ok() {
                    let _ = fs::remove_file(json_path);
                }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_load_save() {
        let test_path = Path::new("test_config_tmp.toml");
        let _ = fs::remove_file(test_path);
        
        let config = load_config(test_path);
        assert_eq!(config.sort, "default");
        assert_eq!(config.mode, "both");
        assert!(config.priority.is_empty());
        assert!(config.locked.is_empty());
        
        let new_config = AppConfig {
            sort: "survivor".to_string(),
            priority: vec!["Frankfurt".to_string(), "London".to_string()],
            ..Default::default()
        };
        save_config(test_path, &new_config).unwrap();
        
        let loaded = load_config(test_path);
        assert_eq!(loaded.sort, "survivor");
        assert_eq!(loaded.priority.len(), 2);
        assert_eq!(loaded.priority[0], "Frankfurt");
        assert_eq!(loaded.priority[1], "London");
        
        let _ = fs::remove_file(test_path);
    }

    #[test]
    fn test_migrate_json() {
        let toml_path = Path::new("test_config_migrate.toml");
        let json_path = toml_path.with_file_name("config.json");
        
        let _ = fs::remove_file(toml_path);
        let _ = fs::remove_file(&json_path);
        
        let json_content = r#"{
            "priority": ["Virginia"],
            "locked": ["us-east-1"],
            "sort": "killer",
            "mode": "standard"
        }"#;
        fs::write(&json_path, json_content).unwrap();
        
        migrate_json_if_needed(toml_path);
        
        assert!(toml_path.exists());
        assert!(!json_path.exists());
        
        let loaded = load_config(toml_path);
        assert_eq!(loaded.sort, "killer");
        assert_eq!(loaded.mode, "standard");
        assert_eq!(loaded.priority, vec!["Virginia"]);
        assert_eq!(loaded.locked, vec!["us-east-1"]);
        
        let _ = fs::remove_file(toml_path);
    }

}

