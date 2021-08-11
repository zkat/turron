use std::sync::Arc;

pub use ruget_common::surf::Body;
use ruget_common::{
    serde::{Deserialize, Serialize},
    serde_json,
    surf::{self, StatusCode},
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
                Ok(serde_json::from_str(&body).map_err(|e| {
                    NuGetApiError::BadJson {
                        source: e,
                        url: url.into(),
                        json: Arc::new(body),
                    }
                })?)
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
pub struct NuSpec {}
