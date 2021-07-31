use std::env;
use std::path::PathBuf;

use async_trait::async_trait;
use clap::{ArgMatches, Clap, FromArgMatches, IntoApp};
use directories::ProjectDirs;
use ruget_command::RuGetCommand;
use ruget_config::{RuGetConfig, RuGetConfigLayer, RuGetConfigOptions};
use ruget_diagnostics::{DiagnosticResult as Result, IntoDiagnostic};

use ruget_cmd_ping::PingCmd;

#[derive(Debug, Clap)]
#[clap(
    author = "Kat Marchán <kzm@zkat.tech>",
    about = "Manage your NuGet packages.",
    version = clap::crate_version!(),
    setting = clap::AppSettings::ColoredHelp,
    setting = clap::AppSettings::DisableHelpSubcommand,
    setting = clap::AppSettings::DeriveDisplayOrder,
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
            .into_diagnostic("ruget::load::logging")?;
        ruget.execute().await?;
        log::info!("Ran in {}s", start.elapsed().as_millis() as f32 / 1000.0);
        Ok(())
    }
}

#[derive(Debug, Clap)]
pub enum RuGetCmd {
    #[clap(
        about = "Ping the registry",
        setting = clap::AppSettings::ColoredHelp,
        setting = clap::AppSettings::DisableHelpSubcommand,
        setting = clap::AppSettings::DeriveDisplayOrder,
    )]
    Ping(PingCmd),
}

#[async_trait]
impl RuGetCommand for RuGet {
    async fn execute(self) -> Result<()> {
        log::info!("Running command: {:#?}", self.subcommand);
        match self.subcommand {
            RuGetCmd::Ping(ping) => ping.execute().await,
        }
    }
}

impl RuGetConfigLayer for RuGet {
    fn layer_config(&mut self, args: &ArgMatches, conf: &RuGetConfig) -> Result<()> {
        match self.subcommand {
            RuGetCmd::Ping(ref mut ping) => {
                ping.layer_config(&args.subcommand_matches("ping").unwrap(), conf)
            }
        }
    }
}
