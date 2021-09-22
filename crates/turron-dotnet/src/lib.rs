use turron_common::{
    regex::Regex,
    smol::{self, process::Command},
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
    if output.status.success() {
        Ok(())
    } else {
        // TODO: handle bad utf8 errors
        let stdout = String::from_utf8(output.stdout).unwrap_or_else(|_| "".into());
        let regex = Regex::new(
            r"^\s*(?P<file>.*?)(\((?P<line>\d+),(?P<column>\d+)\))?\s*:\s+error\s+(?P<code>.*):\s+(?P<message>.*)$",
        ).expect("TURRON BUG: oops, bad regex?");
        let mut errors = Vec::new();
        for line in stdout.lines() {
            if let Some(captures) = regex.captures(line) {
                errors.push(MsBuildError {
                    file: captures.name("file").unwrap().as_str().trim().into(),
                    line: captures
                        .name("line")
                        .map(|x| x.as_str().parse::<usize>().unwrap()),
                    column: captures
                        .name("column")
                        .map(|x| x.as_str().parse::<usize>().unwrap()),
                    code: captures.name("code").unwrap().as_str().trim().into(),
                    message: captures.name("message").unwrap().as_str().trim().into(),
                });
            } else {
                println!("{}", line);
            }
        }
        for err in errors.clone() {
            println!("{:?}", err);
        }
        Err(DotnetError::PackFailed(errors))
    }
}
