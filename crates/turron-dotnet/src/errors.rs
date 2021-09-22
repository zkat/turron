use turron_common::{
    miette::{self, Diagnostic, Report},
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

    #[error("Pack failed.\n{}", .0.iter().map(|e| format!("{:?}", Report::from(e.clone()))).collect::<Vec<_>>().join("\n"))]
    #[diagnostic(code(turron::dotnet::pack_failed))]
    PackFailed(Vec<MsBuildError>),
}

#[derive(Error, Debug, Clone)]
#[error("{message}")]
pub struct MsBuildError {
    pub file: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub code: String,
    pub message: String,
}

impl Diagnostic for MsBuildError {
    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        Some(Box::new(&self.code))
    }
}
