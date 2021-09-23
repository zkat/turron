use std::collections::HashMap;

use nu_table::{draw_table, StyledString, Table, TextStyle, Theme};
use nuget_api::v3::{NuGetClient, SearchQuery};
use turron_command::{
    async_trait::async_trait,
    clap::{self, Clap},
    log,
    turron_config::{self, TurronConfigLayer},
    TurronCommand,
};
use turron_common::{
    miette::{Context, IntoDiagnostic, Result},
    serde_json,
};

#[derive(Debug, Clap)]
pub struct SearchCmd {
    #[clap(about = "Search query", multiple = true)]
    query: Vec<String>,
    #[clap(
        about = "Source to search.",
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
    #[clap(about = "Number of results to show.", long, short = 'n')]
    take: Option<usize>,
    #[clap(about = "Number of results to skip.", long)]
    skip: Option<usize>,
    #[clap(about = "Include pre-releases", long)]
    prerelease: Option<bool>,
    #[clap(about = "Package type to filter by", long = "type")]
    package_type: Option<String>,
}

impl TurronConfigLayer for SearchCmd {
    fn layer_config(
        &mut self,
        matches: &turron_config::ArgMatches,
        config: &turron_config::TurronConfig,
    ) -> Result<()> {
        if !matches.is_present("source") {
            if let Ok(source) = config.get_str("commands.search.source") {
                self.source = source;
            } else if let Ok(source) = config.get_str("source") {
                self.source = source;
            }
        }
        if !matches.is_present("json") {
            if let Ok(json) = config.get_bool("commands.search.json") {
                self.json = json;
            } else if let Ok(json) = config.get_bool("json") {
                self.json = json;
            }
        }
        if !matches.is_present("take") {
            if let Ok(take) = config.get_str("commands.search.take") {
                self.take = Some(take.parse().into_diagnostic()?);
            } else if let Ok(take) = config.get_str("take") {
                self.take = Some(take.parse().into_diagnostic()?);
            }
        }
        if !matches.is_present("package_type") {
            if let Ok(val) = config.get_str("commands.search.package_type") {
                self.package_type = Some(val.parse().into_diagnostic()?);
            } else if let Ok(val) = config.get_str("package_type") {
                self.package_type = Some(val.parse().into_diagnostic()?);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl TurronCommand for SearchCmd {
    async fn execute(self) -> Result<()> {
        let client = NuGetClient::from_source(self.source.clone()).await?;

        let query = SearchQuery {
            query: Some(self.query.join(" ")),
            skip: self.skip,
            take: self.take,
            prerelease: self.prerelease,
            package_type: self.package_type,
        };

        let response = client.search(query).await?;

        if !self.quiet && self.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&response)
                    .into_diagnostic()
                    .context("Failed to serialize response back into JSON")?
            );
        } else if !self.quiet {
            let headers = vec!["id", "version", "description"]
                .iter()
                .map(|h| StyledString::new(h.to_string(), TextStyle::default_header()))
                .collect::<Vec<StyledString>>();
            let rows = response
                .data
                .iter()
                .map(|row| {
                    vec![
                        StyledString::new(row.id.clone(), TextStyle::basic_left()),
                        StyledString::new(row.version.clone(), TextStyle::basic_left()),
                        StyledString::new(
                            row.description.clone().unwrap_or_else(|| "".into()),
                            TextStyle::basic_left(),
                        ),
                    ]
                })
                .collect::<Vec<Vec<StyledString>>>();
            let width = if let Some((w, _)) = term_size::dimensions() {
                w
            } else {
                80
            };
            let table = Table::new(headers, rows, Theme::rounded());
            let color_hm: HashMap<String, nu_ansi_term::Style> = HashMap::new();
            let output_table = draw_table(&table, width, &color_hm);
            // Draw the table
            println!("{}", output_table);
            println!("Total hits: {}", response.total_hits);
        }
        Ok(())
    }
}
