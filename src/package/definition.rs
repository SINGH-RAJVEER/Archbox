use crate::{package::Package, Error, Result};
use serde_yaml;
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug)]
pub struct DefinitionLoader {
    packages: HashMap<String, Package>,
}

impl DefinitionLoader {
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
        }
    }
    
    pub async fn load_from_directory<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        
        if !path.exists() {
            return Err(Error::Config(format!("Package directory not found: {}", path.display())));
        }
        
        for entry in WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "yaml" || ext == "yml"))
        {
            self.load_definition_file(entry.path()).await?;
        }
        
        Ok(())
    }
    
    pub async fn load_definition_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let content = tokio::fs::read_to_string(path.as_ref()).await?;
        
        if let Ok(package) = serde_yaml::from_str::<Package>(&content) {
            self.validate_package(&package)?;
            self.packages.insert(package.name.clone(), package);
            return Ok(());
        }
        
        if let Ok(packages) = serde_yaml::from_str::<HashMap<String, Package>>(&content) {
            for (name, mut package) in packages {
                package.name = name.clone();
                self.validate_package(&package)?;
                self.packages.insert(name, package);
            }
            return Ok(());
        }
        
        Err(Error::Config(format!(
            "Invalid package definition format in file: {}",
            path.as_ref().display()
        )))
    }
    
    fn validate_package(&self, package: &Package) -> Result<()> {
        if package.name.is_empty() {
            return Err(Error::Config("Package name cannot be empty".to_string()));
        }
        
        if package.version.is_empty() {
            return Err(Error::Config(format!("Package {} missing version", package.name)));
        }
        
        if package.description.is_empty() {
            return Err(Error::Config(format!("Package {} missing description", package.name)));
        }
        
        self.validate_installation(&package.installation, &package.name)?;
        
        for dep in &package.dependencies {
            if dep.name.is_empty() {
                return Err(Error::Config(format!(
                    "Package {} has dependency with empty name", 
                    package.name
                )));
            }
        }
        
        Ok(())
    }
    
    fn validate_installation(&self, installation: &crate::package::Installation, package_name: &str) -> Result<()> {
        use crate::package::Installation;
        
        match installation {
            Installation::Pacman { packages, .. } => {
                if packages.is_empty() {
                    return Err(Error::Config(format!(
                        "Package {} has empty pacman packages list",
                        package_name
                    )));
                }
            }
            Installation::Aur { package, .. } => {
                if package.is_empty() {
                    return Err(Error::Config(format!(
                        "Package {} has empty AUR package name",
                        package_name
                    )));
                }
            }
            Installation::Binary { url, install_path, .. } => {
                if url.is_empty() || install_path.is_empty() {
                    return Err(Error::Config(format!(
                        "Package {} has invalid binary installation config",
                        package_name
                    )));
                }
            }
            Installation::Source { url, build_commands, install_commands, .. } => {
                if url.is_empty() {
                    return Err(Error::Config(format!(
                        "Package {} has empty source URL",
                        package_name
                    )));
                }
                if build_commands.is_empty() {
                    return Err(Error::Config(format!(
                        "Package {} has empty build commands",
                        package_name
                    )));
                }
                if install_commands.is_empty() {
                    return Err(Error::Config(format!(
                        "Package {} has empty install commands",
                        package_name
                    )));
                }
            }
            Installation::Script { script, .. } => {
                if script.is_empty() {
                    return Err(Error::Config(format!(
                        "Package {} has empty installation script",
                        package_name
                    )));
                }
            }
            Installation::AppImage { url, .. } => {
                if url.is_empty() {
                    return Err(Error::Config(format!(
                        "Package {} has empty AppImage URL",
                        package_name
                    )));
                }
            }
            Installation::Flatpak { id, .. } => {
                if id.is_empty() {
                    return Err(Error::Config(format!(
                        "Package {} has empty Flatpak ID",
                        package_name
                    )));
                }
            }
        }
        
        Ok(())
    }
    
    pub fn packages(&self) -> &HashMap<String, Package> {
        &self.packages
    }
    
    pub fn get_package(&self, name: &str) -> Option<&Package> {
        self.packages.get(name)
    }
    
    pub fn search_packages(&self, query: &str) -> Vec<&Package> {
        let query_lower = query.to_lowercase();
        
        self.packages
            .values()
            .filter(|package| {
                package.name.to_lowercase().contains(&query_lower)
                    || package.description.to_lowercase().contains(&query_lower)
                    || package.metadata.tags.as_ref().map_or(false, |tags| {
                        tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
                    })
            })
            .collect()
    }
    
    pub fn get_packages_by_category(&self, category: &str) -> Vec<&Package> {
        self.packages
            .values()
            .filter(|package| package.categories.contains(&category.to_string()))
            .collect()
    }
    
    pub fn get_categories(&self) -> Vec<String> {
        let mut categories: Vec<String> = self.packages
            .values()
            .flat_map(|package| package.categories.iter())
            .cloned()
            .collect();
        
        categories.sort();
        categories.dedup();
        categories
    }
}

impl Default for DefinitionLoader {
    fn default() -> Self {
        Self::new()
    }
}