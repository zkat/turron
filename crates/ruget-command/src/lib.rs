use async_trait::async_trait;
use ruget_diagnostics::DiagnosticResult as Result;

#[async_trait]
pub trait RuGetCommand {
    async fn execute(self) -> Result<()>;
}
