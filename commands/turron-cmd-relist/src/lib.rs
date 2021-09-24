use nuget_api::v3::NuGetClient;
use turron_command::{
    async_trait::async_trait,
    clap::{self, Clap},
    turron_config::TurronConfigLayer,
    TurronCommand,
};
use turron_common::{miette::Result, thiserror::Error};

#[derive(Debug, Clap, TurronConfigLayer)]
#[config_layer = "relist"]
pub struct RelistCmd {
    #[clap(about = "ID of package to relist")]
    id: String,
    #[clap(about = "Version of package to relist")]
    version: String,
    #[clap(
        about = "Source for package",
        default_value = "https://api.nuget.org/v3/index.json",
        long
    )]
    source: String,
    #[clap(from_global)]
    quiet: bool,
    #[clap(from_global)]
    json: bool,
    #[clap(from_global)]
    api_key: Option<String>,
}

#[async_trait]
impl TurronCommand for RelistCmd {
    async fn execute(self) -> Result<()> {
        let client = NuGetClient::from_source(self.source.clone())
            .await?
            .with_key(self.api_key);
        client.relist(self.id.clone(), self.version.clone()).await?;
        if !self.quiet {
            println!("{}@{} has been relisted. This may take several hours to process.", self.id, self.version);
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum UnlistError {
    /// Api Key is missing.
    #[error("Missing API key")]
    MissingApiKey,
}
