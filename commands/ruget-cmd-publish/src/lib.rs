use std::path::PathBuf;

use async_trait::async_trait;
use clap::Clap;
use nuget_api::v3::{Body, NuGetClient};
use ruget_command::RuGetCommand;
use ruget_config::RuGetConfigLayer;
use thisdiagnostic::{DiagnosticResult as Result, IntoDiagnostic};
use url::Url;

#[derive(Debug, Clap, RuGetConfigLayer)]
pub struct PublishCmd {
    #[clap(about = "Specific packages to publish, if not the current path")]
    nupkgs: Vec<PathBuf>,
    #[clap(
        about = "Source to ping",
        default_value = "https://api.nuget.org/v3/index.json",
        long
    )]
    source: Url,
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
impl RuGetCommand for PublishCmd {
    async fn execute(self) -> Result<()> {
        let client = NuGetClient::from_source(self.source.clone())
            .await?
            .with_key(self.api_key);
        let body = Body::from_file(&self.nupkgs[0])
            .await
            .into_diagnostic("ruget::publish::bad_file")?;
        client.push(body).await?;
        Ok(())
    }
}
