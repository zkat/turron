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
    chrono_humanize::HumanTime,
    miette::Diagnostic,
    miette_utils::{DiagnosticResult as Result, IntoDiagnostic},
    thiserror::{self, Error},
};
use ruget_semver::VersionReq;
use term_grid::{Cell, Direction, Filling, Grid, GridOptions};

#[derive(Debug, Clap, RuGetConfigLayer)]
pub struct ViewCmd {
    #[clap(about = "Name of package to view")]
    package_id: String,
    #[clap(
        about = "Package version to view. Defaults to whatever the highest version is.",
        default_value = "*"
    )]
    package_req: VersionReq,
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
        let (index, leaf) = self.find_version().await?;
        if self.json && !self.quiet {
            // Just print the whole thing tbh
            println!(
                "{}",
                serde_json::to_string_pretty(&leaf)
                    .into_diagnostic(&"ruget::view::json_serialization")?
            );
        } else if !self.quiet {
            self.print_info(&index, &leaf);
        }
        Ok(())
    }
}

impl ViewCmd {
    async fn find_version(&self) -> Result<(RegistrationIndex, RegistrationLeaf)> {
        let client = NuGetClient::from_source(self.source.clone()).await?;
        let index = client.registration(&self.package_id).await?;
        for page in index.items.iter().rev() {
            let page_range: VersionReq = format!("[{}, {}]", page.lower, page.upper).parse()?;
            if let Some(intersection) = page_range.intersect(&self.package_req) {
                let page = if page.items.is_some() {
                    page.clone()
                } else {
                    client.registration_page(&page.id).await?
                };
                for leaf in page
                    .items
                    .expect("RegistrationPage endpoints must have items!")
                    .into_iter()
                    .rev()
                {
                    if intersection.satisfies(&leaf.catalog_entry.version) {
                        return Ok((index, leaf));
                    }
                }
            }
        }
        Err(Box::new(ViewError::VersionNotFound(
            self.package_req.clone(),
        )))
    }

    fn print_info(&self, index: &RegistrationIndex, leaf: &RegistrationLeaf) {
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
    #[error("Failed to find a version that satisfied {0}")]
    VersionNotFound(VersionReq),
}

impl Diagnostic for ViewError {
    fn code(&self) -> &(dyn std::fmt::Display) {
        match self {
            ViewError::VersionNotFound(_) => &"ruget::view::version_not_found",
        }
    }
}
