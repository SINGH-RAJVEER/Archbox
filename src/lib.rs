pub mod cli;
pub mod config;
pub mod error;
pub mod package;
pub mod repository;

pub use error::{Error, Result};

/// The main application context
#[derive(Debug)]
pub struct App {
    pub config: config::Config,
    pub repository: repository::Manager,
}

impl App {
    /// Initialize a new application instance
    pub async fn new() -> Result<Self> {
        let config = config::Config::load()?;
        let repository = repository::Manager::new(&config).await?;
        
        Ok(Self { config, repository })
    }
}