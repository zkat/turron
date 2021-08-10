use nom::branch::alt;
use nom::bytes::complete::tag_no_case as tag;
use nom::combinator::opt;
use nom::error::context;
use nom::sequence::preceded;
use nom::IResult;

use crate::error::SpecParseError;
use crate::parsers::{git, nuget, path};
use crate::PackageSpec;

/// package-spec := ( [ "nuget:" ] nuget-pkg ) | ( [ "file:" ] path ) | git-pkg
pub(crate) fn package_spec(
    input: &str,
) -> IResult<&str, PackageSpec, SpecParseError<&str>> {
    context(
        "package arg",
        alt((
            preceded(opt(tag("file:")), path::path_spec),
            git::git_spec,
            preceded(opt(tag("nuget:")), nuget::nuget_spec),
        )),
    )(input)
}
