use crate::{package::Package, Result};
use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Confirm, MultiSelect, Select};

pub struct InteractiveInstaller {
    term: Term,
    theme: ColorfulTheme,
}

impl InteractiveInstaller {
    pub fn new() -> Self {
        Self {
            term: Term::stdout(),
            theme: ColorfulTheme::default(),
        }
    }
    
    pub fn select_packages(&self, packages: &[Package]) -> Result<Vec<usize>> {
        if packages.is_empty() {
            return Ok(vec![]);
        }
        
        let items: Vec<String> = packages
            .iter()
            .map(|pkg| format!("{} - {}", style(&pkg.name).bold(), pkg.description))
            .collect();
        
        let selection = MultiSelect::with_theme(&self.theme)
            .with_prompt("Select packages to install")
            .items(&items)
            .interact()?;
        
        Ok(selection)
    }
    
    pub fn select_profile(&self, profiles: &[String]) -> Result<Option<usize>> {
        if profiles.is_empty() {
            return Ok(None);
        }
        
        let mut items = vec!["Custom installation".to_string()];
        items.extend(profiles.iter().cloned());
        
        let selection = Select::with_theme(&self.theme)
            .with_prompt("Choose installation profile")
            .items(&items)
            .default(0)
            .interact()?;
        
        if selection == 0 {
            Ok(None)
        } else {
            Ok(Some(selection - 1))
        }
    }
    
    pub fn confirm_installation(&self, packages: &[Package]) -> Result<bool> {
        self.term.write_line(&format!(
            "\n{} packages selected:",
            style("Following").green().bold()
        ))?;
        
        for pkg in packages {
            self.term.write_line(&format!(
                "  {} {} - {}",
                style("→").blue(),
                style(&pkg.name).bold(),
                pkg.description
            ))?;
        }
        
        self.term.write_line("")?;
        
        Ok(Confirm::with_theme(&self.theme)
            .with_prompt("Continue with installation?")
            .default(true)
            .interact()?)
    }
    
    pub fn handle_conflicts(&self, conflicts: &[(String, String)]) -> Result<Vec<String>> {
        if conflicts.is_empty() {
            return Ok(vec![]);
        }
        
        self.term.write_line(&format!(
            "{} Package conflicts detected:",
            style("⚠").yellow().bold()
        ))?;
        
        let mut resolutions = Vec::new();
        
        for (pkg1, pkg2) in conflicts {
            self.term.write_line(&format!(
                "  {} conflicts with {}",
                style(pkg1).red().bold(),
                style(pkg2).red().bold()
            ))?;
            
            let choices = vec![
                format!("Keep {}", pkg1),
                format!("Keep {}", pkg2),
                "Skip both".to_string(),
            ];
            
            let selection = Select::with_theme(&self.theme)
                .with_prompt(&format!("Resolve conflict between {} and {}", pkg1, pkg2))
                .items(&choices)
                .default(0)
                .interact()?;
            
            match selection {
                0 => resolutions.push(pkg1.clone()),
                1 => resolutions.push(pkg2.clone()),
                _ => {} // Skip both
            }
        }
        
        Ok(resolutions)
    }
}

impl Default for InteractiveInstaller {
    fn default() -> Self {
        Self::new()
    }
}