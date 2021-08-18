use ruget::RuGet;
use ruget_common::{miette::DiagnosticResult, smol};

fn main() -> DiagnosticResult<()> {
    smol::block_on(async { RuGet::load().await })?;
    Ok(())
}
