use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NuGetApiError {
    /// Returned when a generic http client-related error has occurred.
    // #[label("ruget::api::generic_http")]
    #[error("Request error:\n\t{0}")]
    SurfError(surf::Error, String),

    /// Source does not seem to be a valid v3 source.
    #[error("Source does not appear to be a valid NuGet API v3 source.")]
    InvalidSource(String),

    /// Returned when a URL failed to parse.
    #[error(transparent)]
    UrlParseError(#[from] surf::http::url::ParseError),

    /// The required endpoint for this call is not supported by this source.
    #[error("Endpoint not supported: {0}")]
    UnsupportedEndpoint(String),

    /// An API key is required.
    #[error("Endpoint operation requires an API key.")]
    NeedsApiKey,

    /// An API key is required.
    #[error("Unauthorized: An invalid API key was provided.")]
    BadApiKey(String),

    /// Published package was invalid.
    #[error("Invalid package.")]
    InvalidPackage,

    /// Published package already exists in source.
    #[error("Package already exists in source.")]
    PackageAlreadyExists,

    /// Package does not exist.
    // #[label("ruget::api::package_not_found")]
    // #[help("This can happen if your provided API key is invalid, or if the version you specified does not exist. Double-check both!")]
    #[error("Package does not exist.")]
    PackageNotFound,

    /// Unexpected response
    // #[label("ruget::api::unexpected_response")]
    // #[help("This is likely a bug with the NuGet API (or its documentation). Please report it.")]
    #[error("Unexpected or undocumented response: {0}")]
    BadResponse(surf::StatusCode),
}

impl Diagnostic for NuGetApiError {
    fn code(&self) -> &(dyn std::fmt::Display) {
        use NuGetApiError::*;
        match self {
            SurfError(_, _) => &"ruget::api::generic_http",
            InvalidSource(_) => &"ruget::api::invalid_source",
            UrlParseError(_) => &"ruget::api::invalid_url",
            UnsupportedEndpoint(_) => &"ruget::api::unsupported_endpoint",
            NeedsApiKey => &"ruget::api::needs_api_key",
            InvalidPackage => &"ruget::api::invalid_package",
            PackageAlreadyExists => &"ruget::api::package_exists",
            PackageNotFound => &"ruget::api::package_not_found",
            BadResponse(_) => &"ruget::api::unexpected_response",
            BadApiKey(_) => &"ruget::api::bad_api_key",
        }
    }

    fn severity(&self) -> miette::Severity {
        miette::Severity::Error
    }

    fn help(&self) -> Option<Box<dyn Iterator<Item = &str> + '_>> {
        use NuGetApiError::*;
        match self {
            SurfError(_, _) => None,
            InvalidSource(_) => Some("Are you sure this is a valid NuGet source? Example: https://api.nuget.org/v3/index.json"),
            UrlParseError(_) => Some("Check the URL syntax. URLs must include the protocol part (https://, etc)"),
            UnsupportedEndpoint(_) => Some("Only fully-compliant v3 sources are supported. See https://docs.microsoft.com/en-us/nuget/api/overview#resources-and-schema for a list of required endpoints"),
            NeedsApiKey => Some("Please supply an API key."),
            BadApiKey(_) => Some("Please make sure your API key is valid."),
            InvalidPackage => Some("Honestly, the NuGet API doesn't give us any more details besides this. :("),
            PackageAlreadyExists => None,
            PackageNotFound => Some("This can happen if your provided API key is invalid, or if the version you specified does not exist. Double-check both!"),
            BadResponse(_) => Some("This is likely a bug with the NuGet API (or its documentation). Please report it."),
        }.map(|help: &'_ str| -> Box<dyn Iterator<Item = &str>> {
            Box::new(vec![help].into_iter())
        })
    }
}
