use dotnet_semver::Range;

use nom::bytes::complete::{tag_no_case as tag, take_till1};
use nom::combinator::{cut, map, map_res, opt};
use nom::error::context;
use nom::sequence::{preceded, tuple};
use nom::IResult;

use crate::error::SpecParseError;
use crate::parsers::util;
use crate::PackageSpec;

/// nuget-spec := not('@/')+ [ '@' version-req ]
pub(crate) fn nuget_spec(input: &str) -> IResult<&str, PackageSpec, SpecParseError<&str>> {
    context(
        "nuget package spec",
        map(
            tuple((
                map_res(take_till1(|x| x == '@'), util::no_url_encode),
                opt(preceded(tag("@"), cut(semver_range))),
            )),
            |(name, req)| PackageSpec::NuGet {
                name: name.into(),
                requested: req,
            },
        ),
    )(input)
}

fn semver_range(input: &str) -> IResult<&str, Range, SpecParseError<&str>> {
    let (input, range) = map_res(take_till1(|_| false), Range::parse)(input)?;
    Ok((input, range))
}
