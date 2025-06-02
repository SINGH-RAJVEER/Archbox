use crate::{App, Result};
use clap::{Args, Subcommand};
use console::style;
use std::path::PathBuf;

#[derive(Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Show current configuration
    Show,
    
    /// Set configuration values
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },
    
    /// Get configuration value
    Get {
        /// Configuration key
        key: String,
    },
    
    /// Add package path
    AddPath {
        /// Path to add
        path: PathBuf,
    },
    
    /// Remove package path
    RemovePath {
        /// Path to remove
        path: PathBuf,
    },
    
    /// Reset configuration to defaults
    Reset,
}

pub async fn execute(app: &mut App, args: ConfigArgs) -> Result<()> {
    match args.command {
        ConfigCommand::Show => Ok(show_config(&app.config)),
        ConfigCommand::Set { key, value } => set_config(&mut app.config, &key, &value).await,
        ConfigCommand::Get { key } => get_config(&app.config, &key),
        ConfigCommand::AddPath { path } => add_package_path(&mut app.config, path).await,
        ConfigCommand::RemovePath { path } => remove_package_path(&mut app.config, &path).await,
        ConfigCommand::Reset => reset_config(&mut app.config).await,
    }
}

fn show_config(config: &crate::config::Config) {
    println!("{}", style("ArchBox Configuration").bold().underlined());
    println!();
    
    println!("{}", style("Package Paths:").bold());
    for (i, path) in config.package_paths.iter().enumerate() {
        println!("  {}. {}", i + 1, path.display());
    }
    
    println!("\n{}", style("Installation:").bold());
    println!("  Binary directory: {}", config.installation.binary_dir.display());
    println!("  Verify checksums: {}", config.installation.verify_checksums);
    println!("  Create backups: {}", config.installation.create_backups);
    println!("  Download timeout: {}s", config.installation.download_timeout);
    
    if let Some(ref temp_dir) = config.installation.temp_dir {
        println!("  Temp directory: {}", temp_dir.display());
    }
    
    println!("\n{}", style("Repository:").bold());
    if let Some(ref url) = config.repository.update_url {
        println!("  Update URL: {}", url);
    }
    println!("  Update interval: {}h", config.repository.update_interval);
    println!("  Auto update: {}", config.repository.auto_update);
    
    println!("\n{}", style("UI:").bold());
    println!("  Use colors: {}", config.ui.use_colors);
    println!("  Show progress: {}", config.ui.show_progress);
    println!("  Log level: {}", config.ui.log_level);
    
    if let Some(ref helper) = config.aur_helper {
        println!("\n{}", style("AUR Helper:").bold());
        println!("  {}", helper);
    }
    
    println!("\n{}", style("Config file:").bold());
    println!("  {}", crate::config::Config::config_path().display());
}

async fn set_config(config: &mut crate::config::Config, key: &str, value: &str) -> Result<()> {
    match key {
        "aur_helper" => {
            config.set_aur_helper(value.to_string());
            crate::cli::print_success(&format!("Set AUR helper to: {}", value));
        }
        "installation.verify_checksums" => {
            config.installation.verify_checksums = value.parse()
                .map_err(|_| crate::Error::Config("Invalid boolean value".to_string()))?;
            crate::cli::print_success(&format!("Set verify_checksums to: {}", value));
        }
        "installation.create_backups" => {
            config.installation.create_backups = value.parse()
                .map_err(|_| crate::Error::Config("Invalid boolean value".to_string()))?;
            crate::cli::print_success(&format!("Set create_backups to: {}", value));
        }
        "installation.download_timeout" => {
            config.installation.download_timeout = value.parse()
                .map_err(|_| crate::Error::Config("Invalid number value".to_string()))?;
            crate::cli::print_success(&format!("Set download_timeout to: {}", value));
        }
        "repository.update_url" => {
            config.repository.update_url = Some(value.to_string());
            crate::cli::print_success(&format!("Set update_url to: {}", value));
        }
        "repository.auto_update" => {
            config.repository.auto_update = value.parse()
                .map_err(|_| crate::Error::Config("Invalid boolean value".to_string()))?;
            crate::cli::print_success(&format!("Set auto_update to: {}", value));
        }
        "ui.use_colors" => {
            config.ui.use_colors = value.parse()
                .map_err(|_| crate::Error::Config("Invalid boolean value".to_string()))?;
            crate::cli::print_success(&format!("Set use_colors to: {}", value));
        }
        "ui.log_level" => {
            config.ui.log_level = value.to_string();
            crate::cli::print_success(&format!("Set log_level to: {}", value));
        }
        _ => {
            return Err(crate::Error::Config(format!("Unknown configuration key: {}", key)));
        }
    }
    
    config.save()?;
    Ok(())
}

fn get_config(config: &crate::config::Config, key: &str) -> Result<()> {
    let value = match key {
        "aur_helper" => config.aur_helper.as_deref().unwrap_or("not set").to_string(),
        "installation.verify_checksums" => config.installation.verify_checksums.to_string(),
        "installation.create_backups" => config.installation.create_backups.to_string(),
        "installation.download_timeout" => config.installation.download_timeout.to_string(),
        "repository.update_url" => config.repository.update_url.as_deref().unwrap_or("not set").to_string(),
        "repository.auto_update" => config.repository.auto_update.to_string(),
        "ui.use_colors" => config.ui.use_colors.to_string(),
        "ui.log_level" => config.ui.log_level.clone(),
        _ => {
            return Err(crate::Error::Config(format!("Unknown configuration key: {}", key)));
        }
    };
    
    println!("{}: {}", style(key).bold(), value);
    Ok(())
}

async fn add_package_path(config: &mut crate::config::Config, path: PathBuf) -> Result<()> {
    config.add_package_path(path.clone());
    config.save()?;
    crate::cli::print_success(&format!("Added package path: {}", path.display()));
    Ok(())
}

async fn remove_package_path(config: &mut crate::config::Config, path: &PathBuf) -> Result<()> {
    config.remove_package_path(path);
    config.save()?;
    crate::cli::print_success(&format!("Removed package path: {}", path.display()));
    Ok(())
}

async fn reset_config(config: &mut crate::config::Config) -> Result<()> {
    *config = crate::config::Config::default();
    config.save()?;
    crate::cli::print_success("Configuration reset to defaults");
    Ok(())
}