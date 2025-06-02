use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Package not found: {0}")]
    PackageNotFound(String),

    #[error("Installation failed: {0}")]
    InstallationFailed(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("System command failed: {message}")]
    CommandFailed { message: String },

    #[error("Permission denied: {operation}")]
    PermissionDenied { operation: String },

    #[error("Dependency error: {0}")]
    Dependency(String),
}