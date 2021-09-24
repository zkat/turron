use turron_common::{
    miette::{self, Diagnostic, LabeledSpan, NamedSource, Severity, SourceSpan},
    thiserror::{self, Error},
};

#[derive(Error, Diagnostic, Debug)]
pub enum DotnetError {
    #[error("dotnet CLI was not found in $PATH")]
    #[diagnostic(
        code(turron::dotnet::cli_not_found),
        help("Turron requires the `dotnet` CLI to be installed and available in the user $PATH. If you haven't already, head over to https://dotnet.microsoft.com/download or use your favorite package manager to install it.")
    )]
    DotnetNotFound(#[from] which::Error),

    #[error("Failed to execute dotnet CLI.")]
    #[diagnostic(code(turron::dotnet::cli_failed))]
    DotnetFailed(#[from] std::io::Error),

    #[error("Pack failed.")]
    #[diagnostic(code(turron::dotnet::pack_failed))]
    PackFailed(#[related] Vec<MsBuildError>),
}

#[derive(Error, Debug)]
#[error("{message}")]
pub struct MsBuildError {
    pub file: NamedSource,
    pub span: SourceSpan,
    pub code: String,
    pub message: String,
    pub severity: Severity,
}

impl Diagnostic for MsBuildError {
    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        Some(Box::new(&self.code))
    }

    fn severity(&self) -> Option<miette::Severity> {
        Some(self.severity)
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        None
    }

    fn url<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        None
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&self.file)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        Some(Box::new(std::iter::once(LabeledSpan::new_with_span(
            Some("here".into()),
            self.span.clone(),
        ))))
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        None
    }
}
