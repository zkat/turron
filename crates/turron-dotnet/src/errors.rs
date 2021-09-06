use turron_common::{
    miette::{self, Diagnostic},
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

    #[error("Pack failed")]
    #[diagnostic(code(turron::dotnet::pack_failed))]
    PackFailed,
}
