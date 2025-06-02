pub mod definition;
pub mod installer;

pub use definition::*;
pub use installer::*;

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a package in the repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    pub long_description: Option<String>,
    pub categories: Vec<String>,
    pub dependencies: Vec<Dependency>,
    pub installation: Installation,
    pub post_install: Option<PostInstall>,
    pub metadata: PackageMetadata,
}

/// Package dependency definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: Option<String>,
    
    #[serde(default)]
    pub optional: bool,
    pub platform: Option<String>,
    
    #[serde(default)]
    pub dep_type: DependencyType,
}

/// Types of dependencies
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
    #[default]
    System,
    
    Package,
    
    Runtime,
    
    Build,
}

/// Installation methods
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum Installation {
    /// Install via pacman
    #[serde(rename = "pacman")]
    Pacman {
        packages: Vec<String>,
        flags: Option<Vec<String>>,
    },
    
    /// Install from AUR
    #[serde(rename = "aur")]
    Aur {
        package: String,
        helper: Option<String>,
    },
    
    /// Download and install binary
    #[serde(rename = "binary")]
    Binary {
        url: String,
        checksum: Option<String>,
        install_path: String,
        #[serde(default = "default_true")]
        executable: bool,
    },
    
    /// Install from source
    #[serde(rename = "source")]
    Source {
        url: String,
        build_commands: Vec<String>,
        install_commands: Vec<String>,
    },
    
    /// Install via script
    #[serde(rename = "script")]
    Script {
        script: String,
        #[serde(default = "default_shell")]
        interpreter: String,
    },
    
    /// Install AppImage
    #[serde(rename = "appimage")]
    AppImage {
        url: String,
        checksum: Option<String>,
        /// Desktop integration
        #[serde(default)]
        integrate: bool,
    },
    
    /// Install Flatpak
    #[serde(rename = "flatpak")]
    Flatpak {
        id: String,
        remote: Option<String>,
    },
}

/// Post-installation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostInstall {
    pub commands: Option<Vec<String>>,
    
    pub config_files: Option<HashMap<String, String>>,
    
    pub enable_services: Option<Vec<String>>,
    
    pub user_groups: Option<Vec<String>>,
    
    pub environment: Option<HashMap<String, String>>,
}

/// Package metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    pub author: Option<String>,
    
    pub homepage: Option<String>,
    
    pub repository: Option<String>,
    
    pub license: Option<String>,
    
    pub tags: Option<Vec<String>>,
    
    pub updated: Option<String>,
    
    pub size: Option<String>,
}

/// Package installation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallStatus {
    NotInstalled,
    Installed { version: String, installed_at: String },
    UpdateAvailable { current: String, available: String },
    Error { message: String },
}

fn default_true() -> bool { true }
fn default_shell() -> String { "/bin/bash".to_string() }

impl Package {
    /// Check if this package is a system package
    pub fn is_system_package(&self) -> bool {
        matches!(self.installation, Installation::Pacman { .. })
    }
    
    /// Get all dependencies of a specific type
    pub fn get_dependencies(&self, dep_type: DependencyType) -> Vec<&Dependency> {
        self.dependencies
            .iter()
            .filter(|dep| std::mem::discriminant(&dep.dep_type) == std::mem::discriminant(&dep_type))
            .collect()
    }
    
    /// Check if package has any optional dependencies
    pub fn has_optional_dependencies(&self) -> bool {
        self.dependencies.iter().any(|dep| dep.optional)
    }
}