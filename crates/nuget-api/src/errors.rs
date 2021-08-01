use thisdiagnostic::{Diagnostic, DiagnosticMetadata, GetMetadata};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum NuGetApiError {
    /// Returned when a generic http client-related error has occurred.
    #[label("ruget::api::generic_http")]
    #[error("Request error:\n\t{0}")]
    SurfError(surf::Error, String),

    /// Source does not seem to be a valid v3 source.
    #[label("ruget::api::invalid_source")]
    #[help(
        "Are you sure this is a valid NuGet source? Example: https://api.nuget.org/v3/index.json"
    )]
    #[error("Source does not appear to be a valid NuGet API v3 source.")]
    InvalidSource(String),

    /// Returned when a URL failed to parse.
    #[label("ruget::api::invalid_url")]
    #[help("Check the URL syntax. URLs must include the protocol part (https://, etc)")]
    #[error(transparent)]
    UrlParseError(#[from] surf::http::url::ParseError),

    /// The required endpoint for this call is not supported by this source.
    #[label("ruget::api::unsupported_endpoint")]
    #[error("Endpoint not supported: {0}")]
    #[help("Only fully-compliant v3 sources are supported. See https://docs.microsoft.com/en-us/nuget/api/overview#resources-and-schema for a list of required endpoints")]
    UnsupportedEndpoint(String),

    /// An API key is required.
    #[label("ruget::api::needs_api_key")]
    #[help("Please supply an API key.")]
    #[error("Endpoint operation requires an API key.")]
    NeedsApiKey,

    /// Published package was invalid.
    #[label("ruget::api::invalid_package")]
    #[help("Honestly, the NuGet API doesn't give us any more details besides this. :(")]
    #[error("Invalid package.")]
    InvalidPackage,

    /// Published package already exists in source.
    #[label("ruget::api::package_exists")]
    #[error("Package already exists in source.")]
    PackageAlreadyExists,

    /// Package does not exist.
    #[label("ruget::api::package_not_found")]
    #[help("This can happen if your provided API key is invalid, or if the version you specified does not exist. Double-check both!")]
    #[error("Package does not exist.")]
    PackageNotFound,

    /// Unexpected response
    #[label("ruget::api::unexpected_response")]
    #[help("This is likely a bug with the NuGet API (or its documentation). Please report it.")]
    #[error("Unexpected or undocumented response.")]
    BadResponse,
}

impl GetMetadata for NuGetApiError {
    fn meta(&self) -> Option<DiagnosticMetadata> {
        use NuGetApiError::*;
        match self {
            SurfError(_, url) => Some(DiagnosticMetadata::Net { url: url.clone() }),
            InvalidSource(url) => Some(DiagnosticMetadata::Net { url: url.clone() }),
            UnsupportedEndpoint(url) => Some(DiagnosticMetadata::Net { url: url.clone() }),
            _ => None,
        }
    }
}
