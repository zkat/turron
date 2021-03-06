use turron_command::{
    async_trait::async_trait,
    clap::{self, ArgMatches, Clap},
    turron_config::{TurronConfig, TurronConfigLayer},
    TurronCommand,
};
use turron_common::{miette::Result, tracing};

use subcommands::{IconCmd, ReadmeCmd, SummaryCmd, VersionsCmd};

mod error;
mod subcommands;

#[derive(Debug, Clap)]
pub enum ViewSubCmd {
    #[clap(
        about = "Display a summary of package metadata",
        setting = clap::AppSettings::ColoredHelp,
        setting = clap::AppSettings::DisableHelpSubcommand,
        setting = clap::AppSettings::DeriveDisplayOrder,
    )]
    Summary(SummaryCmd),
    #[clap(
        about = "Display a list of package versions",
        setting = clap::AppSettings::ColoredHelp,
        setting = clap::AppSettings::DisableHelpSubcommand,
        setting = clap::AppSettings::DeriveDisplayOrder,
    )]
    Versions(VersionsCmd),
    #[clap(
        about = "Show package README, if any",
        setting = clap::AppSettings::ColoredHelp,
        setting = clap::AppSettings::DisableHelpSubcommand,
        setting = clap::AppSettings::DeriveDisplayOrder,
    )]
    Readme(ReadmeCmd),
    #[clap(
        about = "Show package icon, if any",
        setting = clap::AppSettings::ColoredHelp,
        setting = clap::AppSettings::DisableHelpSubcommand,
        setting = clap::AppSettings::DeriveDisplayOrder,
    )]
    Icon(IconCmd),
}

#[derive(Debug, Clap)]
#[clap(
    setting = clap::AppSettings::InferSubcommands,
)]
pub struct ViewCmd {
    #[clap(subcommand)]
    subcommand: ViewSubCmd,
}

#[async_trait]
impl TurronCommand for ViewCmd {
    async fn execute(self) -> Result<()> {
        tracing::debug!("Running command: {:#?}", self.subcommand);
        match self.subcommand {
            ViewSubCmd::Summary(summary) => summary.execute().await,
            ViewSubCmd::Readme(readme) => readme.execute().await,
            ViewSubCmd::Icon(icon) => icon.execute().await,
            ViewSubCmd::Versions(versions) => versions.execute().await,
        }
    }
}

impl TurronConfigLayer for ViewCmd {
    fn layer_config(&mut self, args: &ArgMatches, conf: &TurronConfig) -> Result<()> {
        match self.subcommand {
            ViewSubCmd::Icon(ref mut icon) => {
                icon.layer_config(args.subcommand_matches("icon").unwrap(), conf)
            }
            ViewSubCmd::Readme(ref mut readme) => {
                readme.layer_config(args.subcommand_matches("readme").unwrap(), conf)
            }
            ViewSubCmd::Versions(ref mut versions) => {
                versions.layer_config(args.subcommand_matches("versions").unwrap(), conf)
            }
            ViewSubCmd::Summary(ref mut summary) => {
                summary.layer_config(args.subcommand_matches("summary").unwrap(), conf)
            }
        }
    }
}
