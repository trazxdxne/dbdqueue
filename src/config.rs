use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    #[default]
    Default,
    Killer,
    Survivor,
    Ping,
}

impl<'de> Deserialize<'de> for SortOrder {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer).unwrap_or_default();
        Ok(match s.to_lowercase().as_str() {
            "killer" => SortOrder::Killer,
            "survivor" => SortOrder::Survivor,
            "ping" | "priority" => SortOrder::Ping,
            "default" => SortOrder::Default,
            _ => SortOrder::Default,
        })
    }
}

impl Serialize for SortOrder {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            SortOrder::Default => serializer.serialize_str("default"),
            SortOrder::Killer => serializer.serialize_str("killer"),
            SortOrder::Survivor => serializer.serialize_str("survivor"),
            SortOrder::Ping => serializer.serialize_str("ping"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameMode {
    #[default]
    Standard,
    Event,
    Both,
}

impl<'de> Deserialize<'de> for GameMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer).unwrap_or_default();
        Ok(match s.to_lowercase().as_str() {
            "event" => GameMode::Event,
            "both" => GameMode::Both,
            "standard" => GameMode::Standard,
            _ => GameMode::Standard,
        })
    }
}

impl Serialize for GameMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            GameMode::Standard => serializer.serialize_str("standard"),
            GameMode::Event => serializer.serialize_str("event"),
            GameMode::Both => serializer.serialize_str("both"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    Auto,
    En,
    Ru,
}

impl<'de> Deserialize<'de> for Language {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer).unwrap_or_default();
        Ok(match s.to_lowercase().as_str() {
            "en" => Language::En,
            "ru" => Language::Ru,
            "auto" => Language::Auto,
            _ => Language::Auto,
        })
    }
}

impl Serialize for Language {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Language::Auto => serializer.serialize_str("auto"),
            Language::En => serializer.serialize_str("en"),
            Language::Ru => serializer.serialize_str("ru"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AppConfig {
    #[serde(default)]
    pub priority: Vec<String>,
    #[serde(default)]
    pub locked: Vec<String>,
    #[serde(default)]
    pub sort: SortOrder,
    #[serde(default)]
    pub mode: GameMode,
    #[serde(default)]
    pub lang: Language,
    #[serde(default)]
    pub api_url: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            priority: vec![],
            locked: vec![],
            sort: SortOrder::Default,
            mode: GameMode::Standard,
            lang: Language::Auto,
            api_url: None,
        }
    }
}

pub fn get_config_path() -> PathBuf {
    if cfg!(windows) {
        if let Ok(appdata) = std::env::var("APPDATA") {
            PathBuf::from(appdata).join("dbdqueue").join("config.toml")
        } else if let Ok(userprofile) = std::env::var("USERPROFILE") {
            PathBuf::from(userprofile)
                .join(".config")
                .join("dbdqueue")
                .join("config.toml")
        } else {
            PathBuf::from("config.toml")
        }
    } else if let Ok(home) = std::env::var("HOME") {
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
        && let Ok(config) = toml::from_str::<AppConfig>(&contents)
    {
        return config;
    }
    AppConfig::default()
}

pub fn save_config(path: &Path, config: &AppConfig) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let toml_str =
        toml::to_string_pretty(config).map_err(|e| std::io::Error::other(e.to_string()))?;
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
        && save_config(toml_path, &config).is_ok()
    {
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
        assert_eq!(config.sort, SortOrder::Default);
        assert_eq!(config.mode, GameMode::Standard);
        assert_eq!(config.lang, Language::Auto);
        assert!(config.priority.is_empty());
        assert!(config.locked.is_empty());

        let new_config = AppConfig {
            sort: SortOrder::Survivor,
            mode: GameMode::Event,
            lang: Language::Ru,
            priority: vec!["Frankfurt".to_string(), "London".to_string()],
            locked: vec!["eu-central-1".to_string()],
            api_url: None,
        };
        save_config(test_path, &new_config).unwrap();

        let loaded = load_config(test_path);
        assert_eq!(loaded.sort, SortOrder::Survivor);
        assert_eq!(loaded.mode, GameMode::Event);
        assert_eq!(loaded.lang, Language::Ru);
        assert_eq!(loaded.priority.len(), 2);
        assert_eq!(loaded.priority[0], "Frankfurt");
        assert_eq!(loaded.priority[1], "London");
        assert_eq!(loaded.locked, vec!["eu-central-1"]);

        let _ = fs::remove_file(test_path);
    }

    #[test]
    fn test_config_backward_compatibility() {
        let legacy_toml = r#"
            sort = "killer"
            mode = "standard"
            priority = ["Virginia"]
            locked = ["us-east-1"]
        "#;
        let config: AppConfig = toml::from_str(legacy_toml).unwrap();
        assert_eq!(config.sort, SortOrder::Killer);
        assert_eq!(config.mode, GameMode::Standard);
        assert_eq!(config.lang, Language::Auto); // Missing lang defaults to Auto
        assert_eq!(config.priority, vec!["Virginia"]);
        assert_eq!(config.locked, vec!["us-east-1"]);

        // Unknown values fallback to defaults
        let unknown_toml = r#"
            sort = "unrecognized_sort"
            mode = "custom_mode"
            lang = "es"
        "#;
        let config2: AppConfig = toml::from_str(unknown_toml).unwrap();
        assert_eq!(config2.sort, SortOrder::Default);
        assert_eq!(config2.mode, GameMode::Standard);
        assert_eq!(config2.lang, Language::Auto);

        // "priority" sort maps to Ping
        let prio_toml = r#"
            sort = "priority"
        "#;
        let config3: AppConfig = toml::from_str(prio_toml).unwrap();
        assert_eq!(config3.sort, SortOrder::Ping);
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
        assert_eq!(loaded.sort, SortOrder::Killer);
        assert_eq!(loaded.mode, GameMode::Standard);
        assert_eq!(loaded.priority, vec!["Virginia"]);
        assert_eq!(loaded.locked, vec!["us-east-1"]);

        let _ = fs::remove_file(toml_path);
    }
}
