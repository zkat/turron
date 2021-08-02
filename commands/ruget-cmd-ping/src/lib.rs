use std::time::Instant;

use async_trait::async_trait;
use clap::Clap;
use nuget_api::v3::NuGetClient;
use ruget_command::RuGetCommand;
use ruget_config::RuGetConfigLayer;
use serde_json::json;
use thisdiagnostic::{DiagnosticResult as Result, BoxDiagnostic, IntoDiagnostic};
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
    #[clap(from_global)]
    json: bool,
}

#[async_trait]
impl RuGetCommand for PingCmd {
    async fn execute(self) -> Result<()> {
        let start = Instant::now();
        if !self.quiet && !self.json {
            eprintln!("ping: {}", self.source);
        }
        let client = NuGetClient::from_source(self.source.clone())
            .await
            .box_diagnostic()?;
        let time = start.elapsed().as_micros() as f32 / 1000.0;
        if !self.quiet && self.json {
            let output = serde_json::to_string_pretty(&json!({
                "source": self.source.to_string(),
                "time": time,
                "endpoints": client.endpoints,
            }))
            .into_diagnostic("ruget::ping::serialize")?;
            println!("{}", output);
        }
        if !self.quiet && !self.json {
            eprintln!("pong: {}ms", time);
        }
        Ok(())
    }
}
