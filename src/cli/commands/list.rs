use crate::{App, Result};
use clap::Args;
use console::style;

#[derive(Args)]
pub struct ListArgs {
    #[arg(short, long)]
    pub installed: bool,
    
    #[arg(short, long)]
    pub available: bool,
    
    #[arg(short, long)]
    pub category: Option<String>,
    
    #[arg(short, long)]
    pub verbose: bool,
}

pub async fn execute(app: &App, args: ListArgs) -> Result<()> {
    let packages = app.repository.list_packages(&args).await?;
    
    if packages.is_empty() {
        crate::cli::print_warning("No packages found");
        return Ok(());
    }
    
    println!("Found {} package(s):\n", packages.len());
    
    for package in packages {
        let installed = app.repository.is_installed(&package.name).await?;
        
        if args.installed && !installed {
            continue;
        }
        if args.available && installed {
            continue;
        }
        
        print_package_entry(&package, installed, args.verbose);
    }
    
    Ok(())
}

fn print_package_entry(package: &crate::package::Package, installed: bool, verbose: bool) {
    let status_icon = if installed {
        style("●").green()
    } else {
        style("○").dim()
    };
    
    println!("{} {} {}", 
        status_icon,
        style(&package.name).bold(),
        style(&package.version).dim()
    );
    
    if verbose {
        println!("  {}", package.description);
        if !package.categories.is_empty() {
            println!("  Categories: {}", 
                package.categories.iter()
                    .map(|c| style(c).cyan().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        println!();
    }
}