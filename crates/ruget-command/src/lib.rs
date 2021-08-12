use ruget_common::miette_utils::DiagnosticResult as Result;

// Re-exports for common command deps:
pub use async_trait;
pub use clap;
pub use log;
pub use owo_colors;
pub use ruget_config;

#[async_trait::async_trait]
pub trait RuGetCommand {
    async fn execute(self) -> Result<()>;
}
