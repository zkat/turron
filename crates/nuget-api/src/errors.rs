use ruget_diagnostics::{Diagnostic, DiagnosticCategory, Explain};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum NuGetApiError {
    /// Returned when a generic http client-related error has occurred.
    #[label("ruget::api::generic_http")]
    #[category(Net)]
    #[error("Request error:\n\t{0}")]
    SurfError(surf::Error),

    /// Source does not seem to be a valid v3 source.
    #[category(Net)]
    #[label("ruget::api::invalid_source")]
    #[advice("Are you sure this is a valid NuGet source? Example: https://api.nuget.org/v3/index.json")]
    #[error("Source does not appear to be a valid NuGet API v3 source.")]
    InvalidSource(String),

    /// Returned when a URL failed to parse.
    #[category(Parse)]
    #[label("ruget::api::invalid_url")]
    #[advice("Check the URL syntax. URLs must include the protocol part (https://, etc)")]
    #[error(transparent)]
    UrlParseError(#[from] surf::http::url::ParseError),

    /// The required endpoint for this call is not supported by this source.
    #[category(Net)]
    #[label("ruget::api::unsupported_endpoint")]
    #[error("Endpoint not supported: {0}")]
    #[advice("Only fully-compliant v3 sources are supported. See https://docs.microsoft.com/en-us/nuget/api/overview#resources-and-schema for a list of required endpoints")]
    UnsupportedEndpoint(String),

    /// Published package was invalid.
    #[category(Misc)]
    #[label("ruget::api::invalid_package")]
    #[advice("Honestly, the NuGet API doesn't give us any more details besides this. :(")]
    #[error("Invalid package.")]
    InvalidPackage,

    /// Published package already exists in source.
    #[category(Misc)]
    #[label("ruget::api::package_exists")]
    #[error("Package already exists in source.")]
    PackageAlreadyExists,

    /// Package does not exist.
    #[category(Misc)]
    #[label("ruget::api::package_not_found")]
    #[advice("This can happen if your provided API key is invalid, or if the version you specified does not exist. Double-check both!")]
    #[error("Package does not exist.")]
    PackageNotFound,

    /// Unexpected response
    #[category(Net)]
    #[label("ruget::api::unexpected_response")]
    #[advice("This is likely a bug with the NuGet API (or its documentation). Please report it.")]
    #[error("Unexpected or undocumented response.")]
    BadResponse,
}

impl Explain for NuGetApiError {}
