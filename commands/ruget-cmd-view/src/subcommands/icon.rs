use std::io::{Cursor, Read};

use nuget_api::v3::NuGetClient;
use ruget_command::{
    async_trait::async_trait,
    clap::{self, Clap},
    log,
    ruget_config::{self, RuGetConfigLayer},
    RuGetCommand,
};
use ruget_common::miette_utils::{DiagnosticResult as Result, IntoDiagnostic};
use ruget_package_spec::PackageSpec;
use ruget_semver::{Version, VersionReq};
use zip::ZipArchive;

use crate::error::ViewError;

#[derive(Debug, Clap, RuGetConfigLayer)]
pub struct IconCmd {
    #[clap(about = "Package spec to look up")]
    package: PackageSpec,
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
impl RuGetCommand for IconCmd {
    async fn execute(self) -> Result<()> {
        let client = NuGetClient::from_source(self.source.clone()).await?;
        let (package_id, requested) = if let PackageSpec::NuGet { name, requested } = &self.package
        {
            (name, requested.clone().unwrap_or_else(VersionReq::any))
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
        requested: &VersionReq,
    ) -> Result<()> {
        let versions = client.versions(&package_id).await?;
        let version = self.pick_version(package_id, requested, versions).await?;
        let nuspec = client.nuspec(package_id, &version).await?;
        if let Some(icon) = &nuspec.metadata.icon {
            let icon = icon.to_lowercase();
            let nupkg = Cursor::new(client.nupkg(package_id, &version).await?);
            let mut zip = ZipArchive::new(nupkg).into_diagnostic(&"ruget::view::nupkg_open")?;
            for i in 0..zip.len() {
                let mut file = zip
                    .by_index(i)
                    .into_diagnostic(&"ruget::view::nupkg_read_file")?;
                if file.is_file() && file.name().to_lowercase() == icon {
                    let mut buf = Vec::new();
                    file.read_to_end(&mut buf)
                        .into_diagnostic(&"ruget::view::nupkg_read_file")?;

                    let conf = viuer::Config {
                        transparent: true,
                        absolute_offset: false,
                        ..Default::default()
                    };
                    let img = image::load_from_memory(&buf)
                        .into_diagnostic(&"ruget::view::icon::image_load")?;
                    viuer::print(&img, &conf).into_diagnostic(&"ruget::view::icon::image_print")?;
                    return Ok(());
                }
            }
            Err(ViewError::IconNotFound(nuspec.metadata.id, version).into())
        } else {
            Err(ViewError::IconNotFound(nuspec.metadata.id, version).into())
        }
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
}
