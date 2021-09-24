use std::path::PathBuf;

use directories::ProjectDirs;
use turron_command::TurronCommand;
use turron_command::{
    async_trait::async_trait,
    clap::{self, ArgMatches, Clap, FromArgMatches, IntoApp},
    tracing,
    turron_config::{TurronConfig, TurronConfigLayer, TurronConfigOptions},
};
use turron_common::miette::{Context, Result};

use turron_cmd_pack::PackCmd;
use turron_cmd_ping::PingCmd;
use turron_cmd_publish::PublishCmd;
use turron_cmd_relist::RelistCmd;
use turron_cmd_search::SearchCmd;
use turron_cmd_unlist::UnlistCmd;
use turron_cmd_view::ViewCmd;

#[derive(Debug, Clap)]
#[clap(
    author = "Kat Marchán <kzm@zkat.tech>",
    about = "Manage your NuGet packages.",
    version = clap::crate_version!(),
    setting = clap::AppSettings::ColoredHelp,
    setting = clap::AppSettings::DisableHelpSubcommand,
    setting = clap::AppSettings::DeriveDisplayOrder,
    setting = clap::AppSettings::InferSubcommands,
)]
pub struct Turron {
    #[clap(global = true, long = "root", about = "Package path to operate on.")]
    root: Option<PathBuf>,
    #[clap(global = true, about = "File to read configuration values from.", long)]
    config: Option<PathBuf>,
    #[clap(
        global = true,
        about = "Log verbosity level (off, error, warn, info, debug, trace)",
        long,
        short,
        default_value = "warn"
    )]
    verbosity: tracing::Level,
    #[clap(global = true, about = "Disable all output", long, short = 'q')]
    quiet: bool,
    #[clap(global = true, long, about = "Format output as JSON.")]
    json: bool,
    #[clap(
        global = true,
        long,
        short = 'k',
        about = "NuGet API key for the targeted NuGet source."
    )]
    api_key: Option<String>,
    #[clap(subcommand)]
    subcommand: TurronCmd,
}

impl Turron {
    fn setup_logging(&self) -> Result<()> {
        let mut collector = tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .without_time();
        if self.quiet {
            collector = collector.with_max_level(tracing_subscriber::filter::LevelFilter::OFF);
        } else {
            collector = collector.with_max_level(self.verbosity);
        }
        // TODO: Switch to try_init (ugh, `Box<dyn Error>` issues)
        if self.json {
            collector.json().init();
        } else {
            collector.init();
        }

        Ok(())
    }

    pub async fn load() -> Result<()> {
        let start = std::time::Instant::now();
        let clp = Turron::into_app();
        let matches = clp.get_matches();
        let mut turron = Turron::from_arg_matches(&matches);
        let cfg = if let Some(file) = &turron.config {
            TurronConfigOptions::new()
                .global_config_file(Some(file.clone()))
                .load()?
        } else {
            TurronConfigOptions::new()
                .global_config_file(
                    ProjectDirs::from("", "", "turron")
                        .map(|d| d.config_dir().to_owned().join("turronrc.toml")),
                )
                .pkg_root(turron.root.clone())
                .load()?
        };
        turron.layer_config(&matches, &cfg)?;
        turron.setup_logging().context("Failed to set up logging")?;
        turron.execute().await?;
        tracing::info!("Ran in {}s", start.elapsed().as_millis() as f32 / 1000.0);
        Ok(())
    }
}

#[derive(Debug, Clap)]
pub enum TurronCmd {
    #[clap(
        about = "Pack a project",
        setting = clap::AppSettings::ColoredHelp,
        setting = clap::AppSettings::DisableHelpSubcommand,
        setting = clap::AppSettings::DeriveDisplayOrder,
    )]
    Pack(PackCmd),
    #[clap(
        about = "Ping a source",
        setting = clap::AppSettings::ColoredHelp,
        setting = clap::AppSettings::DisableHelpSubcommand,
        setting = clap::AppSettings::DeriveDisplayOrder,
    )]
    Ping(PingCmd),
    #[clap(
        about = "Publish a package",
        setting = clap::AppSettings::ColoredHelp,
        setting = clap::AppSettings::DisableHelpSubcommand,
        setting = clap::AppSettings::DeriveDisplayOrder,
    )]
    Publish(PublishCmd),
    #[clap(
        about = "Relist a previously unlisted package version",
        setting = clap::AppSettings::ColoredHelp,
        setting = clap::AppSettings::DisableHelpSubcommand,
        setting = clap::AppSettings::DeriveDisplayOrder,
    )]
    Relist(RelistCmd),
    #[clap(
        about = "Search for packages",
        setting = clap::AppSettings::ColoredHelp,
        setting = clap::AppSettings::DisableHelpSubcommand,
        setting = clap::AppSettings::DeriveDisplayOrder,
    )]
    Search(SearchCmd),
    #[clap(
        about = "Unlist a package version",
        setting = clap::AppSettings::ColoredHelp,
        setting = clap::AppSettings::DisableHelpSubcommand,
        setting = clap::AppSettings::DeriveDisplayOrder,
    )]
    Unlist(UnlistCmd),
    #[clap(
        about = "View package info",
        setting = clap::AppSettings::ColoredHelp,
        setting = clap::AppSettings::DisableHelpSubcommand,
        setting = clap::AppSettings::DeriveDisplayOrder,
    )]
    View(ViewCmd),
}

#[async_trait]
impl TurronCommand for Turron {
    async fn execute(self) -> Result<()> {
        tracing::debug!("Running command: {:#?}", self.subcommand);
        match self.subcommand {
            TurronCmd::Pack(pack) => pack.execute().await,
            TurronCmd::Ping(ping) => ping.execute().await,
            TurronCmd::Publish(publish) => publish.execute().await,
            TurronCmd::Relist(relist) => relist.execute().await,
            TurronCmd::Search(search) => search.execute().await,
            TurronCmd::Unlist(unlist) => unlist.execute().await,
            TurronCmd::View(view) => view.execute().await,
        }
    }
}

impl TurronConfigLayer for Turron {
    fn layer_config(&mut self, args: &ArgMatches, conf: &TurronConfig) -> Result<()> {
        match self.subcommand {
            TurronCmd::Pack(ref mut pack) => {
                pack.layer_config(args.subcommand_matches("pack").unwrap(), conf)
            }
            TurronCmd::Ping(ref mut ping) => {
                ping.layer_config(args.subcommand_matches("ping").unwrap(), conf)
            }
            TurronCmd::Publish(ref mut publish) => {
                publish.layer_config(args.subcommand_matches("publish").unwrap(), conf)
            }
            TurronCmd::Relist(ref mut relist) => {
                relist.layer_config(args.subcommand_matches("relist").unwrap(), conf)
            }
            TurronCmd::Search(ref mut search) => {
                search.layer_config(args.subcommand_matches("search").unwrap(), conf)
            }
            TurronCmd::Unlist(ref mut unlist) => {
                unlist.layer_config(args.subcommand_matches("unlist").unwrap(), conf)
            }
            TurronCmd::View(ref mut view) => {
                view.layer_config(args.subcommand_matches("view").unwrap(), conf)
            }
        }
    }
}
