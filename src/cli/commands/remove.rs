use crate::{App, Result};
use clap::Args;
use console::style;
use std::io::{self, Write};

#[derive(Args)]
pub struct RemoveArgs {
    /// Package names to remove
    #[arg(required = true)]
    pub packages: Vec<String>,
    
    /// Skip confirmation prompts
    #[arg(short, long)]
    pub yes: bool,
    
    /// Remove dependencies that are no longer needed
    #[arg(long)]
    pub autoremove: bool,
    
    /// Dry run - show what would be removed without removing
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn execute(app: &mut App, args: RemoveArgs) -> Result<()> {
    println!("{} Preparing to remove packages...", style("🗑️").red());
    
    let mut packages_to_remove = Vec::new();
    let mut not_installed = Vec::new();
    
    // Check which packages are actually installed
    for package_name in &args.packages {
        if app.repository.is_installed(package_name).await? {
            if let Some(package) = app.repository.loader.get_package(package_name) {
                packages_to_remove.push(package.clone());
            }
        } else {
            not_installed.push(package_name);
        }
    }
    
    // Report packages that aren't installed
    if !not_installed.is_empty() {
        for pkg in &not_installed {
            crate::cli::print_warning(&format!("Package '{}' is not installed", pkg));
        }
    }
    
    if packages_to_remove.is_empty() {
        crate::cli::print_info("No packages to remove");
        return Ok(());
    }
    
    // Show removal plan
    show_removal_plan(&packages_to_remove, args.dry_run);
    
    if args.dry_run {
        return Ok(());
    }
    
    // Confirm removal
    if !args.yes && !confirm_removal(&packages_to_remove)? {
        crate::cli::print_info("Removal cancelled");
        return Ok(());
    }
    
    // Remove packages
    for package in &packages_to_remove {
        match remove_package(package, args.autoremove).await {
            Ok(_) => {
                crate::cli::print_success(&format!("Removed {}", package.name));
            }
            Err(e) => {
                crate::cli::print_error(&format!("Failed to remove {}: {}", package.name, e));
            }
        }
    }
    
    Ok(())
}

fn show_removal_plan(packages: &[crate::package::Package], dry_run: bool) {
    let action = if dry_run { "Would remove" } else { "Will remove" };
    
    println!("\n{} {} packages:", action, packages.len());
    for package in packages {
        println!("  {} {} ({})", 
            style("→").red(),
            style(&package.name).bold(),
            package.version
        );
    }
    println!();
}

fn confirm_removal(packages: &[crate::package::Package]) -> Result<bool> {
    print!("Continue with removal? [y/N]: ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    let input = input.trim().to_lowercase();
    Ok(input == "y" || input == "yes")
}

async fn remove_package(package: &crate::package::Package, autoremove: bool) -> Result<()> {
    use tokio::process::Command;
    
    match &package.installation {
        crate::package::Installation::Pacman { packages, .. } => {
            let mut cmd = Command::new("pacman");
            cmd.args(&["-R", "--noconfirm"]);
            
            if autoremove {
                cmd.arg("-s"); // Remove dependencies
            }
            
            cmd.args(packages);
            
            let output = cmd.output().await?;
            if !output.status.success() {
                return Err(crate::Error::InstallationFailed(format!(
                    "Failed to remove pacman packages: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
        }
        crate::package::Installation::Flatpak { id, .. } => {
            let output = Command::new("flatpak")
                .args(&["uninstall", "-y", id])
                .output()
                .await?;
            
            if !output.status.success() {
                return Err(crate::Error::InstallationFailed(format!(
                    "Failed to remove Flatpak: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
        }
        crate::package::Installation::Binary { install_path, .. } => {
            let expanded_path = shellexpand::tilde(install_path);
            let path = std::path::Path::new(expanded_path.as_ref());
            
            if path.exists() {
                tokio::fs::remove_file(path).await?;
            }
        }
        _ => {
            return Err(crate::Error::InstallationFailed(
                "Removal not implemented for this installation method".to_string()
            ));
        }
    }
    
    Ok(())
}