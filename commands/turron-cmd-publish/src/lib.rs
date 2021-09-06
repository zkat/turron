use std::path::PathBuf;

use nuget_api::v3::{Body, NuGetClient};
use turron_command::{
    async_trait::async_trait,
    clap::{self, Clap},
    log,
    turron_config::{self, TurronConfigLayer},
    TurronCommand,
};
use turron_common::miette::{Context, IntoDiagnostic, Result};

#[derive(Debug, Clap, TurronConfigLayer)]
pub struct PublishCmd {
    #[clap(about = "Specific packages to publish, if not the current path")]
    nupkgs: Vec<PathBuf>,
    #[clap(
        about = "Source to ping",
        default_value = "https://api.nuget.org/v3/index.json",
        long
    )]
    source: String,
    #[clap(from_global)]
    loglevel: log::LevelFilter,
    #[clap(from_global)]
    quiet: bool,
    #[clap(from_global)]
    json: bool,
    #[clap(from_global)]
    api_key: Option<String>,
}

#[async_trait]
impl TurronCommand for PublishCmd {
    async fn execute(self) -> Result<()> {
        let client = NuGetClient::from_source(self.source.clone())
            .await?
            .with_key(self.api_key);
        let body = Body::from_file(&self.nupkgs[0])
            .await
            .into_diagnostic()
            .context("Failed to open provided nupkg")?;
        client.push(body).await?;
        Ok(())
    }
}
