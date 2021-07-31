use std::time::Instant;

use async_trait::async_trait;
use clap::Clap;
use nuget_api::v3::NuGetClient;
use ruget_command::RuGetCommand;
use ruget_config::RuGetConfigLayer;
use ruget_diagnostics::{DiagnosticResult as Result, IntoDiagnostic};
use url::Url;

#[derive(Debug, Clap, RuGetConfigLayer)]
pub struct PingCmd {
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
}

#[async_trait]
impl RuGetCommand for PingCmd {
    async fn execute(self) -> Result<()> {
        let start = Instant::now();
        if !self.quiet {
            eprintln!("ping: {}", self.source);
        }
        NuGetClient::from_source(self.source.clone())
            .await
            .into_diagnostic("ping::source")?;
        let time = start.elapsed().as_micros() as f32 / 1000.0;
        if !self.quiet {
            eprintln!("pong: {}ms", time);
        }
        Ok(())
    }
}
