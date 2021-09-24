use nuget_api::v3::NuGetClient;
use turron_command::{
    async_trait::async_trait,
    clap::{self, Clap},
    turron_config::TurronConfigLayer,
    TurronCommand,
};
use turron_common::{miette::Result, thiserror::Error};

#[derive(Debug, Clap, TurronConfigLayer)]
#[config_layer = "unlist"]
pub struct UnlistCmd {
    #[clap(about = "ID of package to unlist")]
    id: String,
    #[clap(about = "Version of package to unlist")]
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
impl TurronCommand for UnlistCmd {
    async fn execute(self) -> Result<()> {
        let client = NuGetClient::from_source(self.source.clone())
            .await?
            .with_key(self.api_key);
        client.unlist(self.id.clone(), self.version.clone()).await?;
        if !self.quiet {
            println!("{}@{} has been unlisted.", self.id, self.version);
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
