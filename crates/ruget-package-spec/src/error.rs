use nom::error::{ContextError, ErrorKind, FromExternalError, ParseError};
use ruget_common::{
    miette::{self, Diagnostic},
    thiserror::{self, Error},
};
use ruget_semver::SemverError;
use url::ParseError as UrlParseError;

#[derive(Debug, Error)]
#[error("Error parsing package spec. {kind}")]
pub struct PackageSpecError {
    pub input: String,
    pub offset: usize,
    pub kind: SpecErrorKind,
}

impl Diagnostic for PackageSpecError {
    fn code<'a>(&'a self) -> Box<dyn std::fmt::Display + 'a> {
        self.kind.code()
    }

    fn severity(&self) -> Option<miette::Severity> {
        self.kind.severity()
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.kind.help()
    }

    fn snippets(&self) -> Option<Box<dyn Iterator<Item = miette::DiagnosticSnippet>>> {
        self.kind.snippets()
    }
}

impl PackageSpecError {
    pub fn location(&self) -> (usize, usize) {
        // Taken partially from nom.
        let prefix = &self.input.as_bytes()[..self.offset];

        // Count the number of newlines in the first `offset` bytes of input
        let line_number = bytecount::count(prefix, b'\n');

        // Find the line that includes the subslice:
        // Find the *last* newline before the substring starts
        let line_begin = prefix
            .iter()
            .rev()
            .position(|&b| b == b'\n')
            .map(|pos| self.offset - pos)
            .unwrap_or(0);

        // Find the full line after that newline
        let line = self.input[line_begin..]
            .lines()
            .next()
            .unwrap_or(&self.input[line_begin..])
            .trim_end();

        // The (1-indexed) column number is the offset of our substring into that line
        let column_number = self.input[self.offset..].as_ptr() as usize - line.as_ptr() as usize;

        (line_number, column_number)
    }
}

#[derive(Debug, Diagnostic, Error)]
pub enum SpecErrorKind {
    #[error("Found invalid characters: `{0}`")]
    #[diagnostic(code(ruget::spec::invalid_chars))]
    InvalidCharacters(String),

    #[error("Drive letters on Windows can only be alphabetical. Got `{0}`.")]
    #[diagnostic(code(ruget::spec::invalid_drive_letter))]
    InvalidDriveLetter(char),

    #[error("Invalid git host `{0}`. Only github:, gitlab:, gist:, and bitbucket: are supported in shorthands.")]
    #[diagnostic(code(ruget::spec::invalid_git_host))]
    InvalidGitHost(String),

    #[error(transparent)]
    #[diagnostic(code(ruget::spec::invalid_semver))]
    SemverParseError(SemverError),

    #[error(transparent)]
    #[diagnostic(code(ruget::spec::invalid_url))]
    UrlParseError(UrlParseError),

    #[error(transparent)]
    #[diagnostic(code(ruget::spec::invalid_git_host))]
    GitHostParseError(Box<PackageSpecError>),

    #[error("Failed to parse {0} component of semver string.")]
    #[diagnostic(code(ruget::spec::invalid_semver_component))]
    Context(&'static str),

    #[error("Incomplete input to semver parser.")]
    #[diagnostic(code(ruget::spec::incomplete_semver))]
    IncompleteInput,

    #[error("An unspecified error occurred.")]
    #[diagnostic(code(ruget::spec::other))]
    Other,
}

#[derive(Debug)]
pub(crate) struct SpecParseError<I> {
    pub(crate) input: I,
    pub(crate) context: Option<&'static str>,
    pub(crate) kind: Option<SpecErrorKind>,
}

impl<I> ParseError<I> for SpecParseError<I> {
    fn from_error_kind(input: I, _kind: nom::error::ErrorKind) -> Self {
        Self {
            input,
            context: None,
            kind: None,
        }
    }

    fn append(_input: I, _kind: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}

impl<I> ContextError<I> for SpecParseError<I> {
    fn add_context(_input: I, ctx: &'static str, mut other: Self) -> Self {
        other.context = Some(ctx);
        other
    }
}

// There's a few parsers that just... manually return SpecParseError in a
// map_res, so this absurd thing is actually needed. Curious? Just comment it
// out and look at all the red.
impl<'a> FromExternalError<&'a str, SpecParseError<&'a str>> for SpecParseError<&'a str> {
    fn from_external_error(_input: &'a str, _kind: ErrorKind, e: SpecParseError<&'a str>) -> Self {
        e
    }
}

impl<'a> FromExternalError<&'a str, SemverError> for SpecParseError<&'a str> {
    fn from_external_error(input: &'a str, _kind: ErrorKind, e: SemverError) -> Self {
        SpecParseError {
            input,
            context: None,
            kind: Some(SpecErrorKind::SemverParseError(e)),
        }
    }
}

impl<'a> FromExternalError<&'a str, UrlParseError> for SpecParseError<&'a str> {
    fn from_external_error(input: &'a str, _kind: ErrorKind, e: UrlParseError) -> Self {
        SpecParseError {
            input,
            context: None,
            kind: Some(SpecErrorKind::UrlParseError(e)),
        }
    }
}

impl<'a> FromExternalError<&'a str, PackageSpecError> for SpecParseError<&'a str> {
    fn from_external_error(input: &'a str, _kind: ErrorKind, e: PackageSpecError) -> Self {
        SpecParseError {
            input,
            context: None,
            kind: Some(SpecErrorKind::GitHostParseError(Box::new(e))),
        }
    }
}
