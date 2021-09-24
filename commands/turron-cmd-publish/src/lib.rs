use std::{path::PathBuf, time::Duration};

use nuget_api::v3::{Body, NuGetClient};
use turron_command::{
    async_trait::async_trait,
    clap::{self, Clap},
    indicatif::ProgressBar,
    tracing,
    turron_config::{self, TurronConfigLayer},
    TurronCommand,
};
use turron_common::{
    miette::{Context, IntoDiagnostic, Result},
    smol::{self, Timer},
};

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
    verbosity: tracing::Level,
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
        let spinner = if self.quiet || self.json {
            ProgressBar::hidden()
        } else {
            ProgressBar::new_spinner()
        };
        let spin_clone = spinner.clone();
        let spin_fut = smol::spawn(async move {
            while !spin_clone.is_finished() {
                spin_clone.tick();
                Timer::after(Duration::from_millis(20)).await;
            }
        });

        let client = NuGetClient::from_source(self.source.clone())
            .await?
            .with_key(self.api_key);
        let body = Body::from_file(&self.nupkgs[0])
            .await
            .into_diagnostic()
            .context("Failed to open provided nupkg")?;

        spinner.println("Uploading nupkg...");

        client.push(body).await?;

        spinner.finish();
        spin_fut.await;
        Ok(())
    }
}
