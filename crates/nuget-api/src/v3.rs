use async_std::io::{Cursor, ReadExt};
use semver::Version;
use serde::{Deserialize, Serialize};
pub use surf::Body;
use surf::{Client, StatusCode, Url};

use crate::errors::NuGetApiError;

#[derive(Debug)]
pub struct NuGetClient {
    client: Client,
    pub key: Option<String>,
    pub endpoints: NuGetEndpoints,
}

#[derive(Debug, Serialize)]
pub struct NuGetEndpoints {
    pub package_content: Option<Url>,
    pub publish: Option<Url>,
    pub metadata: Option<Url>,
    pub search: Option<Url>,
    pub catalog: Option<Url>,
    pub signatures: Option<Url>,
    pub autocomplete: Option<Url>,
    pub symbol_publish: Option<Url>,
}

impl NuGetEndpoints {
    fn find_endpoint(resources: &[IndexResource], restype: &str) -> Option<Url> {
        resources
            .iter()
            .find(|res| res.restype == restype)
            .map(|res| res.id.clone())
    }

    fn from_resources(resources: Vec<IndexResource>) -> Self {
        NuGetEndpoints {
            package_content: Self::find_endpoint(&resources, "PackageBaseAddress/3.0.0"),
            publish: Self::find_endpoint(&resources, "PackagePublish/2.0.0"),
            metadata: Self::find_endpoint(&resources, "RegistrationsBaseUrl/3.6.0"),
            search: Self::find_endpoint(&resources, "SearchQueryService/3.5.0"),
            catalog: Self::find_endpoint(&resources, "Catalog/3.0.0"),
            signatures: Self::find_endpoint(&resources, "RepositorySignatures/5.0.0"),
            autocomplete: Self::find_endpoint(&resources, "SearchAutocompleteService/3.5.0"),
            symbol_publish: Self::find_endpoint(&resources, "SymbolPackagePublish/4.9.0"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Index {
    version: Version,
    resources: Vec<IndexResource>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IndexResource {
    #[serde(rename = "@id")]
    id: Url,
    #[serde(rename = "@type")]
    restype: String,
    comment: Option<String>,
}

impl NuGetClient {
    pub async fn from_source(source: impl AsRef<str>) -> Result<Self, NuGetApiError> {
        let client = Client::new();
        let url: Url = source.as_ref().parse().map_err(|_| NuGetApiError::InvalidSource(source.as_ref().into()))?;
        let req = surf::get(&url);
        let Index { resources, .. } = serde_json::from_slice(
            &client
                .send(req)
                .await
                .map_err(|e| NuGetApiError::SurfError(e, url.clone().into()))?
                .body_bytes()
                .await
                .map_err(|e| NuGetApiError::SurfError(e, url.clone().into()))?,
        )
        .map_err(|_| NuGetApiError::InvalidSource(source.as_ref().into()))?;
        Ok(NuGetClient {
            client,
            key: None,
            endpoints: NuGetEndpoints::from_resources(resources),
        })
    }

    pub fn get_key(&self) -> Result<String, NuGetApiError> {
        self.key.clone().ok_or(NuGetApiError::NeedsApiKey)
    }

    pub fn with_key(mut self, key: Option<impl AsRef<str>>) -> Self {
        self.key = key.map(|k| k.as_ref().into());
        self
    }

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
        let req = surf::post(&url)
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
            _ => Err(BadResponse),
        }
    }

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
            _ => Err(BadResponse),
        }
    }

    pub async fn relist(
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

        let req = surf::post(url.join(package_id.as_ref())?.join(version.as_ref())?)
            .header("X-NuGet-ApiKey", self.get_key()?);

        let res = self
            .client
            .send(req)
            .await
            .map_err(|e| NuGetApiError::SurfError(e, url.into()))?;

        match res.status() {
            StatusCode::Ok => Ok(()),
            StatusCode::NotFound => Err(PackageNotFound),
            _ => Err(BadResponse),
        }
    }

    pub async fn search(self, query: SearchQuery) -> Result<SearchResponse, NuGetApiError> {
        use NuGetApiError::*;
        let mut url = self
            .endpoints
            .search
            .ok_or_else(|| UnsupportedEndpoint("SearchQueryService/3.5.0".into()))?
            .clone();
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("semVerLevel", "2.0.0");
            if let Some(query) = query.query {
                pairs.append_pair("q", &query);
            }
            if let Some(skip) = query.skip {
                pairs.append_pair("skip", &skip.to_string());
            }
            if let Some(take) = query.take {
                pairs.append_pair("take", &take.to_string());
            }
            if let Some(prerelease) = query.prerelease {
                pairs.append_pair("prerelease", &prerelease.to_string());
            }
            if let Some(package_type) = query.package_type {
                pairs.append_pair("packageType", &package_type);
            }
        }

        let req = surf::get(&url);

        let mut res = self
            .client
            .send(req)
            .await
            .map_err(|e| NuGetApiError::SurfError(e, url.clone().into()))?;

        match res.status() {
            StatusCode::Ok => Ok(res.body_json().await.map_err(|e| NuGetApiError::SurfError(e, url.into()))?),
            StatusCode::NotFound => Err(PackageNotFound),
            _ => Err(BadResponse),
        }
    }
}

#[derive(Debug)]
pub struct SearchQuery {
    pub query: Option<String>,
    pub skip: Option<usize>,
    pub take: Option<usize>,
    pub prerelease: Option<bool>,
    pub package_type: Option<String>,
}

impl SearchQuery {
    pub fn from_query(query: impl AsRef<str>) -> Self {
        Self {
            query: Some(query.as_ref().to_string()),
            skip: None,
            take: None,
            prerelease: None,
            package_type: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    #[serde(rename = "totalHits")]
    pub total_hits: usize,
    pub data: Vec<SearchResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub version: String,
    pub description: Option<String>,
    // TODO: there's a lot more of these fields, but they're a pain to add.
    // https://docs.microsoft.com/en-us/nuget/api/search-query-service-resource#search-result
}
