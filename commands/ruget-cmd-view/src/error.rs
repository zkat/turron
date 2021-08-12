use ruget_common::{
    miette::Diagnostic,
    thiserror::{self, Error},
};
use ruget_semver::{Version, VersionReq};

#[derive(Clone, Debug, Error)]
pub enum ViewError {
    #[error("Invalid utf8 text")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
    #[error("Only NuGet package specifiers are acceptable. Directories and git repositories are not supported... yet ��")]
    InvalidPackageSpec,
    #[error("Failed to find a version for {0} that satisfied {1}")]
    VersionNotFound(String, VersionReq),
    #[error("{0}@{1} does not have a readme")]
    ReadmeNotFound(String, Version),
    #[error("{0}@{1} does not have an icon")]
    IconNotFound(String, Version),
}

impl Diagnostic for ViewError {
    fn code(&self) -> Box<dyn std::fmt::Display> {
        Box::new(match self {
            ViewError::VersionNotFound(_, _) => &"ruget::view::version_not_found",
            ViewError::InvalidPackageSpec => &"ruget::view::invalid_package_spec",
            ViewError::ReadmeNotFound(_, _) => &"ruget::view::readme_not_found",
            ViewError::IconNotFound(_, _) => &"ruget::view::icon_not_found",
            ViewError::InvalidUtf8(_) => &"ruget::view::invalid_utf8",
        })
    }

    fn help(&self) -> Option<Box<dyn std::fmt::Display>> {
        match self {
            // TODO: I guess this is good motivation to change miette...
            // ViewError::VersionNotFound(id, _) => Some(&format!("Try running `ruget view {} versions`", id))
            ViewError::InvalidPackageSpec => None,
            ViewError::VersionNotFound(_, _) => Some(&"Try running `ruget view <id> versions`"),
            ViewError::ReadmeNotFound(_, _) => Some(&"ruget only supports READMEs included in the package itself, which is not commonly used."),
            ViewError::IconNotFound(_, _) => Some(&"ruget only supports icons included in the package itself, not iconUrl."),
            ViewError::InvalidUtf8(_) => Some(&"ruget only supports text files which are valid UTF-8 text.")
        }
        .map(|s| -> Box<dyn std::fmt::Display> { Box::new(*s) })
    }
}
