use turron_common::{
    miette::{NamedSource, Severity, SourceOffset},
    regex::Regex,
    smol::{self, fs, process::Command},
    tracing,
};

pub use errors::{DotnetError, MsBuildError};

mod errors;

pub async fn pack() -> Result<(), DotnetError> {
    let cli_path = smol::unblock(|| which::which("dotnet")).await?;
    let output = Command::new(cli_path)
        .arg("pack")
        .arg("--nologo")
        .output()
        .await?;
    // TODO: handle bad utf8 errors
    let stdout = String::from_utf8(output.stdout).unwrap_or_else(|_| "".into());
    let regex = Regex::new(
            r"^\s*(?P<file>.*?)(\((?P<line>\d+),(?P<column>\d+)\))?\s*:\s+(?P<severity>.*?)\s+(?P<code>.*):\s+(?P<message>.*)$",
        ).expect("TURRON BUG: oops, bad regex?");
    let mut errors = Vec::new();

    for line in stdout.lines() {
        if let Some(captures) = regex.captures(line) {
            let filename: String = captures.name("file").unwrap().as_str().trim().into();
            let contents = fs::read_to_string(&filename).await?;
            let line = captures
                .name("line")
                .map(|x| x.as_str().parse::<usize>().unwrap())
                .unwrap_or(0);
            let column = captures
                .name("column")
                .map(|x| x.as_str().parse::<usize>().unwrap())
                .unwrap_or(0);
            let err_offset = SourceOffset::from_location(&contents, line, column);
            errors.push(MsBuildError {
                file: NamedSource::new(filename, contents),
                span: (err_offset, 0.into()).into(),
                code: captures.name("code").unwrap().as_str().trim().into(),
                message: captures.name("message").unwrap().as_str().trim().into(),
                severity: match captures.name("severity").unwrap().as_str().trim() {
                    "warning" => Severity::Warning,
                    "info" => Severity::Advice,
                    _ => Severity::Error,
                },
            });
        } else {
            tracing::info!("{}", line);
        }
    }
    if output.status.success() {
        Ok(())
    } else {
        Err(DotnetError::PackFailed(errors))
    }
}
