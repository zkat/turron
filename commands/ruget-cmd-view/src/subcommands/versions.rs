use std::collections::HashMap;

use nu_table::{draw_table, StyledString, Table, TextStyle, Theme};
use nuget_api::v3::NuGetClient;
use ruget_command::{
    async_trait::async_trait,
    clap::{self, Clap},
    log,
    ruget_config::{self, RuGetConfigLayer},
    RuGetCommand,
};
use ruget_common::{
    chrono::Datelike,
    chrono_humanize::HumanTime,
    miette::{DiagnosticResult as Result, IntoDiagnostic},
    serde_json,
};
use ruget_package_spec::PackageSpec;

use crate::error::ViewError;

#[derive(Debug, Clap, RuGetConfigLayer)]
pub struct VersionsCmd {
    #[clap(about = "Package spec to look up")]
    package: String,
    #[clap(
        about = "Source to view packages from",
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
impl RuGetCommand for VersionsCmd {
    async fn execute(self) -> Result<()> {
        let package = self.package.parse()?;
        let client = NuGetClient::from_source(self.source.clone()).await?;
        let package_id = if let PackageSpec::NuGet { name, .. } = &package {
            name
        } else {
            return Err(ViewError::InvalidPackageSpec.into());
        };
        self.print_versions(&client, package_id).await
    }
}

impl VersionsCmd {
    async fn print_versions(&self, client: &NuGetClient, package_id: &str) -> Result<()> {
        let index = client.registration(package_id).await?;
        let mut versions = Vec::new();
        for page in index.items {
            let page = if page.items.is_some() {
                page
            } else {
                client.registration_page(&page.id).await?
            };
            for leaf in page
                .items
                .expect("RegistrationPage endpoints must have items!")
                .into_iter()
            {
                versions.push((leaf.catalog_entry.version, leaf.catalog_entry.published));
            }
        }
        versions.sort_unstable();
        if self.json && !self.quiet {
            let mut map = HashMap::new();
            for (version, published) in versions {
                map.insert(version, published);
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&map)
                    .into_diagnostic(&"ruget::view::json_serialization")?
            );
        } else if !self.quiet {
            let headers = vec!["version", "published_at"]
                .iter()
                .map(|h| StyledString::new(h.to_string(), TextStyle::default_header()))
                .collect::<Vec<StyledString>>();
            let rows = versions
                .iter()
                .map(|(v, p)| {
                    vec![
                        StyledString::new(v.to_string(), TextStyle::basic_left()),
                        StyledString::new(
                            p.map(|p| {
                                if p.year() > 1900 {
                                    HumanTime::from(p).to_string()
                                } else {
                                    "unlisted".into()
                                }
                            })
                            .unwrap_or_else(|| "unlisted".into()),
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
        }
        Ok(())
    }
}
