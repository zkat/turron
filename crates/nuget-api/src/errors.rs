use std::cmp;

use miette::{Diagnostic, DiagnosticSnippet, Severity, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NuGetApiError {
    /// Returned when a generic http client-related error has occurred.
    // #[label("ruget::api::generic_http")]
    #[error("Request error:\n\t{0}")]
    SurfError(surf::Error, String),

    /// Source does not seem to be a valid v3 source.
    #[error("Source does not appear to be a valid NuGet API v3 source: {0}")]
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
    #[error("Package does not exist.")]
    PackageNotFound,

    /// Got some bad JSON we couldn't parse.
    #[error("Received some bad JSON from the source. Unable to parse.")]
    BadJson {
        source: serde_json::Error,
        snippets: Vec<DiagnosticSnippet>,
    },

    /// Unexpected response
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
            BadJson { .. } => &"ruget::api::bad_json",
        }
    }

    fn severity(&self) -> Severity {
        Severity::Error
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
            BadJson { .. } => Some("This is a bug. It might be in ruget, or it might be in the source you're using, but it's definitely a bug and should be reported."),
        }.map(|help: &'_ str| -> Box<dyn Iterator<Item = &str>> {
            Box::new(vec![help].into_iter())
        })
    }

    fn snippets(&self) -> Option<&[DiagnosticSnippet]> {
        use NuGetApiError::*;
        match self {
            BadJson { snippets, .. } => Some(&snippets[..]),
            _ => None,
        }
    }
}

impl NuGetApiError {
    pub fn from_json_err(err: serde_json::Error, url: String, src: String) -> Self {
        let mut line = 0usize;
        let mut col = 0usize;
        let mut offset = 0usize;
        let len = src.len();
        for char in src.chars() {
            if char == '\n' {
                col = 0;
                line += 1;
            } else {
                col += 1;
            }
            if line + 1 == err.line() && col + 1 == err.column() {
                break;
            }
            offset += char.len_utf8();
        }
        Self::BadJson {
            source: err,
            snippets: vec![DiagnosticSnippet {
                message: None,
                source_name: url,
                source: Box::new(src),
                context: SourceSpan {
                    start: (offset - cmp::min(35, offset)).into(),
                    end: (offset + cmp::min(35, len - offset) - 1).into(),
                },
                highlights: Some(vec![(
                    "here".into(),
                    SourceSpan {
                        start: offset.into(),
                        end: offset.into(),
                    },
                )]),
            }],
        }
    }
}
