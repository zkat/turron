use std::time::Instant;

use nuget_api::v3::NuGetClient;
use turron_command::{
    async_trait::async_trait,
    clap::{self, Clap},
    log,
    turron_config::{self, TurronConfigLayer},
    TurronCommand,
};
use turron_common::{
    miette::{Context, IntoDiagnostic, Result},
    serde_json::{self, json},
};

#[derive(Debug, Clap, TurronConfigLayer)]
pub struct PingCmd {
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
}

#[async_trait]
impl TurronCommand for PingCmd {
    async fn execute(self) -> Result<()> {
        let start = Instant::now();
        if !self.quiet && !self.json {
            eprintln!("ping: {}", self.source);
        }
        let client = NuGetClient::from_source(self.source.clone()).await?;
        let time = start.elapsed().as_micros() as f32 / 1000.0;
        if !self.quiet && self.json {
            let output = serde_json::to_string_pretty(&json!({
                "source": self.source.to_string(),
                "time": time,
                "endpoints": client.endpoints,
            }))
            .into_diagnostic()
            .context("Failed to serialize JSON ping output.")?;
            println!("{}", output);
        }
        if !self.quiet && !self.json {
            eprintln!("pong: {}ms", time);
        }
        Ok(())
    }
}