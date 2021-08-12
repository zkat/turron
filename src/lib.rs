use std::env;
use std::path::PathBuf;

use directories::ProjectDirs;
use ruget_command::RuGetCommand;
use ruget_command::{
    async_trait::async_trait,
    clap::{self, ArgMatches, Clap, FromArgMatches, IntoApp},
    log,
    ruget_config::{RuGetConfig, RuGetConfigLayer, RuGetConfigOptions},
};
use ruget_common::miette_utils::{DiagnosticResult as Result, IntoDiagnostic};

use ruget_cmd_ping::PingCmd;
use ruget_cmd_publish::PublishCmd;
use ruget_cmd_relist::RelistCmd;
use ruget_cmd_search::SearchCmd;
use ruget_cmd_unlist::UnlistCmd;
use ruget_cmd_view::ViewCmd;

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
pub struct RuGet {
    #[clap(global = true, long = "root", about = "Package path to operate on.")]
    root: Option<PathBuf>,
    #[clap(global = true, about = "File to read configuration values from.", long)]
    config: Option<PathBuf>,
    #[clap(
        global = true,
        about = "Log output level (off, error, warn, info, debug, trace)",
        long,
        default_value = "warn"
    )]
    loglevel: log::LevelFilter,
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
    subcommand: RuGetCmd,
}

impl RuGet {
    fn setup_logging(&self) -> std::result::Result<(), fern::InitError> {
        let fern = fern::Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!(
                    "ruget [{}][{}] {}",
                    record.level(),
                    record.target(),
                    message,
                ))
            })
            .chain(
                fern::Dispatch::new()
                    .level(if self.quiet {
                        log::LevelFilter::Off
                    } else {
                        self.loglevel
                    })
                    .chain(std::io::stderr()),
            );
        // TODO: later
        // if let Some(logfile) = ProjectDirs::from("", "", "ruget")
        //     .map(|d| d.data_dir().to_owned().join(format!("ruget-debug-{}.log", chrono::Local::now().to_rfc3339())))
        // {
        //     fern = fern.chain(
        //         fern::Dispatch::new()
        //         .level(log::LevelFilter::Trace)
        //         .chain(fern::log_file(logfile)?)
        //     )
        // }
        fern.apply()?;
        Ok(())
    }

    pub async fn load() -> Result<()> {
        let start = std::time::Instant::now();
        let clp = RuGet::into_app();
        let matches = clp.get_matches();
        let mut ruget = RuGet::from_arg_matches(&matches);
        let cfg = if let Some(file) = &ruget.config {
            RuGetConfigOptions::new()
                .global_config_file(Some(file.clone()))
                .load()?
        } else {
            RuGetConfigOptions::new()
                .global_config_file(
                    ProjectDirs::from("", "", "ruget")
                        .map(|d| d.config_dir().to_owned().join("rugetrc.toml")),
                )
                .pkg_root(ruget.root.clone())
                .load()?
        };
        ruget.layer_config(&matches, &cfg)?;
        ruget
            .setup_logging()
            .into_diagnostic(&"ruget::load::logging")?;
        ruget.execute().await?;
        log::info!("Ran in {}s", start.elapsed().as_millis() as f32 / 1000.0);
        Ok(())
    }
}

#[derive(Debug, Clap)]
pub enum RuGetCmd {
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
impl RuGetCommand for RuGet {
    async fn execute(self) -> Result<()> {
        log::info!("Running command: {:#?}", self.subcommand);
        match self.subcommand {
            RuGetCmd::Ping(ping) => ping.execute().await,
            RuGetCmd::Publish(publish) => publish.execute().await,
            RuGetCmd::Relist(relist) => relist.execute().await,
            RuGetCmd::Search(search) => search.execute().await,
            RuGetCmd::Unlist(unlist) => unlist.execute().await,
            RuGetCmd::View(view) => view.execute().await,
        }
    }
}

impl RuGetConfigLayer for RuGet {
    fn layer_config(&mut self, args: &ArgMatches, conf: &RuGetConfig) -> Result<()> {
        match self.subcommand {
            RuGetCmd::Ping(ref mut ping) => {
                ping.layer_config(args.subcommand_matches("ping").unwrap(), conf)
            }
            RuGetCmd::Publish(ref mut publish) => {
                publish.layer_config(args.subcommand_matches("publish").unwrap(), conf)
            }
            RuGetCmd::Relist(ref mut relist) => {
                relist.layer_config(args.subcommand_matches("relist").unwrap(), conf)
            }
            RuGetCmd::Search(ref mut search) => {
                search.layer_config(args.subcommand_matches("search").unwrap(), conf)
            }
            RuGetCmd::Unlist(ref mut unlist) => {
                unlist.layer_config(args.subcommand_matches("unlist").unwrap(), conf)
            }
            RuGetCmd::View(ref mut view) => {
                view.layer_config(args.subcommand_matches("view").unwrap(), conf)
            }
        }
    }
}
