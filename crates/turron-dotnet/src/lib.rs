use turron_common::smol::{self, process::Command};

pub use errors::DotnetError;

mod errors;

pub async fn pack() -> Result<(), DotnetError> {
    let cli_path = smol::unblock(|| which::which("dotnet")).await?;
    let status = Command::new(cli_path)
        .arg("pack")
        .arg("--nologo")
        .status()
        .await?;
    if status.success() {
        Ok(())
    } else {
        Err(DotnetError::PackFailed)
    }
}
