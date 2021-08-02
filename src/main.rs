use ruget::RuGet;
use thisdiagnostic::DiagnosticResult;

#[async_std::main]
async fn main() -> DiagnosticResult<()> {
    Ok(RuGet::run().await?)
}
