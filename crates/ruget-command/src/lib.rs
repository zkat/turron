use async_trait::async_trait;
use ruget_common::miette::Diagnostic;

#[async_trait]
pub trait RuGetCommand {
    async fn execute(self) -> Result<(), Box<dyn Diagnostic + Send + Sync + 'static>>;
}
