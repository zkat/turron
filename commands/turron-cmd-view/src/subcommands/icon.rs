use nuget_api::{v3::NuGetClient, NuGetApiError};
use turron_command::{
    async_trait::async_trait,
    clap::{self, Clap},
    log,
    turron_config::TurronConfigLayer,
    TurronCommand,
};
use turron_common::miette::{Context, IntoDiagnostic, Report, Result};
use turron_package_spec::PackageSpec;
use turron_semver::Range;

use crate::error::ViewError;

#[derive(Debug, Clap, TurronConfigLayer)]
#[config_layer = "view.icon"]
pub struct IconCmd {
    #[clap(about = "Package spec to look up")]
    package: String,
    #[clap(
        about = "Height, in pixels, that the image should be rendered at",
        long,
        default_value = "15"
    )]
    height: u32,
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
impl TurronCommand for IconCmd {
    async fn execute(self) -> Result<()> {
        let package = self.package.parse()?;
        let client = NuGetClient::from_source(self.source.clone()).await?;
        let (package_id, requested) = if let PackageSpec::NuGet { name, requested } = &package {
            (name, requested.clone().unwrap_or_else(Range::any_floating))
        } else {
            return Err(ViewError::InvalidPackageSpec.into());
        };
        self.print_icon(&client, package_id, &requested).await
    }
}

impl IconCmd {
    async fn print_icon(
        &self,
        client: &NuGetClient,
        package_id: &str,
        requested: &Range,
    ) -> Result<()> {
        let versions = client.versions(&package_id).await?;
        let version = turron_pick_version::pick_version(requested, &versions[..])
            .ok_or_else(|| ViewError::VersionNotFound(package_id.into(), requested.clone()))?;
        let nuspec = client.nuspec(package_id, &version).await?;
        if let Some(icon) = &nuspec.metadata.icon {
            let icon = icon.to_lowercase();
            let data = client
                .get_from_nupkg(package_id, &version, &icon)
                .await
                .map_err(|err| -> Report {
                    match err {
                        NuGetApiError::FileNotFound(_, _, _) => {
                            ViewError::IconNotFound(nuspec.metadata.id, version).into()
                        }
                        _ => err.into(),
                    }
                })?;
            let conf = viuer::Config {
                transparent: true,
                absolute_offset: false,
                height: Some(self.height),
                ..Default::default()
            };
            let img = image::load_from_memory(&data)
                .into_diagnostic()
                .context("Failed to load image into memory")?;
            viuer::print(&img, &conf)
                .into_diagnostic()
                .context("Failed to print image to terminal")?;
            Ok(())
        } else {
            Err(ViewError::IconNotFound(nuspec.metadata.id, version).into())
        }
    }
}
