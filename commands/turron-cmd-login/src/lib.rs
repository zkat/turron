use turron_command::{
    async_trait::async_trait,
    clap::{self, Clap},
    dialoguer::{Confirm, Input},
    directories::ProjectDirs,
    turron_config::TurronConfigLayer,
    TurronCommand,
};
use turron_common::{
    miette::{miette, Context, IntoDiagnostic, Result},
    smol::{
        self,
        fs::{self, OpenOptions},
        io::AsyncWriteExt,
    },
};

#[derive(Debug, Clap, TurronConfigLayer)]
#[config_layer = "login"]
pub struct LoginCmd {
    #[clap(from_global)]
    api_key: Option<String>,
}

#[async_trait]
impl TurronCommand for LoginCmd {
    async fn execute(self) -> Result<()> {
        if self.api_key.is_some() {
            let confirm = smol::unblock(|| -> Result<bool> {
                Confirm::new()
                    .with_prompt("You already have an API key configured. Continue?")
                    .default(true)
                    .interact()
                    .into_diagnostic()
            })
            .await?;
            if !confirm {
                return Ok(());
            }
        }

        let key = smol::unblock(|| -> Result<String> {
            Input::new()
                .with_prompt("Please paste an API token generated from https://www.nuget.org/account/apikeys")
                .interact_text()
                .into_diagnostic()
                .context("Failed to read api key")
        }).await?;

        let config = ProjectDirs::from("", "", "turron")
            .map(|d| d.config_dir().to_owned().join("turron.kdl"))
            .ok_or_else(|| miette!("Failed to calculate config file location."))?;

        fs::create_dir_all(config.parent().unwrap())
            .await
            .into_diagnostic()
            .context("Failed to create directories for config file location")?;

        OpenOptions::new()
            .append(true)
            .create(true)
            .open(&config)
            .await
            .into_diagnostic()
            .context("Failed to open turron config file")?
            .write_all(format!("\napi_key \"{}\"\n", key).as_bytes())
            .await
            .into_diagnostic()
            .context("Failed to append key to config file")?;

        println!("API Key written to {}.", config.display());
        Ok(())
    }
}
