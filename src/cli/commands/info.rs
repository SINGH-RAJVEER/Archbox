use crate::{App, Result};
use clap::Args;
use console::style;

#[derive(Args)]
pub struct InfoArgs {
    /// Package name to show information for
    #[arg(required = true)]
    pub package: String,
    
    /// Show detailed dependency information
    #[arg(short, long)]
    pub dependencies: bool,
    
    /// Show installation method details
    #[arg(short, long)]
    pub installation: bool,
}

pub async fn execute(app: &App, args: InfoArgs) -> Result<()> {
    let package = app.repository.loader.get_package(&args.package)
        .ok_or_else(|| crate::Error::PackageNotFound(args.package.clone()))?;
    
    let installed = app.repository.is_installed(&package.name).await?;
    let status = if installed {
        style("Installed").green().bold()
    } else {
        style("Not Installed").yellow().bold()
    };
    
    // Basic information
    println!("{}", style(&package.name).cyan().bold().underlined());
    println!("Version: {}", style(&package.version).bold());
    println!("Status: {}", status);
    println!("Description: {}", package.description);
    
    if let Some(long_desc) = &package.long_description {
        println!("\n{}", style("Details:").bold());
        println!("{}", long_desc);
    }
    
    // Categories
    if !package.categories.is_empty() {
        println!("\n{} {}", 
            style("Categories:").bold(),
            package.categories.iter()
                .map(|c| style(c).cyan().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    
    // Dependencies
    if args.dependencies || !package.dependencies.is_empty() {
        println!("\n{}", style("Dependencies:").bold());
        if package.dependencies.is_empty() {
            println!("  None");
        } else {
            for dep in &package.dependencies {
                let dep_type = match dep.dep_type {
                    crate::package::DependencyType::System => "system",
                    crate::package::DependencyType::Package => "package",
                    crate::package::DependencyType::Runtime => "runtime",
                    crate::package::DependencyType::Build => "build",
                };
                
                let optional = if dep.optional { " (optional)" } else { "" };
                println!("  {} {} [{}]{}",
                    style("→").blue(),
                    style(&dep.name).bold(),
                    style(dep_type).dim(),
                    style(optional).dim()
                );
            }
        }
    }
    
    // Installation method
    if args.installation {
        println!("\n{}", style("Installation Method:").bold());
        match &package.installation {
            crate::package::Installation::Pacman { packages, flags } => {
                println!("  Method: pacman");
                println!("  Packages: {}", packages.join(", "));
                if let Some(flags) = flags {
                    println!("  Flags: {}", flags.join(" "));
                }
            }
            crate::package::Installation::Aur { package: pkg, helper } => {
                println!("  Method: AUR");
                println!("  Package: {}", pkg);
                if let Some(helper) = helper {
                    println!("  Helper: {}", helper);
                }
            }
            crate::package::Installation::Binary { url, install_path, .. } => {
                println!("  Method: Binary download");
                println!("  URL: {}", url);
                println!("  Install path: {}", install_path);
            }
            crate::package::Installation::Flatpak { id, remote } => {
                println!("  Method: Flatpak");
                println!("  ID: {}", id);
                println!("  Remote: {}", remote.as_deref().unwrap_or("flathub"));
            }
            _ => {
                println!("  Method: {:?}", package.installation);
            }
        }
    }
    
    // Metadata
    println!("\n{}", style("Metadata:").bold());
    if let Some(author) = &package.metadata.author {
        println!("  Author: {}", author);
    }
    if let Some(homepage) = &package.metadata.homepage {
        println!("  Homepage: {}", style(homepage).underlined());
    }
    if let Some(repository) = &package.metadata.repository {
        println!("  Repository: {}", style(repository).underlined());
    }
    if let Some(license) = &package.metadata.license {
        println!("  License: {}", license);
    }
    if let Some(size) = &package.metadata.size {
        println!("  Size: {}", size);
    }
    if let Some(tags) = &package.metadata.tags {
        println!("  Tags: {}", tags.join(", "));
    }
    
    Ok(())
}