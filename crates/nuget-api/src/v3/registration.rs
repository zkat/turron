use dotnet_semver::{Range, Version};
pub use turron_common::surf::Body;
use turron_common::{
    chrono::{DateTime, Utc},
    serde::{Deserialize, Serialize},
    serde_json, serde_with,
    surf::{self, StatusCode, Url},
};

use crate::errors::NuGetApiError;
use crate::v3::NuGetClient;

impl NuGetClient {
    pub async fn registration_page(
        &self,
        page: impl AsRef<str>,
    ) -> Result<RegistrationPage, NuGetApiError> {
        use NuGetApiError::*;
        let url = Url::parse(page.as_ref())?;
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
                Ok(serde_json::from_str(&body)
                    .map_err(|e| NuGetApiError::from_json_err(e, url.into(), body))?)
            }
            StatusCode::NotFound => Err(RegistrationPageNotFound),
            code => Err(BadResponse(code)),
        }
    }

    pub async fn registration(
        &self,
        package_id: impl AsRef<str>,
    ) -> Result<RegistrationIndex, NuGetApiError> {
        use NuGetApiError::*;
        let url = self
            .endpoints
            .registration
            .clone()
            .ok_or_else(|| UnsupportedEndpoint("RegistrationsBaseUrl/3.6.0".into()))?
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
                Ok(serde_json::from_str(&body)
                    .map_err(|e| NuGetApiError::from_json_err(e, url.into(), body))?)
            }
            StatusCode::NotFound => Err(PackageNotFound),
            code => Err(BadResponse(code)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegistrationIndex {
    /// The number of registration pages in the index
    pub count: usize,
    /// The registration pages.
    pub items: Vec<RegistrationPage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegistrationPage {
    #[serde(rename = "@id")]
    pub id: String,
    pub parent: Option<String>,
    /// The number of registration leaves in the page.
    pub count: usize,
    pub items: Option<Vec<RegistrationLeaf>>,
    pub lower: Version,
    pub upper: Version,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationLeaf {
    pub catalog_entry: CatalogEntry,
    pub package_content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntry {
    pub id: String,
    pub version: Version,
    pub authors: Option<Authors>,
    pub dependency_groups: Option<Vec<DependencyGroup>>,
    pub deprecation: Option<PackageDeprecation>,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub license_url: Option<String>,
    pub license_expression: Option<String>,
    pub listed: Option<bool>,
    pub project_url: Option<String>,
    pub published: Option<DateTime<Utc>>,
    pub require_license_acceptance: Option<bool>,
    pub tags: Option<Tags>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub vulnerabilities: Option<Vec<Vulnerability>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Authors {
    One(String),
    Many(Vec<String>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Tags {
    One(String),
    Many(Vec<String>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde_with::skip_serializing_none]
pub struct DependencyGroup {
    pub target_framework: Option<String>,
    pub dependencies: Option<Vec<Dependency>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde_with::skip_serializing_none]
pub struct Dependency {
    pub id: String,
    pub range: Option<Range>,
}

impl PartialOrd for Dependency {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl Ord for Dependency {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde_with::skip_serializing_none]
pub struct PackageDeprecation {
    pub reasons: Vec<DeprecationReason>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vulnerability {
    pub advisory_url: String,
    pub severity: Severity,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Severity {
    #[serde(rename = "0")]
    Low,
    #[serde(rename = "1")]
    Moderate,
    #[serde(rename = "2")]
    High,
    #[serde(rename = "3")]
    Critical,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DeprecationReason {
    Legacy,
    CriticalBugs,
    Other,
    #[serde(other)]
    Unknown,
}
