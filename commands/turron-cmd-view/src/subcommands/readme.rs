use dotnet_semver::Range;
use nuget_api::{v3::NuGetClient, NuGetApiError};
use turron_command::{
    async_trait::async_trait,
    clap::{self, Clap},
    turron_config::TurronConfigLayer,
    TurronCommand,
};
use turron_common::miette::{Report, Result};
use turron_package_spec::PackageSpec;

use crate::error::ViewError;

#[derive(Debug, Clap, TurronConfigLayer)]
#[config_layer = "view.readme"]
pub struct ReadmeCmd {
    #[clap(about = "Package spec to look up")]
    package: String,
    #[clap(
        about = "Source to view packages from",
        default_value = "https://api.nuget.org/v3/index.json",
        long
    )]
    source: String,
    #[clap(from_global)]
    quiet: bool,
    #[clap(from_global)]
    json: bool,
}

#[async_trait]
impl TurronCommand for ReadmeCmd {
    async fn execute(self) -> Result<()> {
        let package = self.package.parse()?;
        let client = NuGetClient::from_source(self.source.clone()).await?;
        let (package_id, requested) = if let PackageSpec::NuGet { name, requested } = &package {
            (name, requested.clone().unwrap_or_else(Range::any_floating))
        } else {
            return Err(ViewError::InvalidPackageSpec.into());
        };
        self.print_readme(&client, package_id, &requested).await
    }
}

impl ReadmeCmd {
    async fn print_readme(
        &self,
        client: &NuGetClient,
        package_id: &str,
        requested: &Range,
    ) -> Result<()> {
        let versions = client.versions(&package_id).await?;
        let version = turron_pick_version::pick_version(requested, &versions[..])
            .ok_or_else(|| ViewError::VersionNotFound(package_id.into(), requested.clone()))?;
        let nuspec = client.nuspec(package_id, &version).await?;
        if let Some(readme) = &nuspec.metadata.readme {
            let readme = readme.to_lowercase();
            let data = client
                .get_from_nupkg(package_id, &version, &readme)
                .await
                .map_err(|err| -> Report {
                    match err {
                        NuGetApiError::FileNotFound(_, _, _) => {
                            ViewError::ReadmeNotFound(nuspec.metadata.id, version).into()
                        }
                        _ => err.into(),
                    }
                })?;
            let readme_str = String::from_utf8(data).map_err(ViewError::InvalidUtf8)?;
            termimad::print_text(&readme_str);
            Ok(())
        } else {
            Err(ViewError::ReadmeNotFound(nuspec.metadata.id, version).into())
        }
    }
}
