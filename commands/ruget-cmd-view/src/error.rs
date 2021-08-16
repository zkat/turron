use ruget_common::{
    miette::{self, Diagnostic},
    thiserror::{self, Error},
};
use ruget_semver::{Version, VersionReq};

#[derive(Clone, Debug, Diagnostic, Error)]
pub enum ViewError {
    #[error("Invalid utf8 text")]
    #[diagnostic(
        code(ruget::view::invalid_utf8),
        help("ruget only supports text files which are valid UTF-8 text.")
    )]
    InvalidUtf8(#[from] std::string::FromUtf8Error),

    #[error("Only NuGet package specifiers are acceptable. Directories and git repositories are not supported... yet ��")]
    #[diagnostic(code(ruget::view::invalid_package_spec))]
    InvalidPackageSpec,

    #[error("Failed to find a version for {0} that satisfied {1}")]
    #[diagnostic(
        code(ruget::view::version_not_found),
        help("Try running `ruget view <id> versions`")
    )]
    VersionNotFound(String, VersionReq),

    #[error("{0}@{1} does not have a readme")]
    #[diagnostic(code(ruget::view::readme_not_found), help("ruget only supports READMEs included in the package itself, which is not commonly used."))]
    ReadmeNotFound(String, Version),

    #[error("{0}@{1} does not have an icon")]
    #[diagnostic(
        code(ruget::view::icon_not_found),
        help("ruget only supports icons included in the package itself, not iconUrl.")
    )]
    IconNotFound(String, Version),
}
