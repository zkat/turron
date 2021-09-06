use turron::Turron;
use turron_common::{miette::Result, smol};

fn main() -> Result<()> {
    smol::block_on(Turron::load())?;
    Ok(())
}
