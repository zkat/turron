use std::{path::PathBuf, time::Duration};

use nuget_api::v3::{Body, NuGetClient};
use turron_command::{
    async_trait::async_trait,
    clap::{self, Clap},
    indicatif::ProgressBar,
    turron_config::TurronConfigLayer,
    TurronCommand,
};
use turron_common::{
    miette::{Context, IntoDiagnostic, Result},
    smol::{self, Timer},
    tracing,
};

#[derive(Debug, Clap, TurronConfigLayer)]
#[config_layer = "publish"]
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

        spinner.println(format!("Uploading nupkg to {}...", self.source));

        client.push(body).await?;

        spinner.println("...package upload succeeded.");
        spinner.finish();
        spin_fut.await;
        Ok(())
    }
}
