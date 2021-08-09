use nuget_api::v3::{NuGetClient, RegistrationIndex, RegistrationLeaf};
use ruget_command::{
    async_trait::async_trait,
    clap::{self, Clap},
    log,
    ruget_config::{self, RuGetConfigLayer},
    serde_json, RuGetCommand,
};
use ruget_common::{
    miette::Diagnostic,
    miette_utils::{DiagnosticResult as Result, IntoDiagnostic},
    thiserror::{self, Error},
};
use ruget_semver::VersionReq;

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
        let (_index, leaf) = self.find_version().await?;
        if self.json && !self.quiet {
            // Just print the whole thing tbh
            println!(
                "{}",
                serde_json::to_string_pretty(&leaf)
                    .into_diagnostic(&"ruget::view::json_serialization")?
            );
        } else if !self.quiet {
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
