use crate::{
  config::Config,
  package::{DefinitionLoader, Package, InstallStatus, DependencyType},
  cli::commands::{search::SearchArgs, list::ListArgs},
  Error, Result,
};
use std::collections::{HashMap, HashSet, VecDeque};
use tokio::process::Command;
use tracing::{debug, info, warn};

#[derive(Debug)]
pub struct Manager {
  pub loader: DefinitionLoader,
  config: Config,
  installed_cache: HashMap<String, InstallStatus>,
}

impl Manager {
  pub async fn new(config: &Config) -> Result<Self> {
      let mut loader = DefinitionLoader::new();
      
      for path in &config.package_paths {
          info!("Loading packages from: {}", path.display());
          loader.load_from_directory(path).await?;
      }
      
      let mut manager = Self {
          loader,
          config: config.clone(),
          installed_cache: HashMap::new(),
      };
      
      manager.refresh_installed_cache().await?;
      
      Ok(manager)
  }
  
  pub async fn search_packages(&self, query: &str, args: &SearchArgs) -> Result<Vec<Package>> {
      let mut results = self.loader.search_packages(query);
      
      if let Some(category) = &args.category {
          results.retain(|package| package.categories.contains(category));
      }
      
      if args.installed {
          results.retain(|package| {
              matches!(
                  self.installed_cache.get(&package.name),
                  Some(InstallStatus::Installed { .. })
              )
          });
      }
      
      results.sort_by(|a, b| {
          let a_exact = a.name.to_lowercase() == query.to_lowercase();
          let b_exact = b.name.to_lowercase() == query.to_lowercase();
          
          match (a_exact, b_exact) {
              (true, false) => std::cmp::Ordering::Less,
              (false, true) => std::cmp::Ordering::Greater,
              _ => a.name.cmp(&b.name),
          }
      });
      
      Ok(results.into_iter().cloned().collect())
  }
  
  pub async fn list_packages(&self, args: &ListArgs) -> Result<Vec<Package>> {
      let mut packages: Vec<Package> = self.loader.packages().values().cloned().collect();
      
      if let Some(category) = &args.category {
          packages.retain(|package| package.categories.contains(category));
      }
      
      packages.sort_by(|a, b| a.name.cmp(&b.name));
      
      Ok(packages)
  }
  
  pub async fn resolve_packages(&self, package_names: &[String]) -> Result<Vec<Package>> {
      let mut resolved = Vec::new();
      let mut visited = HashSet::new();
      let mut visiting = HashSet::new();
      
      for name in package_names {
          self.resolve_package_recursive(name, &mut resolved, &mut visited, &mut visiting)?;
      }
      
      Ok(resolved)
  }
  
  fn resolve_package_recursive(
      &self,
      name: &str,
      resolved: &mut Vec<Package>,
      visited: &mut HashSet<String>,
      visiting: &mut HashSet<String>,
  ) -> Result<()> {
      if visited.contains(name) {
          return Ok(());
      }
      
      if visiting.contains(name) {
          return Err(Error::Dependency(format!("Circular dependency detected: {}", name)));
      }
      
      let package = self.loader.get_package(name)
          .ok_or_else(|| Error::PackageNotFound(name.to_string()))?;
      
      visiting.insert(name.to_string());
      
      // Resolve dependencies first
      for dep in &package.dependencies {
          if dep.optional {
              continue; // Skip optional dependencies for now
          }
          
          match dep.dep_type {
              DependencyType::Package => {
                  self.resolve_package_recursive(&dep.name, resolved, visited, visiting)?;
              }
              DependencyType::System => {
                  // System dependencies are handled by the installer
                  debug!("System dependency: {}", dep.name);
              }
              _ => {}
          }
      }
      
      visiting.remove(name);
      visited.insert(name.to_string());
      
      if !resolved.iter().any(|p| p.name == package.name) {
          resolved.push(package.clone());
      }
      
      Ok(())
  }
  
  /// Install a package
  pub async fn install_package(&mut self, package: &Package, force: bool) -> Result<()> {
      info!("Installing package: {}", package.name);
      
      if !force {
          if let Some(InstallStatus::Installed { .. }) = self.installed_cache.get(&package.name) {
              warn!("Package {} is already installed", package.name);
              return Ok(());
          }
      }
      
      self.install_system_dependencies(package).await?;
      
      let installer = crate::package::installer::Installer::new(&self.config);
      installer.install(package).await?;
      
      self.installed_cache.insert(
          package.name.clone(),
          InstallStatus::Installed {
              version: package.version.clone(),
              installed_at: chrono::Utc::now().to_rfc3339(),
          },
      );
      
      info!("Successfully installed package: {}", package.name);
      Ok(())
  }
  
  async fn install_system_dependencies(&self, package: &Package) -> Result<()> {
      let system_deps: Vec<&str> = package
          .get_dependencies(DependencyType::System)
          .iter()
          .map(|dep| dep.name.as_str())
          .collect();
      
      if system_deps.is_empty() {
          return Ok(());
      }
      
      info!("Installing system dependencies: {:?}", system_deps);
      
      let mut cmd = Command::new("pacman");
      cmd.args(&["-S", "--needed", "--noconfirm"])
          .args(&system_deps);
      
      let output = cmd.output().await?;
      
      if !output.status.success() {
          return Err(Error::CommandFailed {
              message: format!(
                  "Failed to install system dependencies: {}",
                  String::from_utf8_lossy(&output.stderr)
              ),
          });
      }
      
      Ok(())
  }
  
  pub async fn is_installed(&self, package_name: &str) -> Result<bool> {
      Ok(matches!(
          self.installed_cache.get(package_name),
          Some(InstallStatus::Installed { .. })
      ))
  }
  
  async fn refresh_installed_cache(&mut self) -> Result<()> {
      debug!("Refreshing installed package cache");
      
      // This is a simplified implementation
      // In practice, you'd want to check various installation methods
      
      for package in self.loader.packages().values() {
          let status = self.check_package_status(package).await?;
          self.installed_cache.insert(package.name.clone(), status);
      }
      
      Ok(())
  }
  
  /// Check the installation status of a specific package
  async fn check_package_status(&self, package: &Package) -> Result<InstallStatus> {
      // Implementation depends on installation method
      // This is a simplified version
      match &package.installation {
          crate::package::Installation::Pacman { packages, .. } => {
              for pkg in packages {
                  let output = Command::new("pacman")
                      .args(&["-Q", pkg])
                      .output()
                      .await?;
                  
                  if output.status.success() {
                      let version_info = String::from_utf8_lossy(&output.stdout);
                      let version = version_info
                          .split_whitespace()
                          .nth(1)
                          .unwrap_or("unknown")
                          .to_string();
                      
                      return Ok(InstallStatus::Installed {
                          version,
                          installed_at: "unknown".to_string(),
                      });
                  }
              }
              Ok(InstallStatus::NotInstalled)
          }
          _ => {
              // For other installation methods, implement specific checks
              Ok(InstallStatus::NotInstalled)
          }
      }
  }
}