use turron_common::surf::{self, StatusCode};

use crate::errors::NuGetApiError;
use crate::v3::NuGetClient;

impl NuGetClient {
    pub async fn unlist(
        self,
        package_id: impl AsRef<str>,
        version: impl AsRef<str>,
    ) -> Result<(), NuGetApiError> {
        use NuGetApiError::*;
        let url = self
            .endpoints
            .publish
            .clone()
            .ok_or_else(|| UnsupportedEndpoint("PackagePublish/2.0.0".into()))?;

        let req = surf::delete(url.join(package_id.as_ref())?.join(version.as_ref())?)
            .header("X-NuGet-ApiKey", self.get_key()?);

        let res = self
            .client
            .send(req)
            .await
            .map_err(|e| NuGetApiError::SurfError(e, url.into()))?;
        match res.status() {
            StatusCode::NoContent => Ok(()),
            StatusCode::NotFound => Err(PackageNotFound),
            StatusCode::Forbidden => Err(BadApiKey(self.get_key()?)),
            code => Err(BadResponse(code)),
        }
    }
}
