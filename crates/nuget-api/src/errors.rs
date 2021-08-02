use thisdiagnostic::{Diagnostic, Severity};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NuGetApiError {
    /// Returned when a generic http client-related error has occurred.
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

    /// Published package was invalid.
    #[error("Invalid package.")]
    InvalidPackage,

    /// Published package already exists in source.
    #[error("Package already exists in source.")]
    PackageAlreadyExists,

    /// Package does not exist.
    #[error("Package does not exist.")]
    PackageNotFound,

    /// Unexpected response
    #[error("Unexpected or undocumented response.")]
    BadResponse,
}

impl Diagnostic for NuGetApiError {
    fn label(&self) -> String {
        match self {
            NuGetApiError::SurfError(_, _) => "ruget::api::generic_http".into(),
            NuGetApiError::InvalidSource(_) => "ruget::api::invalid_source".into(),
            NuGetApiError::UrlParseError(_) => "ruget::api::invalid_url".into(),
            NuGetApiError::UnsupportedEndpoint(_) => "ruget::api::unsupported_endpoint".into(),
            NuGetApiError::NeedsApiKey => "ruget::api::needs_api_key".into(),
            NuGetApiError::InvalidPackage => "ruget::api::invalid_package".into(),
            NuGetApiError::PackageNotFound => "ruget::api::package_not_found".into(),
            NuGetApiError::BadResponse => "ruget::api::unexpected_response".into(),
            NuGetApiError::PackageAlreadyExists => "ruget::api::package_exists".into(),
        }
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn message(&self) -> String {
        self.to_string()
    }

    fn help(&self) -> Option<String> {
        match self {
            NuGetApiError::InvalidSource(_) => Some("Are you sure this is a valid NuGet source? Example: https://api.nuget.org/v3/index.json".into()),
            NuGetApiError::UrlParseError(_) => Some("Check the URL syntax. URLs must include the protocol part (https://, etc)".into()),
            NuGetApiError::UnsupportedEndpoint(_) => Some("Only fully-compliant v3 sources are supported. See https://docs.microsoft.com/en-us/nuget/api/overview#resources-and-schema for a list of required endpoints".into()),
            NuGetApiError::NeedsApiKey => Some("Please supply an API key.".into()),
            NuGetApiError::InvalidPackage => Some("Honestly, the NuGet API doesn't give us any more details besides this. :(".into()),
            NuGetApiError::PackageAlreadyExists => Some("Package already exists in source.".into()),
            NuGetApiError::PackageNotFound => Some("Package does not exist.".into()),
            NuGetApiError::BadResponse => Some("Unexpected or undocumented response.".into()),
            _ => None,
        }
    }

    fn details(&self) -> Option<&[thisdiagnostic::DiagnosticDetail]> {
        None
    }
}
