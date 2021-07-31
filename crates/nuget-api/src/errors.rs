use thiserror::Error;

#[derive(Error, Debug)]
pub enum NuGetApiError {
    /// Returned when a generic http client-related error has occurred.
    #[error("Request error:\n\t{0}")]
    SurfError(surf::Error),

    /// Source does not seem to be a valid v3 source.
    #[error("Source does not appear to be a valid NuGet API v3 source.")]
    InvalidSource(String),

    /// Returned when a URL failed to parse.
    #[error(transparent)]
    UrlParseError(#[from] surf::http::url::ParseError),

    /// The required endpoint for this call is not supported by this source.
    #[error("Endpoint not supported: {0}")]
    UnsupportedEndpoint(String),

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
