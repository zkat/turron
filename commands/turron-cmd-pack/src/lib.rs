use turron_command::{
    async_trait::async_trait,
    clap::{self, Clap},
    turron_config::TurronConfigLayer,
    TurronCommand,
};
use turron_common::miette::Result;

#[derive(Debug, Clap, TurronConfigLayer)]
#[config_layer = "pack"]
pub struct PackCmd {
    #[clap(from_global)]
    quiet: bool,
    #[clap(from_global)]
    json: bool,
}

#[async_trait]
impl TurronCommand for PackCmd {
    async fn execute(self) -> Result<()> {
        turron_dotnet::pack().await?;
        Ok(())
    }
}
