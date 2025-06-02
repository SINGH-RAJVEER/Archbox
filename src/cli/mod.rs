pub mod commands;

use crate::{App, Result};
use clap::{Parser, Subcommand};
use console::style;

#[derive(Parser)]
#[command(name = "archbox")]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Configuration file path
    #[arg(short, long, global = true)]
    pub config: Option<std::path::PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Install packages from the repository
    Install(commands::install::InstallArgs),
    
    /// Search for packages in the repository
    Search(commands::search::SearchArgs),
    
    /// List available or installed packages
    List(commands::list::ListArgs),
    
    /// Update package definitions and system packages
    Update(commands::update::UpdateArgs),
    
    /// Show package information
    Info(commands::info::InfoArgs),
    
    /// Remove packages
    Remove(commands::remove::RemoveArgs),
    
    /// Configure application settings
    Config(commands::config::ConfigArgs),
}

impl Commands {
    pub async fn execute(self, app: &mut App) -> Result<()> {
        match self {
            Commands::Install(args) => commands::install::execute(app, args).await,
            Commands::Search(args) => commands::search::execute(app, args).await,
            Commands::List(args) => commands::list::execute(app, args).await,
            Commands::Update(args) => commands::update::execute(app, args).await,
            Commands::Info(args) => commands::info::execute(app, args).await,
            Commands::Remove(args) => commands::remove::execute(app, args).await,
            Commands::Config(args) => commands::config::execute(app, args).await,
        }
    }
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    
    let mut app = App::new().await?;
    
    // Set verbosity
    if cli.verbose {
        std::env::set_var("RUST_LOG", "archbox=debug");
    }
    
    // Handle color output
    if cli.no_color {
        console::set_colors_enabled(false);
    }
    
    cli.command.execute(&mut app).await
}

pub fn print_success(message: &str) {
    println!("{} {}", style("✓").green().bold(), message);
}

pub fn print_error(message: &str) {
    eprintln!("{} {}", style("✗").red().bold(), message);
}

pub fn print_warning(message: &str) {
    println!("{} {}", style("⚠").yellow().bold(), message);
}

pub fn print_info(message: &str) {
    println!("{} {}", style("ℹ").blue().bold(), message);
}