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

#[derive(Debug, Clap)]
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

impl TurronConfigLayer for PublishCmd {
    fn layer_config(
        &mut self,
        matches: &turron_config::ArgMatches,
        config: &turron_config::TurronConfig,
    ) -> Result<()> {
        if !matches.is_present("source") {
            if let Ok(source) = config.get_str("source") {
                self.source = source;
            }
        }
        Ok(())
    }
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
