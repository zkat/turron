use turron_common::{
    miette::{self, Diagnostic},
    thiserror::{self, Error},
};
use turron_semver::{Range, Version};

#[derive(Clone, Debug, Diagnostic, Error)]
pub enum ViewError {
    #[error("Invalid utf8 text")]
    #[diagnostic(
        code(turron::view::invalid_utf8),
        help("turron only supports text files which are valid UTF-8 text.")
    )]
    InvalidUtf8(#[from] std::string::FromUtf8Error),

    #[error("Only NuGet package specifiers are acceptable. Directories and git repositories are not supported... yet ��")]
    #[diagnostic(code(turron::view::invalid_package_spec))]
    InvalidPackageSpec,

    #[error("Failed to find a version for {0} that satisfied {1}")]
    #[diagnostic(
        code(turron::view::version_not_found),
        help("Try running `turron view <id> versions`")
    )]
    VersionNotFound(String, Range),

    #[error("{0}@{1} does not have a readme")]
    #[diagnostic(code(turron::view::readme_not_found), help("turron only supports READMEs included in the package itself, which is not commonly used."))]
    ReadmeNotFound(String, Version),

    #[error("{0}@{1} does not have an icon")]
    #[diagnostic(
        code(turron::view::icon_not_found),
        help("turron only supports icons included in the package itself, not iconUrl.")
    )]
    IconNotFound(String, Version),
}
