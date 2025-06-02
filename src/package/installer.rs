//! Package installation logic for different methods

use crate::{
  config::Config,
  package::{Installation, Package, PostInstall},
  Error, Result,
};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tokio::fs;
use tracing::{debug, info, warn, error};

/// Package installer handles different installation methods
pub struct Installer {
  config: Config,
}

impl Installer {
  /// Create a new installer
  pub fn new(config: &Config) -> Self {
      Self {
          config: config.clone(),
      }
  }
  
  /// Install a package using the appropriate method
  pub async fn install(&self, package: &Package) -> Result<()> {
      info!("Installing {} via {:?}", package.name, package.installation);
      
      match &package.installation {
          Installation::Pacman { packages, flags } => {
              self.install_pacman(packages, flags.as_ref()).await?;
          }
          Installation::Aur { package: pkg, helper } => {
              self.install_aur(pkg, helper.as_ref()).await?;
          }
          Installation::Binary { url, checksum, install_path, executable } => {
              self.install_binary(url, checksum.as_ref(), install_path, *executable).await?;
          }
          Installation::Source { url, build_commands, install_commands } => {
              self.install_source(url, build_commands, install_commands).await?;
          }
          Installation::Script { script, interpreter } => {
              self.install_script(script, interpreter).await?;
          }
          Installation::AppImage { url, checksum, integrate } => {
              self.install_appimage(url, checksum.as_ref(), *integrate, &package.name).await?;
          }
          Installation::Flatpak { id, remote } => {
              self.install_flatpak(id, remote.as_ref()).await?;
          }
      }
      
      // Run post-installation configuration
      if let Some(post_install) = &package.post_install {
          self.run_post_install(post_install, &package.name).await?;
      }
      
      Ok(())
  }
  
  /// Install packages via pacman
  async fn install_pacman(&self, packages: &[String], flags: Option<&Vec<String>>) -> Result<()> {
      let mut cmd = Command::new("pacman");
      cmd.args(&["-S", "--needed", "--noconfirm"]);
      
      if let Some(flags) = flags {
          cmd.args(flags);
      }
      
      cmd.args(packages);
      
      debug!("Running: pacman {:?}", cmd.as_std().get_args().collect::<Vec<_>>());
      
      let output = cmd.output().await?;
      
      if !output.status.success() {
          let stderr = String::from_utf8_lossy(&output.stderr);
          return Err(Error::InstallationFailed(format!(
              "Pacman installation failed: {}",
              stderr
          )));
      }
      
      info!("Successfully installed pacman packages: {:?}", packages);
      Ok(())
  }
  
  /// Install package from AUR
  async fn install_aur(&self, package: &str, helper: Option<&String>) -> Result<()> {
      let aur_helper = helper
          .map(|h| h.as_str())
          .or(self.config.aur_helper.as_deref())
          .unwrap_or("yay");
      
      // Check if AUR helper is available
      if !self.command_exists(aur_helper).await? {
          return Err(Error::InstallationFailed(format!(
              "AUR helper '{}' not found. Please install it first.",
              aur_helper
          )));
      }
      
      let mut cmd = Command::new(aur_helper);
      cmd.args(&["-S", "--needed", "--noconfirm", package]);
      
      debug!("Running: {} {:?}", aur_helper, cmd.as_std().get_args().collect::<Vec<_>>());
      
      let output = cmd.output().await?;
      
      if !output.status.success() {
          let stderr = String::from_utf8_lossy(&output.stderr);
          return Err(Error::InstallationFailed(format!(
              "AUR installation failed: {}",
              stderr
          )));
      }
      
      info!("Successfully installed AUR package: {}", package);
      Ok(())
  }
  
  /// Install binary from URL
  async fn install_binary(&self, url: &str, checksum: Option<&String>, install_path: &str, executable: bool) -> Result<()> {
      let pb = ProgressBar::new_spinner();
      pb.set_style(ProgressStyle::default_spinner()
          .template("{spinner:.green} {msg}")
          .unwrap());
      pb.set_message("Downloading binary...");
      pb.enable_steady_tick(std::time::Duration::from_millis(100));
      
      // Download the binary
      let client = reqwest::Client::builder()
          .user_agent("archbox/0.1.0")
          .build()?;
      
      let response = client.get(url).send().await?;
      
      if !response.status().is_success() {
          pb.finish_with_message("Download failed");
          return Err(Error::Network(reqwest::Error::from(response.error_for_status().unwrap_err())));
      }
      
      let content = response.bytes().await?;
      
      // Verify checksum if provided
      if let Some(expected_checksum) = checksum {
          pb.set_message("Verifying checksum...");
          let actual_checksum = self.calculate_sha256(&content);
          if actual_checksum != *expected_checksum {
              pb.finish_with_message("Checksum verification failed");
              return Err(Error::InstallationFailed(format!(
                  "Checksum mismatch. Expected: {}, Got: {}",
                  expected_checksum, actual_checksum
              )));
          }
      }
      
      // Ensure install directory exists
      let install_path = PathBuf::from(install_path);
      if let Some(parent) = install_path.parent() {
          fs::create_dir_all(parent).await?;
      }
      
      pb.set_message("Installing binary...");
      
      // Write the binary
      fs::write(&install_path, content).await?;
      
      // Make executable if required
      if executable {
          #[cfg(unix)]
          {
              use std::os::unix::fs::PermissionsExt;
              let mut perms = fs::metadata(&install_path).await?.permissions();
              perms.set_mode(0o755);
              fs::set_permissions(&install_path, perms).await?;
          }
      }
      
      pb.finish_with_message("Binary installed successfully");
      info!("Installed binary to: {}", install_path.display());
      Ok(())
  }
  
