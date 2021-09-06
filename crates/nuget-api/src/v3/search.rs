use turron_common::{
    serde::{Deserialize, Serialize},
    serde_with,
    surf::{self, StatusCode},
};

use crate::errors::NuGetApiError;
use crate::v3::NuGetClient;

impl NuGetClient {
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
            StatusCode::Ok => Ok(res
                .body_json()
                .await
                .map_err(|e| NuGetApiError::SurfError(e, url.into()))?),
            StatusCode::NotFound => Err(PackageNotFound),
            code => Err(BadResponse(code)),
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
#[serde_with::skip_serializing_none]
pub struct SearchResult {
    pub id: String,
    pub version: String,
    pub description: Option<String>,
    // TODO: there's a lot more of these fields, but they're a pain to add.
    // https://docs.microsoft.com/en-us/nuget/api/search-query-service-resource#search-result
}
