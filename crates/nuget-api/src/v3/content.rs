use std::sync::Arc;

pub use ruget_common::surf::Body;
use ruget_common::{
    quick_xml,
    serde::{Deserialize, Serialize},
    serde_json,
    surf::{self, StatusCode, Url},
};
use ruget_semver::Version;

use crate::errors::NuGetApiError;
use crate::v3::NuGetClient;

impl NuGetClient {
    pub async fn versions(
        &self,
        package_id: impl AsRef<str>,
    ) -> Result<Vec<Version>, NuGetApiError> {
        use NuGetApiError::*;
        let url = self
            .endpoints
            .package_content
            .clone()
            .ok_or_else(|| UnsupportedEndpoint("PackageBaseAddress/3.0.0".into()))?
            .join(&format!(
                "{}/index.json",
                &package_id.as_ref().to_lowercase()
            ))?;

        let req = surf::get(url.clone());

        let mut res = self
            .client
            .send(req)
            .await
            .map_err(|e| NuGetApiError::SurfError(e, url.clone().into()))?;

        match res.status() {
            StatusCode::Ok => {
                let body = res
                    .body_string()
                    .await
                    .map_err(|e| NuGetApiError::SurfError(e, url.clone().into()))?;
                Ok(serde_json::from_str::<PackageVersions>(&body)
                    .map_err(|e| NuGetApiError::BadJson {
                        source: e,
                        url: url.into(),
                        json: Arc::new(body),
                    })?
                    .versions)
            }
            StatusCode::NotFound => Err(PackageNotFound),
            code => Err(BadResponse(code)),
        }
    }

    pub async fn nuspec(
        &self,
        package_id: impl AsRef<str>,
        version: &Version,
    ) -> Result<NuSpec, NuGetApiError> {
        use NuGetApiError::*;

        // Version needs to undergo "normalization", which means lower-casing
        // and blowing away build.
        let mut version = version.clone();
        version.build.clear();

        let url = self
            .endpoints
            .package_content
            .clone()
            .ok_or_else(|| UnsupportedEndpoint("PackageBaseAddress/3.0.0".into()))?
            .join(&format!(
                "{}/{}/{}.nuspec",
                &package_id.as_ref().to_lowercase(),
                version.to_string().to_lowercase(),
                &package_id.as_ref().to_lowercase(),
            ))?;

        let req = surf::get(url.clone());

        let mut res = self
            .client
            .send(req)
            .await
            .map_err(|e| NuGetApiError::SurfError(e, url.clone().into()))?;

        match res.status() {
            StatusCode::Ok => {
                let body = res
                    .body_string()
                    .await
                    .map_err(|e| NuGetApiError::SurfError(e, url.clone().into()))?;
                Ok(
                    quick_xml::de::from_str(&body).map_err(|e| NuGetApiError::BadXml {
                        source: e,
                        url: url.into(),
                        json: Arc::new(body),
                    })?,
                )
            }
            StatusCode::NotFound => Err(PackageNotFound),
            code => Err(BadResponse(code)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageVersions {
    pub versions: Vec<Version>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename = "package")]
pub struct NuSpec {
    pub metadata: NuSpecMetadata,
    #[serde(default)]
    pub files: Vec<NuSpecFile>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NuSpecMetadata {
    // Required fields
    #[serde(rename = "$unflatten=id")]
    pub id: String,
    #[serde(rename = "$unflatten=version")]
    pub version: Version,
    #[serde(rename = "$unflatten=description")]
    pub description: String,
    // TODO: comma-separated
    #[serde(rename = "$unflatten=authors")]
    pub authors: String,

    // Attributes
    #[serde(rename = "minClientVersion")]
    pub min_client_version: Option<Version>,

    // Optional fields
    // TODO: comma-separated
    #[serde(rename = "$unflatten=owners")]
    pub owners: Option<String>,
    #[serde(rename = "$unflatten=projectUrl")]
    pub project_url: Option<Url>,
    #[serde(rename = "$unflatten=licenseUrl")]
    pub license_url: Option<Url>,
    #[serde(rename = "$unflatten=iconUrl")]
    pub icon_url: Option<Url>,
    #[serde(rename = "$unflatten=icon")]
    pub icon: Option<String>,
    #[serde(rename = "$unflatten=readme")]
    pub readme: Option<String>,
    #[serde(rename = "$unflatten=requireLicenseAcceptance")]
    pub require_license_acceptance: Option<bool>,
    #[serde(rename = "$unflatten=license")]
    pub license: Option<String>,
    #[serde(rename = "$unflatten=copyright")]
    pub copyright: Option<String>,
    #[serde(rename = "$unflatten=developmentDependency")]
    pub development_dependency: Option<bool>,
    #[serde(rename = "$unflatten=releaseNotes")]
    pub release_notes: Option<String>,
    // TODO: space-separated
    #[serde(rename = "$unflatten=tags")]
    pub tags: Option<String>,
    #[serde(rename = "$unflatten=language")]
    pub language: Option<String>,
    #[serde(rename = "$unflatten=repository")]
    pub repository: Option<NuSpecRepository>,

    // Collections
    #[serde(rename = "$unflatten=dependencies")]
    pub dependencies: Option<Vec<NuSpecDependency>>,
    #[serde(rename = "$unflatten=frameworkAssemblies")]
    pub framework_assemblies: Option<Vec<NuSpecFrameworkAssembly>>,
    #[serde(rename = "$unflatten=packageTypes")]
    pub package_types: Option<Vec<NuSpecPackageType>>,
    #[serde(rename = "$unflatten=references")]
    pub references: Option<Vec<NuSpecReference>>,
    #[serde(rename = "$unflatten=contentFiles")]
    pub content_files: Option<Vec<NuSpecContentFiles>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NuSpecRepository {
    #[serde(rename = "type")]
    pub repo_type: Option<String>,
    pub url: Option<Url>,
    pub branch: Option<String>,
    pub commit: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NuSpecFile {
    pub src: String,
    pub target: String,
    pub exclude: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NuSpecDependency {
    pub id: String,
    pub version: Version,
    pub exclude: Option<String>,
    pub include: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NuSpecFrameworkAssembly {
    #[serde(rename = "assemblyName")]
    pub assembly_name: String,
    #[serde(rename = "targetFramework")]
    pub target_framework: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NuSpecPackageType {
    Dependency,
    DotnetTool,
    Template,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NuSpecReferenceOrGroup {
    Group {
        #[serde(rename = "targetFramework")]
        target_framework: String,
        #[serde(rename = "reference", default)]
        references: Vec<NuSpecReference>,
    },
    Reference(NuSpecReference),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NuSpecReference {
    pub file: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NuSpecContentFiles {
    pub include: String,
    pub exclude: Option<String>,
    #[serde(rename = "buildAction")]
    pub build_action: Option<String>,
    #[serde(rename = "copyToOutput")]
    pub copy_to_output: Option<bool>,
    pub flatten: Option<bool>,
}
