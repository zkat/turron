use ruget_common::{
    smol::io::{Cursor, AsyncReadExt},
    surf::{self, Body, StatusCode},
};

use crate::errors::NuGetApiError;
use crate::v3::NuGetClient;

impl NuGetClient {
    pub async fn push(self, body: Body) -> Result<(), NuGetApiError> {
        use NuGetApiError::*;
        let line1 = "--X-BOUNDARY\r\n".as_bytes();
        let line2 =
            "Content-Disposition: form-data; name=\"package\";filename=\"package.nupkg\"\r\n\r\n"
                .as_bytes();
        let line3 = "\r\n--X-BOUNDARY--\r\n".as_bytes();
        let len = body
            .len()
            .map(|len| len + line1.len() + line2.len() + line3.len());
        let chain = Cursor::new(line1)
            .chain(Cursor::new(line2))
            .chain(body)
            .chain(Cursor::new(line3));
        let body = Body::from_reader(chain, len);

        let url = self
            .endpoints
            .publish
            .clone()
            .ok_or_else(|| UnsupportedEndpoint("PackagePublish/2.0.0".into()))?;
        let req = surf::put(&url)
            .header("X-NuGet-ApiKey", self.get_key()?)
            .header("Content-Type", "multipart/form-data; boundary=X-BOUNDARY")
            .body(body);

        let res = self
            .client
            .send(req)
            .await
            .map_err(|e| NuGetApiError::SurfError(e, url.into()))?;

        match res.status() {
            s if s.is_success() => Ok(()),
            StatusCode::BadRequest => Err(InvalidPackage),
            StatusCode::Conflict => Err(PackageAlreadyExists),
            StatusCode::Forbidden => Err(BadApiKey(self.get_key()?)),
            code => Err(BadResponse(code)),
        }
    }
}
