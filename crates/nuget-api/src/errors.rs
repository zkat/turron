use std::{cmp, io, sync::Arc};

use ruget_common::{
    miette::{self, Diagnostic, SourceOffset, SourceSpan},
    quick_xml, serde_json, surf,
    thiserror::{self, Error},
};

#[derive(Error, Debug, Diagnostic)]
pub enum NuGetApiError {
    /// Returned when a generic http client-related error has occurred.
    #[error("Request error:\n\t{0}")]
    #[diagnostic(code(ruget::api::generic_http))]
    SurfError(surf::Error, String),

    /// std::io::Error wrapper
    #[error(transparent)]
    #[diagnostic(code(ruget::api::io_error))]
    IoError(#[from] io::Error),

    /// Source does not seem to be a valid v3 source.
    #[error("Source does not appear to be a valid NuGet API v3 source: {0}")]
    #[diagnostic(
        code(ruget::api::invalid_source),
        help("Are you sure this is a valid NuGet source? Example: https://api.nuget.org/v3/index.json"),
    )]
    InvalidSource(String),

    /// Returned when a URL failed to parse.
    #[error(transparent)]
    #[diagnostic(
        code(ruget::api::invalid_url),
        help("Check the URL syntax. URLs must include the protocol part (https://, etc)")
    )]
    UrlParseError(#[from] surf::http::url::ParseError),

    /// The required endpoint for this call is not supported by this source.
    #[error("Endpoint not supported: {0}")]
    #[diagnostic(
        code(ruget::api::unsupported_endpoint),
        help("Only fully-compliant v3 sources are supported. See https://docs.microsoft.com/en-us/nuget/api/overview#resources-and-schema for a list of required endpoints")
    )]
    UnsupportedEndpoint(String),

    /// An API key is required.
    #[error("Endpoint operation requires an API key.")]
    #[diagnostic(code(ruget::api::needs_api_key), help("Please supply an API key."))]
    NeedsApiKey,

    /// An API key is required.
    #[error("Unauthorized: An invalid API key was provided.")]
    #[diagnostic(
        code(ruget::api::invalid_api_key),
        help("Please make sure your API key is valid or generate a new one.")
    )]
    BadApiKey(String),

    /// Published package was invalid.
    #[error("Invalid package.")]
    #[diagnostic(
        code(ruget::api::invalid_package),
        help("Honestly, the NuGet API doesn't give us any more details besides this. :(")
    )]
    InvalidPackage,

    /// Published package already exists in source.
    #[error("Package already exists in source.")]
    #[diagnostic(code(ruget::api::package_exists))]
    PackageAlreadyExists,

    /// Package does not exist.
    #[error("Package does not exist.")]
    #[diagnostic(
        code(ruget::api::package_not_found),
        help("This can happen if your provided API key is invalid, or if the version you specified does not exist. Double-check both!")
    )]
    PackageNotFound,

    /// The given RegistrationPage URL did not return results.
    #[error("Registration page URL is invalid.")]
    #[diagnostic(
        code(ruget::api::registration_page_not_found),
        help("Are you sure you used the right URL? This might also happen if your API key is invalid."),
    )]
    RegistrationPageNotFound,

    /// Got some bad JSON we couldn't parse.
    #[error("Received some bad JSON from the source. Unable to parse.")]
    #[diagnostic(
        code(ruget::api::bad_json),
        help("This is a bug. It might be in ruget, or it might be in the source you're using, but it's definitely a bug and should be reported.")
    )]
    BadJson {
        source: serde_json::Error,
        url: String,
        json: String,
        #[snippet(json)]
        snip: SourceSpan,
        #[highlight(snip)]
        err_loc: SourceSpan,
    },

    /// Got some bad XML we couldn't parse.
    #[error("Received some bad XML from the source. Unable to parse.")]
    #[diagnostic(
        code(ruget::api::bad_xml),
        help("This is a bug. It might be in ruget, or it might be in the source you're using, but it's definitely a bug and should be reported.")
    )]
    BadXml {
        source: quick_xml::DeError,
        url: String,
        json: Arc<String>,
    },

    /// Unexpected response
    #[error("Unexpected or undocumented response: {0}")]
    #[diagnostic(
        code(ruget::api::unexpected_response),
        help("This is likely a bug with the NuGet API (or its documentation). Please report it.")
    )]
    BadResponse(surf::StatusCode),

    /// File was not found in nupkg.
    #[error("File not found in .nupkg")]
    #[diagnostic(code(ruget::api::file_not_found))]
    FileNotFound(String, ruget_semver::Version, String),

    /// Something went wrong while reading/writing a .nupkg
    #[error(transparent)]
    #[diagnostic(code(ruget::api::zip_error))]
    ZipError(#[from] zip::result::ZipError),
}

impl NuGetApiError {
    pub fn from_json_err(err: serde_json::Error, url: String, json: String) -> Self {
        // The offset of the error itself
        let err_offset = SourceOffset::from_location(&json, err.line(), err.column());
        let len = json.len();
        Self::BadJson {
            source: err,
            url,
            json,
            snip: (
                err_offset.offset() - cmp::min(40, err_offset.offset()),
                cmp::min(40, len - err_offset.offset()),
            )
                .into(),
            err_loc: ("here", err_offset, 1.into()).into(),
        }
    }
}
