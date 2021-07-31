use async_trait::async_trait;
use clap::Clap;
use nuget_api::v3::NuGetClient;
use ruget_command::RuGetCommand;
use ruget_config::RuGetConfigLayer;
use ruget_diagnostics::{
    DiagnosticCategory, DiagnosticError, DiagnosticMetadata, DiagnosticResult as Result,
};
use thiserror::Error;
use url::Url;

#[derive(Debug, Clap, RuGetConfigLayer)]
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
impl RuGetCommand for UnlistCmd {
    async fn execute(self) -> Result<()> {
        let client = NuGetClient::from_source(self.source.clone())
            .await
            .map_err(|e| DiagnosticError {
                category: DiagnosticCategory::Net,
                error: Box::new(e),
                label: "ruget::unlist::badsource".into(),
                advice: Some("Are you sure this is a valid NuGet source? Example: https://api.nuget.org/v3/index.json".into()),
                meta: Some(DiagnosticMetadata::Net {
                    url: self.source.to_string(),
                })
            })?.with_key(self.api_key.clone().ok_or_else(|| DiagnosticError {
                category: DiagnosticCategory::Misc,
                error: Box::new(UnlistError::MissingApiKey),
                label: "ruget::unlist::apikey".into(),
                advice: Some("Make sure an `api_key` is in your config file, or pass one with `--api-key <key>`".into()),
                meta: None,
            })?);
        client
            .unlist(self.id.clone(), self.version.clone())
            .await
            .map_err(|e| DiagnosticError {
                category: DiagnosticCategory::Net,
                error: Box::new(e),
                label: "ruget::unlist::missingpackage".into(),
                advice: Some("This can happen if your provided API key is invalid, or if the version you specified does not exist. Double-check both!".into()),
                meta: Some(DiagnosticMetadata::Net {
                    url: self.source.to_string(),
                })
            })?;
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