  /// Install from source
  async fn install_source(&self, url: &str, build_commands: &[String], install_commands: &[String]) -> Result<()> {
      let temp_dir = tempfile::tempdir()?;
      let work_dir = temp_dir.path();
      
      // Clone/download source
      let pb = ProgressBar::new_spinner();
      pb.set_style(ProgressStyle::default_spinner()
          .template("{spinner:.green} {msg}")
          .unwrap());
      pb.set_message("Downloading source...");
      pb.enable_steady_tick(std::time::Duration::from_millis(100));
      
      if url.ends_with(".git") || url.contains("github.com") || url.contains("gitlab.com") {
          // Git repository
          let output = Command::new("git")
              .args(&["clone", url, "."])
              .current_dir(work_dir)
              .output()
              .await?;
          
          if !output.status.success() {
              pb.finish_with_message("Source download failed");
              return Err(Error::InstallationFailed(format!(
                  "Git clone failed: {}",
                  String::from_utf8_lossy(&output.stderr)
              )));
          }
      } else {
          // Download and extract archive
          let client = reqwest::Client::new();
          let response = client.get(url).send().await?;
          let content = response.bytes().await?;
          
          // This is simplified - in practice you'd detect archive type and extract accordingly
          return Err(Error::InstallationFailed("Archive extraction not implemented yet".to_string()));
      }
      
      pb.set_message("Building from source...");
      
      // Run build commands
      for command in build_commands {
          let output = self.run_shell_command(command, work_dir).await?;
          if !output.status.success() {
              pb.finish_with_message("Build failed");
              return Err(Error::InstallationFailed(format!(
                  "Build command failed: {}\n{}",
                  command,
                  String::from_utf8_lossy(&output.stderr)
              )));
          }
      }
      
      pb.set_message("Installing...");
      
      // Run install commands
      for command in install_commands {
          let output = self.run_shell_command(command, work_dir).await?;
          if !output.status.success() {
              pb.finish_with_message("Installation failed");
              return Err(Error::InstallationFailed(format!(
                  "Install command failed: {}\n{}",
                  command,
                  String::from_utf8_lossy(&output.stderr)
              )));
          }
      }
      
      pb.finish_with_message("Source installation complete");
      Ok(())
  }
  
  /// Install via script
  async fn install_script(&self, script: &str, interpreter: &str) -> Result<()> {
      let temp_file = tempfile::NamedTempFile::new()?;
      let script_path = temp_file.path();
      
      // Write script to temporary file
      fs::write(script_path, script).await?;
      
      // Make script executable
      #[cfg(unix)]
      {
          use std::os::unix::fs::PermissionsExt;
          let mut perms = fs::metadata(script_path).await?.permissions();
          perms.set_mode(0o755);
          fs::set_permissions(script_path, perms).await?;
      }
      
      // Execute script
      let output = Command::new(interpreter)
          .arg(script_path)
          .output()
          .await?;
      
      if !output.status.success() {
          return Err(Error::InstallationFailed(format!(
              "Installation script failed: {}",
              String::from_utf8_lossy(&output.stderr)
          )));
      }
      
      info!("Installation script completed successfully");
      Ok(())
  }
  
  /// Install AppImage
  async fn install_appimage(&self, url: &str, checksum: Option<&String>, integrate: bool, name: &str) -> Result<()> {
      let appimage_dir = dirs::home_dir()
          .ok_or_else(|| Error::InstallationFailed("Could not find home directory".to_string()))?
          .join(".local/share/applications");
      
      fs::create_dir_all(&appimage_dir).await?;
      
      let appimage_path = appimage_dir.join(format!("{}.AppImage", name));
      
      // Download AppImage (reuse binary installation logic)
      self.install_binary(url, checksum, &appimage_path.to_string_lossy(), true).await?;
      
      if integrate {
          // Extract desktop file and icon for integration
          let output = Command::new(&appimage_path)
              .arg("--appimage-extract-and-run")
              .arg("--appimage-extract")
              .current_dir(&appimage_dir)
              .output()
              .await;
          
          if let Err(e) = output {
              warn!("Failed to extract AppImage for desktop integration: {}", e);
          } else {
              info!("AppImage desktop integration completed");
          }
      }
      
      Ok(())
  }
  
