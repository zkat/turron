use std::path::PathBuf;

use nom::branch::alt;
use nom::bytes::complete::tag_no_case as tag;
use nom::character::complete::{anychar, one_of};
use nom::combinator::{map, map_res, opt, recognize, rest};
use nom::error::context;
use nom::multi::{many0, many1};
use nom::sequence::{delimited, preceded, tuple};
use nom::IResult;

use crate::error::{SpecErrorKind, SpecParseError};
use crate::PackageSpec;

/// path := ( relative-dir | absolute-dir )
pub(crate) fn path_spec(input: &str) -> IResult<&str, PackageSpec, SpecParseError<&str>> {
    context(
        "path spec",
        map(alt((relative_path, absolute_path)), |p| PackageSpec::Dir {
            path: p,
        }),
    )(input)
}

/// relative-path := [ '.' ] '.' [path-sep] .*
fn relative_path(input: &str) -> IResult<&str, PathBuf, SpecParseError<&str>> {
    context(
        "relative path",
        map(
            recognize(tuple((tag("."), opt(tag(".")), many0(path_sep), rest))),
            PathBuf::from,
        ),
    )(input)
}

/// absolute-path := [ alpha ':' ] path-sep+ [ '?' path-sep+ ] .*
fn absolute_path(input: &str) -> IResult<&str, PathBuf, SpecParseError<&str>> {
    context(
        "absolute path",
        map(
            recognize(preceded(
                delimited(
                    opt(preceded(
                        map_res(anychar, |c| {
                            if c.is_alphabetic() {
                                Ok(c)
                            } else {
                                Err(SpecParseError {
                                    input,
                                    context: None,
                                    kind: Some(SpecErrorKind::InvalidDriveLetter(c)),
                                })
                            }
                        }),
                        tag(":"),
                    )),
                    many1(path_sep),
                    opt(preceded(tag("?"), many1(path_sep))),
                ),
                rest,
            )),
            PathBuf::from,
        ),
    )(input)
}

/// path-sep := ( '/' | '\' )
fn path_sep(input: &str) -> IResult<&str, char, SpecParseError<&str>> {
    one_of("/\\")(input)
}
