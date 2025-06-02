use ArchBox::cli;
use ArchBox::Result;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("archbox=info".parse().unwrap()),
        )
        .init();

    // Run the CLI
    cli::run().await
}