use ruget_semver::{Version as SemVerVersion, VersionReq as SemVerVersionReq};

use nom::branch::alt;
use nom::bytes::complete::{tag_no_case as tag, take_till1};
use nom::combinator::{cut, map, map_res, opt};
use nom::error::context;
use nom::sequence::{preceded, tuple};
use nom::IResult;

use crate::error::SpecParseError;
use crate::parsers::util;
use crate::{PackageSpec, VersionSpec};

/// nuget-spec := not('@/')+ [ '@' version-req ]
pub(crate) fn nuget_spec(input: &str) -> IResult<&str, PackageSpec, SpecParseError<&str>> {
    context(
        "nuget package spec",
        map(
            tuple((
                map_res(take_till1(|x| x == '@'), util::no_url_encode),
                opt(preceded(tag("@"), cut(version_req))),
            )),
            |(name, req)| PackageSpec::NuGet {
                name: name.into(),
                requested: req,
            },
        ),
    )(input)
}

fn version_req(input: &str) -> IResult<&str, VersionSpec, SpecParseError<&str>> {
    context(
        "version requirement",
        alt((semver_version, semver_range, version_tag)),
    )(input)
}

fn semver_version(input: &str) -> IResult<&str, VersionSpec, SpecParseError<&str>> {
    let (input, version) = map_res(take_till1(|_| false), SemVerVersion::parse)(input)?;
    Ok((input, VersionSpec::Version(version)))
}

fn semver_range(input: &str) -> IResult<&str, VersionSpec, SpecParseError<&str>> {
    let (input, range) = map_res(take_till1(|_| false), SemVerVersionReq::parse)(input)?;
    Ok((input, VersionSpec::Range(range)))
}

fn version_tag(input: &str) -> IResult<&str, VersionSpec, SpecParseError<&str>> {
    context(
        "dist tag",
        map(map_res(take_till1(|_| false), util::no_url_encode), |t| {
            VersionSpec::Tag(t.into())
        }),
    )(input)
}
