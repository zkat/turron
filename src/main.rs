use ruget::RuGet;
use ruget_common::{miette::Result, smol};

fn main() -> Result<()> {
    smol::block_on(RuGet::load())?;
    Ok(())
}
