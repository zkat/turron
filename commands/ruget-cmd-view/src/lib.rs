use nuget_api::v3::NuGetClient;
use ruget_command::{
    async_trait::async_trait,
    clap::{self, Clap},
    log,
    ruget_config::{self, RuGetConfigLayer},
    serde_json, RuGetCommand,
};
use ruget_common::miette_utils::{DiagnosticResult as Result, IntoDiagnostic};

#[derive(Debug, Clap, RuGetConfigLayer)]
pub struct ViewCmd {
    #[clap(about = "Name of package to view")]
    package_id: String,
    #[clap(
        about = "Source to view packages from",
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
}

#[async_trait]
impl RuGetCommand for ViewCmd {
    async fn execute(self) -> Result<()> {
        let client = NuGetClient::from_source(self.source.clone()).await?;
        let registration = client.metadata(&self.package_id).await?;
        if self.json && !self.quiet {
            // Just print the whole thing tbh
            println!(
                "{}",
                serde_json::to_string_pretty(&registration)
                    .into_diagnostic(&"ruget::view::json_serialization")?
            );
        } else if !self.quiet {
        }
        Ok(())
    }
}
