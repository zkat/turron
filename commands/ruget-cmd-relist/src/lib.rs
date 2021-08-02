use async_trait::async_trait;
use clap::Clap;
use nuget_api::v3::NuGetClient;
use ruget_command::RuGetCommand;
use ruget_config::RuGetConfigLayer;
use thisdiagnostic::{BoxDiagnostic, DiagnosticResult as Result};
use thiserror::Error;
use url::Url;

#[derive(Debug, Clap, RuGetConfigLayer)]
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
impl RuGetCommand for RelistCmd {
    async fn execute(self) -> Result<()> {
        let client = NuGetClient::from_source(self.source.clone())
            .await
            .box_diagnostic()?
            .with_key(self.api_key);
        client.unlist(self.id.clone(), self.version.clone()).await.box_diagnostic()?;
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
