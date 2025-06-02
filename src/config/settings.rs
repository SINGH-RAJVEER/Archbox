use crate::{Error, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub package_paths: Vec<PathBuf>,
    pub aur_helper: Option<String>,
    pub installation: InstallationConfig,
    pub repository: RepositoryConfig,
    pub ui: UiConfig,
}

/// Installation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationConfig {
    pub binary_dir: PathBuf,
    pub temp_dir: Option<PathBuf>,
    #[serde(default = "default_true")]
    pub verify_checksums: bool,
    #[serde(default = "default_true")]
    pub create_backups: bool,
    #[serde(default = "default_download_timeout")]
    pub download_timeout: u64,
}

/// Repository configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryConfig {
    pub update_url: Option<String>,
    #[serde(default = "default_update_interval")]
    pub update_interval: u64,
    #[serde(default)]
    pub auto_update: bool,
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_true")]
    pub use_colors: bool,
    #[serde(default = "default_true")]
    pub show_progress: bool,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Default for Config {
    fn default() -> Self {
        let binary_dir = dirs::home_dir()
            .map(|home| home.join(".local/bin"))
            .unwrap_or_else(|| PathBuf::from("/usr/local/bin"));
        
        Self {
            package_paths: vec![
                PathBuf::from("./data/packages"),
                get_config_dir().join("packages"),
                PathBuf::from("/etc/archbox/packages"),
            ],
            aur_helper: None,
            installation: InstallationConfig {
                binary_dir,
                temp_dir: None,
                verify_checksums: true,
                create_backups: true,
                download_timeout: 300,
            },
            repository: RepositoryConfig {
                update_url: Some("https://raw.githubusercontent.com/example/archbox-packages/main/packages.yaml".to_string()),
                update_interval: 24,
                auto_update: false,
            },
            ui: UiConfig {
                use_colors: true,
                show_progress: true,
                log_level: "info".to_string(),
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = get_config_path();
        
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = serde_yaml::from_str(&content)?;
            Ok(config)
        } else {
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }
    
    pub fn save(&self) -> Result<()> {
        let config_path = get_config_path();
        
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let content = serde_yaml::to_string(self)?;
        std::fs::write(&config_path, content)?;
        
        Ok(())
    }
    
    pub fn config_path() -> PathBuf {
        get_config_path()
    }
    
    pub fn set_aur_helper(&mut self, helper: String) {
        self.aur_helper = Some(helper);
    }
    
    pub fn add_package_path(&mut self, path: PathBuf) {
        if !self.package_paths.contains(&path) {
            self.package_paths.push(path);
        }
    }
    
    pub fn remove_package_path(&mut self, path: &PathBuf) {
        self.package_paths.retain(|p| p != path);
    }
}

pub fn get_config_dir() -> PathBuf {
    ProjectDirs::from("com", "archbox", "ArchBox")
        .map(|dirs| dirs.config_dir().to_path_buf())
        .unwrap_or_else(|| {
            let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            path.push(".config");
            path.push("archbox");
            path
        })
}

fn get_config_path() -> PathBuf {
    get_config_dir().join("config.yaml")
}

fn default_true() -> bool { true }
fn default_download_timeout() -> u64 { 300 }
fn default_update_interval() -> u64 { 24 }
fn default_log_level() -> String { "info".to_string() }