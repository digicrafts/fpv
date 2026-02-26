use crate::config::keymap::UserKeymap;
use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn default_config_path() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".config/fpv/config.toml")
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UserThemeConfig {
    pub directory_color: Option<String>,
    pub fallback_file_color: Option<String>,
    pub hidden_dim_enabled: Option<bool>,
    #[serde(default)]
    pub file_type_colors: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UserConfig {
    #[serde(default)]
    pub mappings: HashMap<String, String>,
    #[serde(default)]
    pub theme: UserThemeConfig,
    pub status_display_mode: Option<StatusDisplayMode>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StatusDisplayMode {
    #[default]
    Bar,
    Title,
}

#[derive(Debug, Clone)]
pub struct ThemeProfile {
    pub directory_color: String,
    pub fallback_file_color: String,
    pub hidden_dim_enabled: bool,
    pub file_type_colors: HashMap<String, String>,
}

impl Default for ThemeProfile {
    fn default() -> Self {
        let mut file_type_colors = HashMap::new();
        file_type_colors.insert("rs".to_string(), "cyan".to_string());
        file_type_colors.insert("toml".to_string(), "magenta".to_string());
        file_type_colors.insert("md".to_string(), "green".to_string());
        file_type_colors.insert("txt".to_string(), "white".to_string());
        Self {
            directory_color: "yellow".to_string(),
            fallback_file_color: "white".to_string(),
            hidden_dim_enabled: true,
            file_type_colors,
        }
    }
}

pub fn load_user_config(path: &Path) -> Result<UserConfig> {
    let data = fs::read_to_string(path)?;
    let parsed = toml::from_str::<UserConfig>(&data)?;
    Ok(parsed)
}

pub fn load_user_keymap(path: &Path) -> Result<UserKeymap> {
    let parsed = load_user_config(path)?;
    Ok(UserKeymap {
        mappings: parsed.mappings,
    })
}
