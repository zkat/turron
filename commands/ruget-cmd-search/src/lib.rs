use std::collections::HashMap;

use async_trait::async_trait;
use clap::Clap;
use nu_table::{draw_table, StyledString, Table, TextStyle, Theme};
use nuget_api::v3::{NuGetClient, SearchQuery};
use ruget_command::RuGetCommand;
use ruget_config::RuGetConfigLayer;
use ruget_diagnostics::{
    DiagnosticCategory, DiagnosticError, DiagnosticMetadata, DiagnosticResult as Result,
    IntoDiagnostic,
};
use url::Url;

#[derive(Debug, Clap, RuGetConfigLayer)]
pub struct SearchCmd {
    #[clap(about = "Search query")]
    query: String,
    #[clap(
        about = "Source to search.",
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
    #[clap(about = "Number of results to show.", long, short = 'n')]
    take: Option<usize>,
    #[clap(about = "Number of results to skip.", long)]
    skip: Option<usize>,
    #[clap(about = "Include pre-releases", long)]
    prerelease: Option<bool>,
    #[clap(about = "Package type to filter by", long = "type")]
    package_type: Option<String>,
}

#[async_trait]
impl RuGetCommand for SearchCmd {
    async fn execute(self) -> Result<()> {
        let client = NuGetClient::from_source(self.source.clone())
            .await
            .map_err(|e| DiagnosticError {
                category: DiagnosticCategory::Net,
                error: Box::new(e),
                label: "ruget::api::badsource".into(),
                advice: Some("Are you sure this is a valid NuGet source? Example: https://api.nuget.org/v3/index.json".into()),
                meta: Some(DiagnosticMetadata::Net {
                    url: self.source.to_string(),
                })
            })?;
        let query = SearchQuery {
            query: Some(self.query),
            skip: self.skip,
            take: self.take,
            prerelease: self.prerelease,
            package_type: self.package_type,
        };
        let response = client
            .search(query)
            .await
            .into_diagnostic("ruget::search::query")?;
        if !self.quiet && self.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&response).into_diagnostic("ruget::search::json")?
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