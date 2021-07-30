use semver::Version;
use serde::{Deserialize, Serialize};
pub use surf::Body;
use surf::{Client, StatusCode, Url};

use crate::errors::NuGetApiError;

#[derive(Debug)]
pub struct NuGetClient {
    client: Client,
    key: Option<String>,
    endpoints: NuGetEndpoints,
}

#[derive(Debug)]
pub struct NuGetEndpoints {
    package_content: Option<Url>,
    publish: Option<Url>,
    metadata: Option<Url>,
    search: Option<Url>,
    catalog: Option<Url>,
    signatures: Option<Url>,
    autocomplete: Option<Url>,
    symbol_publish: Option<Url>,
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
            metadata: Self::find_endpoint(&resources, "RegistrationBaseUrl/3.6.0"),
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
        let req = surf::get(source.as_ref());
        let Index { resources, .. } = client
            .send(req)
            .await
            .map_err(NuGetApiError::SurfError)?
            .body_json()
            .await
            .map_err(NuGetApiError::SurfError)?;
        Ok(NuGetClient {
            client,
            key: None,
            endpoints: NuGetEndpoints::from_resources(resources),
        })
    }

    pub fn with_key(mut self, key: impl AsRef<str>) -> Self {
        self.key = Some(key.as_ref().to_string());
        self
    }

    pub async fn push(self, body: Body) -> Result<(), NuGetApiError> {
        // TODO: **THIS IS BROKEN**. Implementing this correctly is blocked by:
        // https://github.com/http-rs/surf/issues/75
        use NuGetApiError::*;
        let req = surf::post(
            &self
                .endpoints
                .publish
                .ok_or_else(|| UnsupportedEndpoint("PackagePublish/2.0.0".into()))?,
        )
        .header("X-NuGet-ApiKey", &self.key.expect("API Key is required."))
        .header("Content-Type", "multipart/form-data")
        .body(body);
        let res = self
            .client
            .send(req)
            .await
            .map_err(NuGetApiError::SurfError)?;
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
        let req = surf::delete(
            &self
                .endpoints
                .publish
                .ok_or_else(|| UnsupportedEndpoint("PackagePublish/2.0.0".into()))?
                .join(package_id.as_ref())?
                .join(version.as_ref())?,
        )
        .header("X-NuGet-ApiKey", &self.key.expect("API Key is required."));
        let res = self
            .client
            .send(req)
            .await
            .map_err(NuGetApiError::SurfError)?;
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
        let req = surf::post(
            &self
                .endpoints
                .publish
                .ok_or_else(|| UnsupportedEndpoint("PackagePublish/2.0.0".into()))?
                .join(package_id.as_ref())?
                .join(version.as_ref())?,
        )
        .header("X-NuGet-ApiKey", &self.key.expect("API Key is required."));
        let res = self
            .client
            .send(req)
            .await
            .map_err(NuGetApiError::SurfError)?;
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
            .publish
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
        let req = surf::get(url);
        let mut res = self
            .client
            .send(req)
            .await
            .map_err(NuGetApiError::SurfError)?;
        match res.status() {
            StatusCode::Ok => Ok(res.body_json().await.map_err(NuGetApiError::SurfError)?),
            StatusCode::NotFound => Err(PackageNotFound),
            _ => Err(BadResponse),
        }
    }
}

#[derive(Debug)]
pub struct SearchQuery {
    query: Option<String>,
    skip: Option<usize>,
    take: Option<usize>,
    prerelease: Option<bool>,
    package_type: Option<String>,
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
    total_hits: usize,
    data: Vec<SearchResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    id: String,
    version: Version,
    description: Option<String>,
    // TODO: there's a lot more of these fields, but they're a pain to add.
    // https://docs.microsoft.com/en-us/nuget/api/search-query-service-resource#search-result
}
