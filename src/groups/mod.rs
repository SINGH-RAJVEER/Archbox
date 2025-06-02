use crate::{package::Package, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageGroup {
    pub name: String,
    pub description: String,
    pub packages: Vec<String>,
    pub optional_packages: Vec<String>,
    pub conflicts: Vec<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationProfile {
    pub name: String,
    pub description: String,
    pub groups: Vec<String>,
    pub additional_packages: Vec<String>,
    pub excluded_packages: Vec<String>,
    pub post_install_script: Option<String>,
}

pub struct GroupManager {
    groups: HashMap<String, PackageGroup>,
    profiles: HashMap<String, InstallationProfile>,
}

impl GroupManager {
    pub fn new() -> Self {
        let mut manager = Self {
            groups: HashMap::new(),
            profiles: HashMap::new(),
        };
        
        manager.load_default_groups();
        manager.load_default_profiles();
        manager
    }
    
    fn load_default_groups(&mut self) {
        // Development group
        self.groups.insert("development".to_string(), PackageGroup {
            name: "development".to_string(),
            description: "Essential development tools".to_string(),
            packages: vec![
                "neovim".to_string(),
                "git".to_string(),
                "rust-toolchain".to_string(),
                "nodejs-lts".to_string(),
            ],
            optional_packages: vec![
                "docker".to_string(),
                "lazygit".to_string(),
            ],
            conflicts: vec![],
            category: Some("development".to_string()),
        });
        
        // Media group
        self.groups.insert("media".to_string(), PackageGroup {
            name: "media".to_string(),
            description: "Media creation and consumption tools".to_string(),
            packages: vec![
                "vlc".to_string(),
                "gimp".to_string(),
                "obs-studio".to_string(),
            ],
            optional_packages: vec![
                "blender".to_string(),
                "audacity".to_string(),
            ],
            conflicts: vec![],
            category: Some("media".to_string()),
        });
        
        // Gaming group
        self.groups.insert("gaming".to_string(), PackageGroup {
            name: "gaming".to_string(),
            description: "Gaming platform and tools".to_string(),
            packages: vec![
                "steam".to_string(),
                "discord".to_string(),
            ],
            optional_packages: vec![
                "lutris".to_string(),
                "gamemode".to_string(),
            ],
            conflicts: vec![],
            category: Some("gaming".to_string()),
        });
    }
    
    fn load_default_profiles(&mut self) {
        // Developer profile
        self.profiles.insert("developer".to_string(), InstallationProfile {
            name: "developer".to_string(),
            description: "Complete development environment".to_string(),
            groups: vec!["development".to_string()],
            additional_packages: vec![
                "starship".to_string(),
                "obsidian".to_string(),
            ],
            excluded_packages: vec![],
            post_install_script: Some(r#"
                # Configure development environment
                git config --global init.defaultBranch main
                echo "Development environment configured!"
            "#.to_string()),
        });
        
        // Content creator profile
        self.profiles.insert("content-creator".to_string(), InstallationProfile {
            name: "content-creator".to_string(),
            description: "Content creation and streaming setup".to_string(),
            groups: vec!["media".to_string()],
            additional_packages: vec![
                "discord".to_string(),
            ],
            excluded_packages: vec![],
            post_install_script: None,
        });
        
        // Gamer profile
        self.profiles.insert("gamer".to_string(), InstallationProfile {
            name: "gamer".to_string(),
            description: "Gaming setup with performance optimizations".to_string(),
            groups: vec!["gaming".to_string()],
            additional_packages: vec![],
            excluded_packages: vec![],
            post_install_script: Some(r#"
                # Enable gamemode
                sudo systemctl enable --now gamemode
                echo "Gaming optimizations applied!"
            "#.to_string()),
        });
    }
    
    pub fn get_group(&self, name: &str) -> Option<&PackageGroup> {
        self.groups.get(name)
    }
    
    pub fn get_profile(&self, name: &str) -> Option<&InstallationProfile> {
        self.profiles.get(name)
    }
    
    pub fn list_groups(&self) -> Vec<&PackageGroup> {
        self.groups.values().collect()
    }
    
    pub fn list_profiles(&self) -> Vec<&InstallationProfile> {
        self.profiles.values().collect()
    }
    
    pub fn resolve_profile_packages(&self, profile_name: &str) -> Result<Vec<String>> {
        let profile = self.get_profile(profile_name)
            .ok_or_else(|| Error::Config(format!("Profile not found: {}", profile_name)))?;
        
        let mut packages = Vec::new();
        
        // Add packages from groups
        for group_name in &profile.groups {
            if let Some(group) = self.get_group(group_name) {
                packages.extend(group.packages.iter().cloned());
            }
        }
        
        // Add additional packages
        packages.extend(profile.additional_packages.iter().cloned());
        
        // Remove excluded packages
        packages.retain(|pkg| !profile.excluded_packages.contains(pkg));
        
        // Remove duplicates
        packages.sort();
        packages.dedup();
        
        Ok(packages)
    }
}