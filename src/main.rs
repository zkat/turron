use ruget::RuGet;
use ruget_diagnostics::DiagnosticResult;

#[async_std::main]
async fn main() -> DiagnosticResult<()> {
    Ok(RuGet::load().await?)
}
