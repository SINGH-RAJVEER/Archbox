use crate::{App, Result};
use clap::Args;
use console::style;

#[derive(Args)]
pub struct SearchArgs {
    #[arg(required = true)]
    pub query: String,
    
    #[arg(short, long)]
    pub description: bool,
    
    #[arg(short, long)]
    pub verbose: bool,
    
    #[arg(short, long)]
    pub category: Option<String>,
    
    #[arg(long)]
    pub installed: bool,
}

pub async fn execute(app: &App, args: SearchArgs) -> Result<()> {
    println!("{} Searching for '{}'...", style("🔍").cyan(), args.query);
    
    let results = app.repository.search_packages(&args.query, &args).await?;
    
    if results.is_empty() {
        crate::cli::print_warning("No packages found matching the search criteria");
        return Ok(());
    }
    
    println!("\nFound {} package(s):\n", results.len());
    
    for package in results {
        print_package_result(&package, args.verbose, &app).await?;
    }
    
    Ok(())
}

async fn print_package_result(
    package: &crate::package::Package, 
    verbose: bool,
    app: &App
) -> Result<()> {
    let installed = app.repository.is_installed(&package.name).await?;
    let status = if installed {
        style("[installed]").green()
    } else {
        style("[available]").blue()
    };
    
    println!("{} {} {}", 
        style(&package.name).bold().cyan(),
        style(&package.version).dim(),
        status
    );
    
    if verbose {
        println!("  {}", package.description);
        if !package.categories.is_empty() {
            println!("  Categories: {}", package.categories.join(", "));
        }
        println!();
    }
    
    Ok(())
}