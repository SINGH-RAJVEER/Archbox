use crate::{App, Result};
use clap::Args;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Args)]
pub struct InstallArgs {
    #[arg(required = true)]
    pub packages: Vec<String>,
    
    #[arg(short, long)]
    pub yes: bool,
    
    #[arg(long)]
    pub dry_run: bool,
    
    #[arg(short, long)]
    pub force: bool,
}

pub async fn execute(app: &mut App, args: InstallArgs) -> Result<()> {
    println!("{} Installing packages...", style("🔧").cyan());
    
    // Resolve package dependencies
    let packages = app.repository.resolve_packages(&args.packages).await?;
    
    if packages.is_empty() {
        crate::cli::print_warning("No packages found matching the criteria");
        return Ok(());
    }
    
    // Show installation plan
    show_installation_plan(&packages, args.dry_run);
    
    if args.dry_run {
        return Ok(());
    }
    
    // Confirm installation
    if !args.yes && !confirm_installation(&packages)? {
        crate::cli::print_info("Installation cancelled");
        return Ok(());
    }
    
    let pb = create_progress_bar(packages.len());
    
    for (i, package) in packages.iter().enumerate() {
        pb.set_message(format!("Installing {}", package.name));
        
        match app.repository.install_package(package, args.force).await {
            Ok(_) => {
                crate::cli::print_success(&format!("Installed {}", package.name));
            }
            Err(e) => {
                crate::cli::print_error(&format!("Failed to install {}: {}", package.name, e));
            }
        }
        
        pb.set_position(i as u64 + 1);
    }
    
    pb.finish_with_message("Installation complete");
    Ok(())
}

fn show_installation_plan(packages: &[crate::package::Package], dry_run: bool) {
    let action = if dry_run { "Would install" } else { "Will install" };
    
    println!("\n{} {} packages:", action, packages.len());
    for package in packages {
        println!("  {} {} ({})", 
            style("→").blue(),
            style(&package.name).bold(),
            package.version
        );
    }
    println!();
}

fn confirm_installation(packages: &[crate::package::Package]) -> Result<bool> {
    use std::io::{self, Write};
    
    print!("Continue with installation? [Y/n]: ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    let input = input.trim().to_lowercase();
    Ok(input.is_empty() || input == "y" || input == "yes")
}

fn create_progress_bar(len: usize) -> ProgressBar {
    let pb = ProgressBar::new(len as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-")
    );
    pb
}