  /// Install Flatpak
  async fn install_flatpak(&self, id: &str, remote: Option<&String>) -> Result<()> {
      // Check if flatpak is available
      if !self.command_exists("flatpak").await? {
          return Err(Error::InstallationFailed(
              "Flatpak not found. Please install flatpak first.".to_string()
          ));
      }
      
      let mut cmd = Command::new("flatpak");
      cmd.args(&["install", "-y"]);
      
      if let Some(remote) = remote {
          cmd.arg(remote);
      } else {
          cmd.arg("flathub");
      }
      
      cmd.arg(id);
      
      let output = cmd.output().await?;
      
      if !output.status.success() {
          return Err(Error::InstallationFailed(format!(
              "Flatpak installation failed: {}",
              String::from_utf8_lossy(&output.stderr)
          )));
      }
      
      info!("Successfully installed Flatpak: {}", id);
      Ok(())
  }
  
  /// Run post-installation configuration
  async fn run_post_install(&self, post_install: &PostInstall, package_name: &str) -> Result<()> {
      info!("Running post-installation configuration for {}", package_name);
      
      // Run commands
      if let Some(commands) = &post_install.commands {
          for command in commands {
              info!("Running post-install command: {}", command);
              let output = self.run_shell_command(command, Path::new("/")).await?;
              if !output.status.success() {
                  warn!("Post-install command failed: {}", command);
              }
          }
      }
      
      // Create/modify config files
      if let Some(config_files) = &post_install.config_files {
          for (path, content) in config_files {
              self.create_config_file(path, content).await?;
          }
      }
      
      // Enable services
      if let Some(services) = &post_install.enable_services {
          for service in services {
              self.enable_service(service).await?;
          }
      }
      
      // Add user to groups
      if let Some(groups) = &post_install.user_groups {
          for group in groups {
              self.add_user_to_group(group).await?;
          }
      }
      
      // Set environment variables
      if let Some(env_vars) = &post_install.environment {
          self.set_environment_variables(env_vars).await?;
      }
      
      Ok(())
  }
  
  /// Helper function to check if a command exists
  async fn command_exists(&self, command: &str) -> Result<bool> {
      let output = Command::new("which")
          .arg(command)
          .output()
          .await?;
      
      Ok(output.status.success())
  }
  
  /// Helper function to run shell commands
  async fn run_shell_command(&self, command: &str, work_dir: &Path) -> Result<std::process::Output> {
      let output = Command::new("sh")
          .arg("-c")
          .arg(command)
          .current_dir(work_dir)
          .output()
          .await?;
      
      Ok(output)
  }
  
  /// Calculate SHA256 checksum
  fn calculate_sha256(&self, data: &[u8]) -> String {
      use sha2::{Sha256, Digest};
      let mut hasher = Sha256::new();
      hasher.update(data);
      format!("{:x}", hasher.finalize())
  }
  
  /// Create configuration file
  async fn create_config_file(&self, path: &str, content: &str) -> Result<()> {
      let expanded_path = shellexpand::tilde(path);
      let path = Path::new(expanded_path.as_ref());
      
      if let Some(parent) = path.parent() {
          fs::create_dir_all(parent).await?;
      }
      
      fs::write(path, content).await?;
      info!("Created config file: {}", path.display());
      Ok(())
  }
  
  /// Enable systemd service
  async fn enable_service(&self, service: &str) -> Result<()> {
      let output = Command::new("systemctl")
          .args(&["enable", "--now", service])
          .output()
          .await?;
      
      if output.status.success() {
          info!("Enabled service: {}", service);
      } else {
          warn!("Failed to enable service {}: {}", service, String::from_utf8_lossy(&output.stderr));
      }
      
      Ok(())
  }
  
  /// Add user to group
  async fn add_user_to_group(&self, group: &str) -> Result<()> {
      let username = std::env::var("USER")
          .or_else(|_| std::env::var("USERNAME"))
          .unwrap_or_else(|_| "user".to_string());
      
      let output = Command::new("usermod")
          .args(&["-a", "-G", group, &username])
          .output()
          .await?;
      
      if output.status.success() {
          info!("Added user {} to group {}", username, group);
      } else {
          warn!("Failed to add user to group {}: {}", group, String::from_utf8_lossy(&output.stderr));
      }
      
      Ok(())
  }
  
  /// Set environment variables
  async fn set_environment_variables(&self, env_vars: &HashMap<String, String>) -> Result<()> {
      let profile_path = dirs::home_dir()
          .ok_or_else(|| Error::InstallationFailed("Could not find home directory".to_string()))?
          .join(".profile");
      
      let mut content = String::new();
      if profile_path.exists() {
          content = fs::read_to_string(&profile_path).await?;
      }
      
      for (key, value) in env_vars {
          let env_line = format!("export {}=\"{}\"\n", key, value);
          if !content.contains(&env_line) {
              content.push_str(&env_line);
          }
      }
      
      fs::write(&profile_path, content).await?;
      info!("Updated environment variables in {}", profile_path.display());
      Ok(())
  }
}