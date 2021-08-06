use serde::{Deserialize, Serialize};
pub use surf::Body;
use surf::{StatusCode, Url};

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
                    .map_err(|e| NuGetApiError::SurfError(e, url.into()))?;
                Ok(serde_json::from_str(body.trim()).map_err(|_| BadJson)?)
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
    pub items: Option<RegistrationLeaf>,
    pub lower: String,
    pub upper: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistrationLeaf {
    #[serde(rename = "catalogEntry")]
    pub catalog_entry: serde_json::Value,
    #[serde(rename = "packageContent")]
    pub package_content: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub version: String, // TODO: this will eventually be a (NuGet) semver version
}
