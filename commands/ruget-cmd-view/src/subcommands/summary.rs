use nuget_api::v3::{NuGetClient, RegistrationIndex, RegistrationLeaf, Tags};
use ruget_command::{
    async_trait::async_trait,
    clap::{self, Clap},
    log,
    owo_colors::{colors::*, OwoColorize},
    ruget_config::{self, RuGetConfigLayer},
    RuGetCommand,
};
use ruget_common::{
    chrono_humanize::HumanTime,
    miette::{DiagnosticResult as Result, IntoDiagnostic},
    serde_json,
};
use ruget_package_spec::PackageSpec;
use ruget_semver::{Version, Range};
use term_grid::{Cell, Direction, Filling, Grid, GridOptions};

use crate::error::ViewError;

#[derive(Debug, Clap, RuGetConfigLayer)]
pub struct SummaryCmd {
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
impl RuGetCommand for SummaryCmd {
    async fn execute(self) -> Result<()> {
        let package = self.package.parse()?;
        let client = NuGetClient::from_source(self.source.clone()).await?;
        let (package_id, requested) = if let PackageSpec::NuGet { name, requested } = &package {
            (name, requested.clone().unwrap_or_else(Range::any))
        } else {
            return Err(ViewError::InvalidPackageSpec.into());
        };
        self.print_version_details(&client, package_id, &requested)
            .await
    }
}

impl SummaryCmd {
    async fn print_version_details(
        &self,
        client: &NuGetClient,
        package_id: &str,
        requested: &Range,
    ) -> Result<()> {
        let versions = client.versions(&package_id).await?;
        let version = ruget_pick_version::pick_version(requested, &versions[..])
            .ok_or_else(|| ViewError::VersionNotFound(package_id.into(), requested.clone()))?;
        let (index, leaf) = self
            .find_version(client, package_id, requested, &version)
            .await?;
        if self.json && !self.quiet {
            // Just print the whole thing tbh
            println!(
                "{}",
                serde_json::to_string_pretty(&leaf)
                    .into_diagnostic(&"ruget::view::json_serialization")?
            );
        } else if !self.quiet {
            self.print_package_details(&index, &leaf);
        }
        Ok(())
    }

    async fn find_version(
        &self,
        client: &NuGetClient,
        package_id: &str,
        req: &Range,
        version: &Version,
    ) -> Result<(RegistrationIndex, RegistrationLeaf)> {
        let index = client.registration(package_id).await?;
        for page in &index.items {
            let page_range: Range = format!("[{}, {}]", page.lower, page.upper).parse()?;
            if page_range.satisfies(version) {
                let page = if page.items.is_some() {
                    page.clone()
                } else {
                    client.registration_page(&page.id).await?
                };
                for leaf in page
                    .items
                    .expect("RegistrationPage endpoints must have items!")
                    .into_iter()
                {
                    if version == &leaf.catalog_entry.version {
                        return Ok((index, leaf));
                    }
                }
            }
        }
        Err(ViewError::VersionNotFound(package_id.into(), req.clone()).into())
    }

    fn print_package_details(&self, index: &RegistrationIndex, leaf: &RegistrationLeaf) {
        self.print_header(index, leaf);
        println!();
        self.print_tags(leaf);
        println!();
        self.print_nupkg_details(leaf);
        self.print_dependencies(leaf);
        println!();
        self.print_publish_time(leaf);
    }

    fn print_header(&self, index: &RegistrationIndex, leaf: &RegistrationLeaf) {
        let mut total_versions = 0usize;
        for page in &index.items {
            total_versions += page.count;
        }
        let entry = &leaf.catalog_entry;
        let total_deps = 0;
        println!(
            "{}@{} | {} | deps: {} | versions: {}",
            entry.id.fg::<BrightGreen>().underline(),
            entry.version.to_string().fg::<BrightGreen>().underline(),
            entry
                .license_expression
                .clone()
                .and_then(|l| if l.is_empty() {
                    None
                } else {
                    Some(l.fg::<Green>().to_string())
                })
                .unwrap_or_else(|| "No License".fg::<Red>().to_string()),
            total_deps.to_string().fg::<Yellow>(),
            total_versions.to_string().fg::<Yellow>(),
        );
        if let Some(desc) = &entry.description {
            println!("{}", desc);
        }
        if let Some(url) = &entry.project_url {
            println!("{}", url.fg::<Cyan>());
        }
        if let Some(depr) = &entry.deprecation {
            print!("⚠ {}", "DEPRECATED".bright_red());
            if let Some(msg) = &depr.message {
                print!(" - {}", msg);
            }
            println!()
        }
    }

    fn print_tags(&self, leaf: &RegistrationLeaf) {
        let entry = &leaf.catalog_entry;
        match &entry.tags {
            Some(Tags::One(tag)) => {
                println!("Tags: {}", tag.fg::<Yellow>());
            }
            Some(Tags::Many(tags)) => {
                println!(
                    "Tags: {}",
                    tags.iter()
                        .map(|t| t.fg::<Yellow>().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            None => {}
        }
    }

    fn print_nupkg_details(&self, leaf: &RegistrationLeaf) {
        println!("Nupkg: {}", leaf.package_content.fg::<Cyan>());
        // TODO: How tf do I get the nupkg hash?...
    }

    fn print_dependencies(&self, leaf: &RegistrationLeaf) {
        let entry = &leaf.catalog_entry;
        if let Some(groups) = &entry.dependency_groups {
            for group in groups {
                if let Some(deps) = &group.dependencies {
                    if !deps.is_empty() {
                        println!(
                            "\nDependencies for {}:",
                            group
                                .target_framework
                                .clone()
                                .unwrap_or_else(|| "this package".into())
                                .fg::<BrightCyan>()
                        );
                        let max_deps = 25_usize;
                        let mut grid = Grid::new(GridOptions {
                            filling: Filling::Spaces(3),
                            direction: Direction::TopToBottom,
                        });
                        let width = term_size::dimensions().map(|(w, _)| w).unwrap_or(80);
                        let mut deps = deps.clone();
                        deps.sort();
                        let mut vals = Vec::new();
                        for dep in deps.iter().take(max_deps) {
                            let mut val = dep.id.clone().fg::<Yellow>().to_string();
                            if let Some(range) = &dep.range {
                                val.push_str(&format!(": {}", range));
                            }
                            vals.push(val.clone());
                            grid.add(Cell::from(val));
                        }
                        if let Some(out) = grid.fit_into_width(width) {
                            print!("{}", out);
                        } else {
                            // Too wide. Print one per line.
                            for val in &vals {
                                println!("{}", val);
                            }
                        }
                        let count = deps.len();
                        if count > max_deps {
                            println!("(...and {} more)", count - max_deps);
                        }
                    }
                }
            }
        }
    }

    fn print_publish_time(&self, leaf: &RegistrationLeaf) {
        let entry = &leaf.catalog_entry;
        if let Some(published) = &entry.published {
            println!(
                "Published to {} {}",
                self.source.fg::<Cyan>(),
                HumanTime::from(*published).to_string().fg::<Yellow>()
            );
        }
    }
}
