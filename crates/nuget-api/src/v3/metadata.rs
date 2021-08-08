use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
pub use surf::Body;
use surf::StatusCode;

use crate::errors::NuGetApiError;
use crate::v3::NuGetClient;

impl NuGetClient {
    pub async fn metadata(
        self,
        package_id: impl AsRef<str>,
    ) -> Result<RegistrationIndex, NuGetApiError> {
        use NuGetApiError::*;
        let url = self
            .endpoints
            .metadata
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
                Ok(
                    serde_json::from_str(&body).map_err(|e| NuGetApiError::BadJson {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistrationIndex {
    /// The number of registration pages in the index
    pub count: usize,
    /// The registration pages.
    pub items: Vec<RegistrationPage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistrationPage {
    /// The number of registration leaves in the page.
    pub count: usize,
    pub items: Vec<RegistrationLeaf>,
    pub lower: String,
    pub upper: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationLeaf {
    pub catalog_entry: CatalogEntry,
    pub package_content: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntry {
    pub id: String,
    pub version: String, // TODO: this will eventually be a (NuGet) semver version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authors: Option<Authors>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependency_groups: Option<Vec<DependencyGroup>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecation: Option<PackageDeprecation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_license_acceptance: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Tags>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vulnerabilities: Option<Vec<Vulnerability>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Authors {
    One(String),
    Many(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Tags {
    One(String),
    Many(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyGroup {
    pub target_framework: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<Dependency>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<String>, // TODO: what type is this, actually?...
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageDeprecation {
    pub reasons: Vec<DeprecationReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vulnerability {
    pub advisory_url: String,
    pub severity: Severity,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub enum DeprecationReason {
    Legacy,
    CriticalBugs,
    Other,
    #[serde(other)]
    Unknown,
}
