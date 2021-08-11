use std::collections::HashMap;

use nu_table::{draw_table, StyledString, Table, TextStyle, Theme};
use nuget_api::v3::{NuGetClient, RegistrationIndex, RegistrationLeaf, Tags};
use ruget_command::{
    async_trait::async_trait,
    clap::{self, Clap},
    log,
    owo_colors::{colors::*, OwoColorize},
    ruget_config::{self, RuGetConfigLayer},
    serde_json, RuGetCommand,
};
use ruget_common::{
    chrono::Datelike,
    chrono_humanize::HumanTime,
    miette::Diagnostic,
    miette_utils::{DiagnosticResult as Result, IntoDiagnostic},
    thiserror::{self, Error},
};
use ruget_package_spec::PackageSpec;
use ruget_semver::{Version, VersionReq};
use term_grid::{Cell, Direction, Filling, Grid, GridOptions};

#[derive(Debug, Clap, RuGetConfigLayer)]
pub struct ViewCmd {
    #[clap(about = "Package spec to look up")]
    package: PackageSpec,
    #[clap(
        about = "Specific attribute to print out for this package. Supported attributes: `version`."
    )]
    field: Option<String>,
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
impl RuGetCommand for ViewCmd {
    async fn execute(self) -> Result<()> {
        let client = NuGetClient::from_source(self.source.clone()).await?;
        let (package_id, requested) = if let PackageSpec::NuGet { name, requested } = &self.package
        {
            (name, requested.clone().unwrap_or_else(VersionReq::any))
        } else {
            return Err(ViewError::InvalidPackageSpec.into());
        };
        match &self.field.as_deref() {
            Some("versions") => self.print_versions(&client, package_id).await,
            None => {
                self.print_version_details(&client, package_id, &requested)
                    .await
            }
            _ => Err(ViewError::InvalidAttribute.into()),
        }
    }
}

impl ViewCmd {
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

    async fn print_version_details(
        &self,
        client: &NuGetClient,
        package_id: &str,
        requested: &VersionReq,
    ) -> Result<()> {
        let versions = client.versions(&package_id).await?;
        let version = self.pick_version(package_id, requested, versions).await?;
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

    async fn pick_version(
        &self,
        id: &str,
        req: &VersionReq,
        versions: Vec<Version>,
    ) -> Result<Version> {
        let pick = if req.is_floating() {
            versions.into_iter().rev().find(|v| req.satisfies(v))
        } else {
            versions.into_iter().find(|v| req.satisfies(v))
        };
        if let Some(pick) = pick {
            Ok(pick)
        } else {
            Err(ViewError::VersionNotFound(id.into(), req.clone()).into())
        }
    }

    async fn find_version(
        &self,
        client: &NuGetClient,
        package_id: &str,
        req: &VersionReq,
        version: &Version,
    ) -> Result<(RegistrationIndex, RegistrationLeaf)> {
        let index = client.registration(package_id).await?;
        for page in &index.items {
            let page_range: VersionReq = format!("[{}, {}]", page.lower, page.upper).parse()?;
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
        Err(Box::new(ViewError::VersionNotFound(
            package_id.into(),
            req.clone(),
        )))
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
                .unwrap_or_else(|| "Proprietary".into())
                .fg::<Green>(),
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

#[derive(Clone, Debug, Error)]
pub enum ViewError {
    #[error("Invalid package attribute requested")]
    InvalidAttribute,
    #[error("Only NuGet package specifiers are acceptable. Directories and git repositories are not supported... yet ��")]
    InvalidPackageSpec,
    #[error("Failed to find a version for {0} that satisfied {1}")]
    VersionNotFound(String, VersionReq),
}

impl Diagnostic for ViewError {
    fn code(&self) -> Box<dyn std::fmt::Display> {
        Box::new(match self {
            ViewError::VersionNotFound(_, _) => &"ruget::view::version_not_found",
            ViewError::InvalidPackageSpec => &"ruget::view::invalid_package_spec",
            ViewError::InvalidAttribute => &"ruget::view::invalid_attribute",
        })
    }

    fn help(&self) -> Option<Box<dyn std::fmt::Display>> {
        match self {
            // TODO: I guess this is good motivation to change miette...
            // ViewError::VersionNotFound(id, _) => Some(&format!("Try running `ruget view {} versions`", id))
            ViewError::InvalidPackageSpec => None,
            ViewError::VersionNotFound(_, _) => Some(&"Try running `ruget view <id> versions`"),
            ViewError::InvalidAttribute => {
                Some(&"Use `ruget view --help` to see what attributes are supported")
            }
        }
        .map(|s| -> Box<dyn std::fmt::Display> { Box::new(*s) })
    }
}
