use std::cmp::{Ord, Ordering, PartialOrd};
use std::fmt;

use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::space0;
use nom::combinator::{all_consuming, cut, map, map_opt, opt};
use nom::error::context;
use nom::multi::separated_list1;
use nom::sequence::tuple;
use nom::{Err, IResult};
use serde::de::{self, Deserialize, Deserializer, Visitor};
use serde::ser::{Serialize, Serializer};

use crate::{extras, number, SemverError, SemverErrorKind, SemverParseError, Version};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct ComparatorSet {
    floating: bool,
    upper: Bound,
    lower: Bound,
}

impl ComparatorSet {
    fn new(lower: Bound, upper: Bound, floating: bool) -> Option<Self> {
        use Bound::*;
        use Predicate::*;

        match (lower, upper) {
            (Lower(Excluding(v1)), Upper(Including(v2)))
            | (Lower(Including(v1)), Upper(Excluding(v2)))
                if v1 == v2 =>
            {
                None
            }
            (Lower(Including(v1)), Upper(Including(v2))) if v1 == v2 => Some(Self {
                floating,
                lower: Lower(Including(v1)),
                upper: Upper(Including(v2)),
            }),
            (lower, upper) if lower <= upper => Some(Self {
                floating,
                lower,
                upper,
            }),
            _ => None,
        }
    }

    fn has_pre(&self) -> bool {
        use Bound::*;
        use Predicate::*;

        let lower_bound = match &self.lower {
            Lower(Including(lower)) => !lower.pre_release.is_empty(),
            Lower(Excluding(lower)) => !lower.pre_release.is_empty(),
            Lower(Unbounded) => false,
            _ => unreachable!(
                "There should not have been an upper bound: {:#?}",
                self.lower
            ),
        };

        let upper_bound = match &self.upper {
            Upper(Including(upper)) => !upper.pre_release.is_empty(),
            Upper(Excluding(upper)) => !upper.pre_release.is_empty(),
            Upper(Unbounded) => false,
            _ => unreachable!(
                "There should not have been a lower bound: {:#?}",
                self.lower
            ),
        };

        lower_bound || upper_bound
    }

    fn satisfies(&self, version: &Version) -> bool {
        use Bound::*;
        use Predicate::*;

        let lower_bound = match &self.lower {
            Lower(Including(lower)) => lower <= version,
            Lower(Excluding(lower)) => lower < version,
            Lower(Unbounded) => true,
            _ => unreachable!(
                "There should not have been an upper bound: {:#?}",
                self.lower
            ),
        };

        let upper_bound = match &self.upper {
            Upper(Including(upper)) => version <= upper,
            Upper(Excluding(upper)) => version < upper,
            Upper(Unbounded) => true,
            _ => unreachable!(
                "There should not have been an lower bound: {:#?}",
                self.lower
            ),
        };

        lower_bound && upper_bound
    }

    fn allows_all(&self, other: &ComparatorSet) -> bool {
        self.lower <= other.lower && other.upper <= self.upper
    }

    fn allows_any(&self, other: &ComparatorSet) -> bool {
        if other.upper < self.lower {
            return false;
        }

        if self.upper < other.lower {
            return false;
        }

        true
    }

    fn intersect(&self, other: &Self) -> Option<Self> {
        let lower = std::cmp::max(&self.lower, &other.lower);
        let upper = std::cmp::min(&self.upper, &other.upper);

        ComparatorSet::new(
            lower.clone(),
            upper.clone(),
            self.floating || other.floating,
        )
    }

    fn difference(&self, other: &Self) -> Option<Vec<Self>> {
        use Bound::*;
        let floating = self.floating || other.floating;

        if let Some(overlap) = self.intersect(other) {
            if &overlap == self {
                return None;
            }

            if self.lower < overlap.lower && overlap.upper < self.upper {
                return Some(vec![
                    ComparatorSet::new(
                        self.lower.clone(),
                        Upper(overlap.lower.predicate().flip()),
                        floating,
                    )
                    .unwrap(),
                    ComparatorSet::new(
                        Lower(overlap.upper.predicate().flip()),
                        self.upper.clone(),
                        floating,
                    )
                    .unwrap(),
                ]);
            }

            if self.lower < overlap.lower {
                return ComparatorSet::new(
                    self.lower.clone(),
                    Upper(overlap.lower.predicate().flip()),
                    floating,
                )
                .map(|f| vec![f]);
            }

            ComparatorSet::new(
                Lower(overlap.upper.predicate().flip()),
                self.upper.clone(),
                floating,
            )
            .map(|f| vec![f])
        } else {
            Some(vec![self.clone()])
        }
    }
}

impl fmt::Display for ComparatorSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Bound::*;
        use Predicate::*;
        match (&self.lower, &self.upper) {
            (Lower(Unbounded), Upper(Unbounded)) => write!(f, "*"),
            (Lower(Unbounded), Upper(Including(v))) => write!(f, "(,{}]", v),
            (Lower(Unbounded), Upper(Excluding(v))) => write!(f, "(,{})", v),
            (Lower(Including(v)), Upper(Unbounded)) => write!(f, "[{},)", v),
            (Lower(Excluding(v)), Upper(Unbounded)) => write!(f, "({},)", v),
            (Lower(Including(v)), Upper(Including(v2))) if v == v2 => write!(f, "[{}]", v),
            (Lower(Including(v)), Upper(Including(v2))) => write!(f, "[{},{}]", v, v2),
            (Lower(Including(v)), Upper(Excluding(v2))) => write!(f, "[{},{})", v, v2),
            (Lower(Excluding(v)), Upper(Including(v2))) => write!(f, "({},{}]", v, v2),
            (Lower(Excluding(v)), Upper(Excluding(v2))) => write!(f, "({},{})", v, v2),
            _ => unreachable!("does not make sense"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Predicate {
    Excluding(Version), // ( and )
    Including(Version), // [ and ]
    Unbounded,          // *
}

impl Predicate {
    fn flip(&self) -> Self {
        use Predicate::*;
        match self {
            Excluding(v) => Including(v.clone()),
            Including(v) => Excluding(v.clone()),
            Unbounded => Unbounded,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum Bound {
    Lower(Predicate),
    Upper(Predicate),
}

impl Bound {
    fn upper() -> Self {
        Bound::Upper(Predicate::Unbounded)
    }

    fn lower() -> Self {
        Bound::Lower(Predicate::Unbounded)
    }

    fn predicate(&self) -> Predicate {
        use Bound::*;

        match self {
            Lower(p) => p.clone(),
            Upper(p) => p.clone(),
        }
    }
}

impl Ord for Bound {
    fn cmp(&self, other: &Self) -> Ordering {
        use Bound::*;
        use Predicate::*;

        match (self, other) {
            (Lower(Unbounded), Lower(Unbounded)) | (Upper(Unbounded), Upper(Unbounded)) => {
                Ordering::Equal
            }
            (Upper(Unbounded), _) | (_, Lower(Unbounded)) => Ordering::Greater,
            (Lower(Unbounded), _) | (_, Upper(Unbounded)) => Ordering::Less,

            (Upper(Including(v1)), Upper(Including(v2)))
            | (Upper(Including(v1)), Lower(Including(v2)))
            | (Upper(Excluding(v1)), Upper(Excluding(v2)))
            | (Upper(Excluding(v1)), Lower(Excluding(v2)))
            | (Lower(Including(v1)), Upper(Including(v2)))
            | (Lower(Including(v1)), Lower(Including(v2)))
            | (Lower(Excluding(v1)), Lower(Excluding(v2))) => v1.cmp(v2),

            (Lower(Excluding(v1)), Upper(Excluding(v2)))
            | (Lower(Including(v1)), Upper(Excluding(v2))) => {
                if v2 <= v1 {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
            (Upper(Including(v1)), Upper(Excluding(v2)))
            | (Upper(Including(v1)), Lower(Excluding(v2)))
            | (Lower(Excluding(v1)), Upper(Including(v2))) => {
                if v2 < v1 {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
            (Lower(Excluding(v1)), Lower(Including(v2))) => {
                if v1 < v2 {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
            (Lower(Including(v1)), Lower(Excluding(v2)))
            | (Upper(Excluding(v1)), Lower(Including(v2)))
            | (Upper(Excluding(v1)), Upper(Including(v2))) => {
                if v1 <= v2 {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
        }
    }
}

impl PartialOrd for Bound {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Range {
    comparators: Vec<ComparatorSet>,
}

impl Range {
    pub fn parse<S: AsRef<str>>(input: S) -> Result<Self, SemverError> {
        let input = input.as_ref();

        match all_consuming(range)(input) {
            Ok((_, predicates)) => Ok(Range {
                comparators: predicates,
            }),
            Err(err) => Err(match err {
                Err::Error(e) | Err::Failure(e) => SemverError {
                    input: input.into(),
                    offset: e.input.as_ptr() as usize - input.as_ptr() as usize,
                    kind: if let Some(kind) = e.kind {
                        kind
                    } else if let Some(ctx) = e.context {
                        SemverErrorKind::Context(ctx)
                    } else {
                        SemverErrorKind::Other
                    },
                },
                Err::Incomplete(_) => SemverError {
                    input: input.into(),
                    offset: input.len() - 1,
                    kind: SemverErrorKind::IncompleteInput,
                },
            }),
        }
    }

    pub fn any() -> Self {
        Self {
            comparators: vec![ComparatorSet::new(Bound::lower(), Bound::upper(), false).unwrap()],
        }
    }

    pub fn any_floating() -> Self {
        Self {
            comparators: vec![ComparatorSet::new(Bound::lower(), Bound::upper(), true).unwrap()],
        }
    }

    pub fn is_floating(&self) -> bool {
        self.comparators.iter().any(|comp| comp.floating)
    }

    pub fn has_pre_release(&self) -> bool {
        self.comparators.iter().any(|pred| pred.has_pre())
    }

    pub fn satisfies(&self, version: &Version) -> bool {
        for range in &self.comparators {
            if range.satisfies(version) {
                return true;
            }
        }

        false
    }

    pub fn allows_all(&self, other: &Range) -> bool {
        for this in &self.comparators {
            for that in &other.comparators {
                if this.allows_all(that) {
                    return true;
                }
            }
        }

        false
    }

    pub fn allows_any(&self, other: &Range) -> bool {
        for this in &self.comparators {
            for that in &other.comparators {
                if this.allows_any(that) {
                    return true;
                }
            }
        }

        false
    }

    pub fn intersect(&self, other: &Self) -> Option<Self> {
        let mut predicates = Vec::new();

        for lefty in &self.comparators {
            for righty in &other.comparators {
                if let Some(range) = lefty.intersect(righty) {
                    predicates.push(range)
                }
            }
        }

        if predicates.is_empty() {
            None
        } else {
            Some(Self {
                comparators: predicates,
            })
        }
    }

    pub fn difference(&self, other: &Self) -> Option<Self> {
        let mut predicates = Vec::new();

        for lefty in &self.comparators {
            for righty in &other.comparators {
                if let Some(mut range) = lefty.difference(righty) {
                    predicates.append(&mut range)
                }
            }
        }

        if predicates.is_empty() {
            None
        } else {
            Some(Self {
                comparators: predicates,
            })
        }
    }
}

impl std::str::FromStr for Range {
    type Err = SemverError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Range::parse(s)
    }
}

impl Serialize for Range {
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize VersionReq as a string.
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Range {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VersionReqVisitor;

        /// Deserialize `VersionReq` from a string.
        impl<'de> Visitor<'de> for VersionReqVisitor {
            type Value = Range;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a SemVer version requirement as a string")
            }

            fn visit_str<E>(self, v: &str) -> ::std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Range::parse(v).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_str(VersionReqVisitor)
    }
}

fn range(input: &str) -> IResult<&str, Vec<ComparatorSet>, SemverParseError<&str>> {
    context(
        "range",
        separated_list1(tuple((space0, tag("||"), space0)), comparators),
    )(input)
}

fn comparators(input: &str) -> IResult<&str, ComparatorSet, SemverParseError<&str>> {
    alt((
        // [1.2.3, 3.2.1) || [1.*,3.1]
        brackets_range,
        // 1.0 || 1.* || 1 || *
        plain_version_range,
    ))(input)
}

fn plain_version_range(input: &str) -> IResult<&str, ComparatorSet, SemverParseError<&str>> {
    context(
        "base version range",
        map_opt(plain_version, |(floating, version)| {
            ComparatorSet::new(
                if is_empty(&version) {
                    Bound::lower()
                } else {
                    Bound::Lower(Predicate::Including(version.clone()))
                },
                match version {
                    v if is_empty(&v) => Bound::upper(),
                    Version {
                        major: 0,
                        minor: 0,
                        patch: 0,
                        revision,
                        ..
                    } => Bound::Upper(Predicate::Excluding(Version {
                        major: 0,
                        minor: 0,
                        patch: 0,
                        revision: revision + 1,
                        build: Vec::new(),
                        pre_release: Vec::new(),
                    })),
                    Version {
                        major: 0,
                        minor: 0,
                        patch,
                        ..
                    } => Bound::Upper(Predicate::Excluding(Version {
                        major: 0,
                        minor: 0,
                        patch: patch + 1,
                        revision: 0,
                        build: Vec::new(),
                        pre_release: Vec::new(),
                    })),
                    Version {
                        major: 0, minor, ..
                    } => Bound::Upper(Predicate::Excluding(Version {
                        major: 0,
                        minor: minor + 1,
                        patch: 0,
                        revision: 0,
                        build: Vec::new(),
                        pre_release: Vec::new(),
                    })),
                    Version { major, .. } if floating => {
                        // N.*
                        Bound::Upper(Predicate::Excluding(Version {
                            major: major + 1,
                            minor: 0,
                            patch: 0,
                            revision: 0,
                            build: Vec::new(),
                            pre_release: Vec::new(),
                        }))
                    }
                    _ => Bound::upper(),
                },
                floating,
            )
        }),
    )(input)
}

fn plain_version(input: &str) -> IResult<&str, (bool, Version), SemverParseError<&str>> {
    let (input, major) = num_or_star(input)?;

    let (input, minor) = if major.is_some() {
        // Major was a number.
        opt(dotversion)(input)?
    } else {
        // Major was *.
        let (input, extras) = opt(extras)(input)?;
        return Ok((
            input,
            (
                true,
                Version {
                    major: 0,
                    minor: 0,
                    patch: 0,
                    revision: 0,
                    pre_release: extras.map(|(pre, _)| pre).unwrap_or_else(Vec::new),
                    build: Vec::new(),
                },
            ),
        ));
    };

    let (input, patch) = if minor.flatten().is_some() {
        // Minor is a _number_, specifically.
        opt(dotversion)(input)?
    } else {
        // Minor is *.
        let (input, extras) = opt(extras)(input)?;
        return Ok((
            input,
            (
                minor.is_some(),
                Version {
                    major: major.unwrap(),
                    minor: minor.flatten().unwrap_or(0),
                    patch: 0,
                    revision: 0,
                    pre_release: extras.map(|(pre, _)| pre).unwrap_or_else(Vec::new),
                    build: Vec::new(),
                },
            ),
        ));
    };

    let (input, revision) = if patch.flatten().is_some() {
        opt(dotversion)(input)?
    } else {
        let (input, extras) = opt(extras)(input)?;
        return Ok((
            input,
            (
                patch.is_some(),
                Version {
                    major: major.unwrap(),
                    minor: minor.flatten().unwrap_or(0),
                    patch: 0,
                    revision: 0,
                    pre_release: extras.map(|(pre, _)| pre).unwrap_or_else(Vec::new),
                    build: Vec::new(),
                },
            ),
        ));
    };

    let (input, extras) = opt(extras)(input)?;
    let (pre_release, build) = extras.unwrap_or_else(|| (Vec::new(), Vec::new()));
    Ok((
        input,
        (
            revision.is_some(),
            Version {
                major: major.unwrap_or(0),
                minor: minor.flatten().unwrap_or(0),
                patch: patch.flatten().unwrap_or(0),
                revision: revision.flatten().unwrap_or(0),
                build,
                pre_release,
            },
        ),
    ))
}

fn dotversion(input: &str) -> IResult<&str, Option<u64>, SemverParseError<&str>> {
    let (input, _) = tag(".")(input)?;
    num_or_star(input)
}

fn num_or_star(input: &str) -> IResult<&str, Option<u64>, SemverParseError<&str>> {
    context(
        "Version number or asterisk",
        alt((map(number, Some), map(tag("*"), |_| None))),
    )(input)
}

fn is_empty(version: &Version) -> bool {
    version
        == &Version {
            major: 0,
            minor: 0,
            patch: 0,
            revision: 0,
            build: Vec::new(),
            pre_release: Vec::new(),
        }
}

fn brackets_range(input: &str) -> IResult<&str, ComparatorSet, SemverParseError<&str>> {
    let mut floating = false;
    let (input, open) = open_brace(input)?;
    let (input, _) = space0(input)?;
    let (input, comma) = opt(tag(","))(input)?;
    let (input, (is_float, version1)) = cut(plain_version)(input)?;
    floating = floating || is_float;
    if comma.is_some() {
        let (input, _) = space0(input)?;
        let (input, _) = close_brace(input)?;
        return Ok((
            input,
            ComparatorSet::new(
                Bound::lower(),
                if floating && is_empty(&version1) {
                    Bound::upper()
                } else {
                    Bound::Upper(match open {
                        "(" => Predicate::Excluding(version1),
                        "[" => Predicate::Including(version1),
                        _ => unreachable!(),
                    })
                },
                floating,
            )
            .unwrap(),
        ));
    }
    let (input, _) = space0(input)?;
    let (input, comma) = opt(tag(","))(input)?;
    if comma.is_none() {
        let (input, _) = space0(input)?;
        let (input, _) = close_brace(input)?;
        return Ok((
            input,
            ComparatorSet::new(
                if floating && is_empty(&version1) {
                    Bound::lower()
                } else {
                    Bound::Lower(match open {
                        "(" => Predicate::Excluding(version1),
                        "[" => Predicate::Including(version1),
                        _ => unreachable!(),
                    })
                },
                Bound::upper(),
                floating,
            )
            .unwrap(),
        ));
    }

    let (input, _) = space0(input)?;
    let (input, version2) = opt(plain_version)(input)?;
    let (input, close) = close_brace(input)?;

    if let Some((is_float, version2)) = version2 {
        let v1float = floating;
        floating = floating || is_float;
        let lower = if v1float && is_empty(&version1) {
            Bound::lower()
        } else {
            Bound::Lower(match open {
                "(" => Predicate::Excluding(version1),
                "[" => Predicate::Including(version1),
                _ => unreachable!(),
            })
        };
        let upper = if is_float && is_empty(&version2) {
            Bound::upper()
        } else {
            Bound::Upper(match close {
                ")" => Predicate::Excluding(version2),
                "]" => Predicate::Including(version2),
                _ => unreachable!(),
            })
        };
        Ok((input, ComparatorSet::new(lower, upper, floating).unwrap()))
    } else {
        let lower = if floating && is_empty(&version1) {
            Bound::lower()
        } else {
            Bound::Lower(match open {
                "(" => Predicate::Excluding(version1),
                "[" => Predicate::Including(version1),
                _ => unreachable!(),
            })
        };
        let upper = Bound::upper();
        Ok((input, ComparatorSet::new(lower, upper, floating).unwrap()))
    }
}

fn open_brace(input: &str) -> IResult<&str, &str, SemverParseError<&str>> {
    context("opening bracket", alt((tag("["), tag("("))))(input)
}

fn close_brace(input: &str) -> IResult<&str, &str, SemverParseError<&str>> {
    context("closing bracket", alt((tag("]"), tag(")"))))(input)
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, range) in self.comparators.iter().enumerate() {
            if i > 0 {
                write!(f, "||")?;
            }
            write!(f, "{}", range)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn plain_version_range() -> Result<(), SemverError> {
        let range: Range = "*".parse()?;
        assert_eq!(range.comparators.len(), 1);
        assert_eq!(range.comparators[0].to_string(), "*".to_string());

        Ok(())
    }

    #[test]
    fn brackets_range() -> Result<(), SemverError> {
        let range: Range = "[1.2.3, 3.2.1)".parse()?;
        assert_eq!(range.comparators.len(), 1);
        assert_eq!(
            range.comparators[0].to_string(),
            "[1.2.3,3.2.1)".to_string()
        );

        let range: Range = "[1,2.1]".parse()?;
        assert!(!range.is_floating());
        assert_eq!(range.comparators.len(), 1);
        assert_eq!(
            range.comparators[0].to_string(),
            "[1.0.0,2.1.0]".to_string()
        );

        let range: Range = "[1.*,2.1]".parse()?;
        assert!(range.is_floating());
        assert_eq!(range.comparators.len(), 1);
        assert_eq!(
            range.comparators[0].to_string(),
            "[1.0.0,2.1.0]".to_string()
        );

        let range: Range = "[1,2.1.*]".parse()?;
        assert!(range.is_floating());
        assert_eq!(range.comparators.len(), 1);
        assert_eq!(
            range.comparators[0].to_string(),
            "[1.0.0,2.1.0]".to_string()
        );

        let range: Range = "[*]".parse()?;
        assert!(range.is_floating());
        assert_eq!(range.comparators.len(), 1);
        assert_eq!(range.comparators[0].to_string(), "*".to_string());

        let range: Range = "[*,]".parse()?;
        assert!(range.is_floating());
        assert_eq!(range.comparators.len(), 1);
        assert_eq!(range.comparators[0].to_string(), "*".to_string());

        let range: Range = "[,*)".parse()?;
        assert!(range.is_floating());
        assert_eq!(range.comparators.len(), 1);
        assert_eq!(range.comparators[0].to_string(), "*".to_string());

        Ok(())
    }

    #[test]
    fn pre_release_casing() -> Result<(), SemverError> {
        let version: Version = "1.2.3-alpha".parse()?;
        let range: Range = "1.2.3-ALPHA".parse()?;
        assert!(range.satisfies(&version));
        Ok(())
    }
}

/*
macro_rules! create_tests_for {
    ($func:ident $($name:ident => $version_range:expr , { $x:ident => $allows:expr, $y:ident => $denies:expr$(,)? }),+ ,$(,)?) => {

        #[cfg(test)]
        mod $func {
        use super::*;

            $(
                #[test]
                fn $name() {
                    let version_range = Range::parse($version_range).unwrap();

                    let allows: Vec<Range> = $allows.iter().map(|v| Range::parse(v).unwrap()).collect();
                    for version in &allows {
                        assert!(version_range.$func(version), "should have allowed: {}", version);
                    }

                    let ranges: Vec<Range> = $denies.iter().map(|v| Range::parse(v).unwrap()).collect();
                    for version in &ranges {
                        assert!(!version_range.$func(version), "should have denied: {}", version);
                    }
                }
            )+
        }
    }
}

create_tests_for! {
    // The function we are testing:
    allows_all

    greater_than_eq_123   => ">=1.2.3", {
        allows => [">=2.0.0", ">2", "=2.0.0", "0.1 || 1.4", "=1.2.3", "2 - 7", ">2.0.0"],
        denies => ["1.0.0", "<1.2", ">=1.2.2", "1 - 3", "0.1 || <1.2.0", ">1.0.0"],
    },

    greater_than_123      => ">1.2.3", {
        allows => [">=2.0.0", ">2", "2.0.0", "0.1 || 1.4", ">2.0.0"],
        denies => ["1.0.0", "<1.2", ">=1.2.3", "1 - 3", "0.1 || <1.2.0", "<=3"],
    },

    eq_123  => "=1.2.3", {
        allows => ["=1.2.3"],
        denies => ["=1.0.0", "<1.2", "1.x", ">=1.2.2", "1 - 3", "0.1 || <1.2.0", "1.2.3"],
    },

    lt_123  => "<1.2.3", {
        allows => ["<=1.2.0", "<1", "=1.0.0", "^0.1 || ^1.4"],
        denies => ["1 - 3", ">1", "2.0.0", "2.0 || >9", ">1.0.0"],
    },

    lt_eq_123 => "<=1.2.3", {
        allows => ["<=1.2.0", "<1", "=1.0.0", "^0.1 || ^1.4", "=1.2.3"],
        denies => ["1 - 3", ">1.0.0", ">=1.0.0"],
    },

    eq_123_or_gt_400  => ">=1.2.3 || >4", {
        allows => [ "=1.2.3", ">4", "5.x", "5.2.x", ">=8.2.1", "2.0 || =5.6.7"],
        denies => ["<2", "1 - 7"],
    },

    between_two_and_eight => "2 - 8", {
        allows => [ "=2.2.3", "4 - 5"],
        denies => ["1 - 4", "5 - 9", ">3", "<=5"],
    },
}

create_tests_for! {
    // The function we are testing:
    allows_any

    greater_than_eq_123   => ">=1.2.3", {
        allows => ["<=1.2.4", "3.0.0", "<2", ">=3", ">3.0.0"],
        denies => ["<=1.2.0", "=1.0.0", "<1", "<=1.2"],
    },

    greater_than_123   => ">1.2.3", {
        allows => ["<=1.2.4", "3.0.0", "<2", ">=3", ">3.0.0"],
        denies => ["<=1.2.3", "=1.0.0", "<1", "<=1.2"],
    },

    eq_123   => "=1.2.3", {
        allows => ["=1.2.3", "1 - 2", "1.2.3"],
        denies => ["<1.2.3", "=1.0.0", "<=1.2", ">4.5.6", ">5"],
    },

    lt_eq_123  => "<=1.2.3", {
        allows => ["<=1.2.0", "<1.0.0", "=1.0.0", ">1.0.0", ">=1.2.0"],
        denies => [">=4.5.6", ">2.0.0", ">=2.0.0"],
    },

    lt_123  => "<1.2.3", {
        allows => ["<=2.2.0", "<2.0.0", "1.0.0", ">1.0.0", ">=1.2.0"],
        denies => ["2.0.0", ">1.8.0", ">=1.8.0"],
    },

    between_two_and_eight => "2 - 8", {
        allows => ["2.2.3", "4 - 10", ">4", ">4.0.0", "<=4.0.0", "<9.1.2"],
        denies => [">10", "10 - 11", "0 - 1"],
    },

    eq_123_or_gt_400  => "=1.2.3 || >4", {
        allows => [ "=1.2.3", ">3", "5.x", "5.2.x", ">=8.2.1", "2 - 7", "2.0 || 5.6.7"],
        denies => [ "=1.9.4 || 2 - 3"],
    },
}

#[cfg(test)]
mod intersection {
    use super::*;

    fn v(range: &'static str) -> Range {
        range.parse().unwrap()
    }

    #[test]
    fn gt_eq_123() {
        let base_range = v(">=1.2.3");

        let samples = vec![
            ("<=2.0.0", Some(">=1.2.3 <=2.0.0")),
            ("<2.0.0", Some(">=1.2.3 <2.0.0")),
            (">=2.0.0", Some(">=2.0.0")),
            (">2.0.0", Some(">2.0.0")),
            (">1.0.0", Some(">=1.2.3")),
            (">1.2.3", Some(">1.2.3")),
            ("<=1.2.3", Some("=1.2.3")),
            ("=2.0.0", Some("=2.0.0")),
            ("=1.1.1", None),
            ("<1.0.0", None),
        ];

        assert_ranges_match(base_range, samples);
    }

    #[test]
    fn gt_123() {
        let base_range = v(">1.2.3");

        let samples = vec![
            ("<=2.0.0", Some(">1.2.3 <=2.0.0")),
            ("<2.0.0", Some(">1.2.3 <2.0.0")),
            (">=2.0.0", Some(">=2.0.0")),
            (">2.0.0", Some(">2.0.0")),
            ("=2.0.0", Some("=2.0.0")),
            (">1.2.3", Some(">1.2.3")),
            ("<=1.2.3", None),
            ("=1.1.1", None),
            ("<1.0.0", None),
        ];

        assert_ranges_match(base_range, samples);
    }

    #[test]
    fn eq_123() {
        let base_range = v("=1.2.3");

        let samples = vec![
            ("<=2.0.0", Some("=1.2.3")),
            ("<2.0.0", Some("=1.2.3")),
            (">=2.0.0", None),
            (">2.0.0", None),
            ("=2.0.0", None),
            ("=1.2.3", Some("=1.2.3")),
            (">1.2.3", None),
            ("<=1.2.3", Some("=1.2.3")),
            ("=1.1.1", None),
            ("<1.0.0", None),
        ];

        assert_ranges_match(base_range, samples);
    }

    #[test]
    fn lt_123() {
        let base_range = v("<1.2.3");

        let samples = vec![
            ("<=2.0.0", Some("<1.2.3")),
            ("<2.0.0", Some("<1.2.3")),
            (">=2.0.0", None),
            (">=1.0.0", Some(">=1.0.0 <1.2.3")),
            (">2.0.0", None),
            ("=2.0.0", None),
            ("=1.2.3", None),
            (">1.2.3", None),
            ("<=1.2.3", Some("<1.2.3")),
            ("=1.1.1", Some("=1.1.1")),
            ("<1.0.0", Some("<1.0.0")),
        ];

        assert_ranges_match(base_range, samples);
    }

    #[test]
    fn lt_eq_123() {
        let base_range = v("<=1.2.3");

        let samples = vec![
            ("<=2.0.0", Some("<=1.2.3")),
            ("<2.0.0", Some("<=1.2.3")),
            (">=2.0.0", None),
            (">=1.0.0", Some(">=1.0.0 <=1.2.3")),
            (">2.0.0", None),
            ("=2.0.0", None),
            ("=1.2.3", Some("=1.2.3")),
            (">1.2.3", None),
            ("<=1.2.3", Some("<=1.2.3")),
            ("=1.1.1", Some("=1.1.1")),
            ("<1.0.0", Some("<1.0.0")),
        ];

        assert_ranges_match(base_range, samples);
    }

    #[test]
    fn multiple() {
        let base_range = v("<1 || 3 - 4");

        let samples = vec![("0.5 - 3.5.0", Some(">=0.5.0 <1.0.0||>=3.0.0 <=3.5.0"))];

        assert_ranges_match(base_range, samples);
    }

    fn assert_ranges_match(base: Range, samples: Vec<(&'static str, Option<&'static str>)>) {
        for (other, expected) in samples {
            let other = v(other);
            let resulting_range = base.intersect(&other).map(|v| v.to_string());
            assert_eq!(
                resulting_range.clone(),
                expected.map(|e| e.to_string()),
                "{} ∩ {} := {}",
                base,
                other,
                resulting_range.unwrap_or_else(|| "⊗".into())
            );
        }
    }
}

#[cfg(test)]
mod difference {
    use super::*;

    fn v(range: &'static str) -> Range {
        range.parse().unwrap()
    }

    #[test]
    fn gt_eq_123() {
        let base_range = v(">=1.2.3");

        let samples = vec![
            ("<=2.0.0", Some(">2.0.0")),
            ("<2.0.0", Some(">=2.0.0")),
            (">=2.0.0", Some(">=1.2.3 <2.0.0")),
            (">2.0.0", Some(">=1.2.3 <=2.0.0")),
            (">1.0.0", None),
            (">1.2.3", Some("=1.2.3")),
            ("<=1.2.3", Some(">1.2.3")),
            ("=1.1.1", Some(">=1.2.3")),
            ("<1.0.0", Some(">=1.2.3")),
            ("=2.0.0", Some(">=1.2.3 <2.0.0||>2.0.0")),
        ];

        assert_ranges_match(base_range, samples);
    }

    #[test]
    fn gt_123() {
        let base_range = v(">1.2.3");

        let samples = vec![
            ("<=2.0.0", Some(">2.0.0")),
            ("<2.0.0", Some(">=2.0.0")),
            (">=2.0.0", Some(">1.2.3 <2.0.0")),
            (">2.0.0", Some(">1.2.3 <=2.0.0")),
            (">1.0.0", None),
            (">1.2.3", None),
            ("<=1.2.3", Some(">1.2.3")),
            ("=1.1.1", Some(">1.2.3")),
            ("<1.0.0", Some(">1.2.3")),
            ("=2.0.0", Some(">1.2.3 <2.0.0||>2.0.0")),
        ];

        assert_ranges_match(base_range, samples);
    }

    #[test]
    fn eq_123() {
        let base_range = v("=1.2.3");

        let samples = vec![
            ("<=2.0.0", None),
            ("<2.0.0", None),
            (">=2.0.0", Some("=1.2.3")),
            (">2.0.0", Some("=1.2.3")),
            (">1.0.0", None),
            (">1.2.3", Some("=1.2.3")),
            ("=1.2.3", None),
            ("<=1.2.3", None),
            ("=1.1.1", Some("=1.2.3")),
            ("<1.0.0", Some("=1.2.3")),
            ("=2.0.0", Some("=1.2.3")),
        ];

        assert_ranges_match(base_range, samples);
    }

    #[test]
    fn lt_123() {
        let base_range = v("<1.2.3");

        let samples = vec![
            ("<=2.0.0", None),
            ("<2.0.0", None),
            (">=2.0.0", Some("<1.2.3")),
            (">2.0.0", Some("<1.2.3")),
            (">1.0.0", Some("<=1.0.0")),
            (">1.2.3", Some("<1.2.3")),
            ("<=1.2.3", None),
            ("=1.1.1", Some("<1.1.1||>1.1.1 <1.2.3")),
            ("<1.0.0", Some(">=1.0.0 <1.2.3")),
            ("2.0.0", Some("<1.2.3")),
        ];

        assert_ranges_match(base_range, samples);
    }

    #[test]
    fn lt_eq_123() {
        let base_range = v("<=1.2.3");

        let samples = vec![
            ("<=2.0.0", None),
            ("<2.0.0", None),
            (">=2.0.0", Some("<=1.2.3")),
            (">2.0.0", Some("<=1.2.3")),
            (">1.0.0", Some("<=1.0.0")),
            (">1.2.3", Some("<=1.2.3")),
            ("<=1.2.3", None),
            ("=1.1.1", Some("<1.1.1||>1.1.1 <=1.2.3")),
            ("<1.0.0", Some(">=1.0.0 <=1.2.3")),
            ("2.0.0", Some("<=1.2.3")),
        ];

        assert_ranges_match(base_range, samples);
    }

    #[test]
    fn multiple() {
        let base_range = v("<1 || 3 - 4");

        let samples = vec![("0.5 - 3.5.0", Some("<0.5.0||>3.5.0 <5.0.0-0"))];

        assert_ranges_match(base_range, samples);
    }

    fn assert_ranges_match(base: Range, samples: Vec<(&'static str, Option<&'static str>)>) {
        for (other, expected) in samples {
            let other = v(other);
            let resulting_range = base.difference(&other).map(|v| v.to_string());
            assert_eq!(
                resulting_range.clone(),
                expected.map(|e| e.to_string()),
                "{} \\ {} := {}",
                base,
                other,
                resulting_range.unwrap_or_else(|| "⊗".into())
            );
        }
    }
}

#[cfg(test)]
mod satisfies_ranges_tests {
    use super::*;

    macro_rules! refute {
        ($e:expr) => {
            assert!(!$e)
        };
        ($e:expr, $msg:expr) => {
            assert!(!$e, $msg)
        };
    }

    #[test]
    fn greater_than_equals() {
        let parsed = Range::parse(">=1.2.3").expect("unable to parse");

        refute!(parsed.satisfies(&(0, 2, 3).into()), "major too low");
        refute!(parsed.satisfies(&(1, 1, 3).into()), "minor too low");
        refute!(parsed.satisfies(&(1, 2, 2).into()), "patch too low");
        assert!(parsed.satisfies(&(1, 2, 3).into()), "exact");
        assert!(parsed.satisfies(&(2, 2, 3).into()), "above");
    }

    #[test]
    fn greater_than() {
        let parsed = Range::parse(">1.2.3").expect("unable to parse");

        refute!(parsed.satisfies(&(0, 2, 3).into()), "major too low");
        refute!(parsed.satisfies(&(1, 1, 3).into()), "minor too low");
        refute!(parsed.satisfies(&(1, 2, 2).into()), "patch too low");
        refute!(parsed.satisfies(&(1, 2, 3).into()), "exact");
        assert!(parsed.satisfies(&(1, 2, 4).into()), "above");
    }

    #[test]
    fn exact() {
        let parsed = Range::parse("=1.2.3").expect("unable to parse");

        refute!(parsed.satisfies(&(1, 2, 2).into()), "patch too low");
        assert!(parsed.satisfies(&(1, 2, 3).into()), "exact");
        refute!(parsed.satisfies(&(1, 2, 4).into()), "above");
    }

    #[test]
    fn less_than() {
        let parsed = Range::parse("<1.2.3").expect("unable to parse");

        assert!(parsed.satisfies(&(0, 2, 3).into()), "major below");
        assert!(parsed.satisfies(&(1, 1, 3).into()), "minor below");
        assert!(parsed.satisfies(&(1, 2, 2).into()), "patch below");
        refute!(parsed.satisfies(&(1, 2, 3).into()), "exact");
        refute!(parsed.satisfies(&(1, 2, 4).into()), "above");
    }

    #[test]
    fn less_than_equals() {
        let parsed = Range::parse("<=1.2.3").expect("unable to parse");

        assert!(parsed.satisfies(&(0, 2, 3).into()), "major below");
        assert!(parsed.satisfies(&(1, 1, 3).into()), "minor below");
        assert!(parsed.satisfies(&(1, 2, 2).into()), "patch below");
        assert!(parsed.satisfies(&(1, 2, 3).into()), "exact");
        refute!(parsed.satisfies(&(1, 2, 4).into()), "above");
    }

    #[test]
    fn only_major() {
        let parsed = Range::parse("1").expect("unable to parse");

        refute!(parsed.satisfies(&(0, 2, 3).into()), "major below");
        assert!(parsed.satisfies(&(1, 0, 0).into()), "exact bottom of range");
        assert!(parsed.satisfies(&(1, 2, 2).into()), "middle");
        assert!(parsed.satisfies(&(2, 0, 0).into()), "exact top of range");
        assert!(parsed.satisfies(&(2, 7, 3).into()), "above");
    }
}

/// https://github.com/npm/node-semver/blob/master/test/fixtures/range-parse.js
#[cfg(test)]
mod tests {
    use super::*;
    use serde_derive::{Deserialize, Serialize};

    use pretty_assertions::assert_eq;

    macro_rules! range_parse_tests {
        ($($name:ident => $vals:expr),+ ,$(,)?) => {
            $(
                #[test]
                fn $name() {
                    let [input, expected] = $vals;

                    let parsed = Range::parse(input).expect("unable to parse");

                    assert_eq!(expected, parsed.to_string());
                }
            )+
        }

    }

    range_parse_tests![
        //       [input,   parsed and then `to_string`ed]
        exact => ["=1.0.0", "=1.0.0"],
        plain_patch => ["1.0.0", ">=1.0.0"],
        plain_minor => ["1.0", ">=1.0.0"],
        plain_major => ["1", ">=1.0.0"],
        major_minor_patch_brackets => ["[1.0.0,2.0.0)", ">=1.0.0 <2.0.0"],
        major_minor_brackets => ["[1.0,2.0)", ">=1.0.0 <2.0.0"],
        major_brackets => ["[1,2)", ">=1.0.0 <2.0.0"],
        exact_brackets => ["[1.0.0]", "=1.0.0"],
        major_minor_patch_range => ["1.0.0 - 2.0.0", ">=1.0.0 <=2.0.0"],
        only_major_versions =>  ["1 - 2", ">=1.0.0 <3.0.0-0"],
        only_major_and_minor => ["1.0 - 2.0", ">=1.0.0 <2.1.0-0"],
        mixed_major_minor => ["1.2 - 3.4.5", ">=1.2.0 <=3.4.5"],
        mixed_major_minor_2 => ["1.2.3 - 3.4", ">=1.2.3 <3.5.0-0"],
        minor_minor_range => ["1.2 - 3.4", ">=1.2.0 <3.5.0-0"],
        single_sided_only_major => ["1", ">=1.0.0"],
        single_sided_lower_equals_bound =>  [">=1.0.0", ">=1.0.0"],
        single_sided_lower_equals_bound_2 => [">=0.1.97", ">=0.1.97"],
        single_sided_lower_bound => [">1.0.0", ">1.0.0"],
        single_sided_upper_equals_bound => ["<=2.0.0", "<=2.0.0"],
        single_sided_upper_equals_bound_with_minor => ["<=2.0", "<=2.0.0-0"],
        single_sided_upper_bound => ["<2.0.0", "<2.0.0"],
        major_and_minor => ["2.3", ">=2.3.0"],
        major_dot_x => ["2.x", ">=2.0.0"],
        x_and_asterisk_version => ["2.x.x", ">=2.0.0"],
        patch_x => ["1.2.x", ">=1.2.0"],
        minor_asterisk_patch_asterisk => ["2.*.*", ">=2.0.0"],
        patch_asterisk => ["1.2.*", ">=1.2.0"],
        caret_zero => ["^0", "<1.0.0-0"],
        caret_zero_minor => ["^0.1", ">=0.1.0 <0.2.0-0"],
        caret_one => ["^1.0", ">=1.0.0 <2.0.0-0"],
        caret_minor => ["^1.2", ">=1.2.0 <2.0.0-0"],
        caret_patch => ["^0.0.1", ">=0.0.1 <0.0.2-0"],
        caret_with_patch =>   ["^0.1.2", ">=0.1.2 <0.2.0-0"],
        caret_with_patch_2 => ["^1.2.3", ">=1.2.3 <2.0.0-0"],
        tilde_one => ["~1", ">=1.0.0 <2.0.0-0"],
        tilde_minor => ["~1.0", ">=1.0.0 <1.1.0-0"],
        tilde_minor_2 => ["~2.4", ">=2.4.0 <2.5.0-0"],
        // TODO: This test is broken and I have *no idea* why. I spent enough time on it, so if this affects you, fix it yourself <3
        // tilde_with_greater_than_patch => ["~>3.2.1", ">=3.2.1 <3.3.0-0"],
        tilde_major_minor_zero => ["~1.1.0", ">=1.1.0 <1.2.0-0"],
        grater_than_equals_one => [">=1", ">=1.0.0"],
        greater_than_one => [">1", ">=2.0.0"],
        less_than_one_dot_two => ["<1.2", "<1.2.0-0"],
        greater_than_one_dot_two => [">1.2", ">=1.3.0"],
        greater_than_with_prerelease => [">1.1.0-beta-10", ">1.1.0-beta-10"],
        either_one_version_or_the_other => ["0.1.20 || 1.2.4", ">=0.1.20||>=1.2.4"],
        either_one_version_range_or_another => [">=0.2.3 || <0.0.1", ">=0.2.3||<0.0.1"],
        either_x_version_works => ["1.2.x || 2.x", ">=1.2.0||>=2.0.0"],
        either_asterisk_version_works => ["1.2.* || 2.*", ">=1.2.0||>=2.0.0"],
        one_two_three_or_greater_than_four => ["1.2.3 || >4", ">=1.2.3||>=5.0.0"],
        any_version_asterisk => ["*", ">=0.0.0"],
        any_version_x => ["x", ">=0.0.0"],
        whitespace_1 => [">= 1.0.0", ">=1.0.0"],
        whitespace_2 => [">=  1.0.0", ">=1.0.0"],
        whitespace_3 => [">=   1.0.0", ">=1.0.0"],
        whitespace_4 => ["> 1.0.0", ">1.0.0"],
        whitespace_5 => [">  1.0.0", ">1.0.0"],
        whitespace_6 => ["<=   2.0.0", "<=2.0.0"],
        whitespace_7 => ["<= 2.0.0", "<=2.0.0"],
        whitespace_8 => ["<=  2.0.0", "<=2.0.0"],
        whitespace_9 => ["<    2.0.0", "<2.0.0"],
        whitespace_10 => ["<\t2.0.0", "<2.0.0"],
        whitespace_11 => ["^ 1", ">=1.0.0 <2.0.0-0"],
        whitespace_12 => ["~> 1", ">=1.0.0 <2.0.0-0"],
        whitespace_13 => ["~ 1.0", ">=1.0.0 <1.1.0-0"],
        beta          => ["^0.0.1-beta", ">=0.0.1-beta <0.0.2-0"],
        beta_4        => ["^1.2.3-beta.4", ">=1.2.3-beta.4 <2.0.0-0"],
        pre_release_on_both => ["1.0.0-alpha - 2.0.0-beta", ">=1.0.0-alpha <=2.0.0-beta"],
        single_sided_lower_bound_with_pre_release => [">1.0.0-alpha", ">1.0.0-alpha"],
    ];

    /*
    // From here onwards we might have to deal with pre-release tags to?
    [">01.02.03", ">1.2.3", true],
    [">01.02.03", null],
    ["~1.2.3beta", ">=1.2.3-beta <1.3.0-0", { loose: true }],
    ["~1.2.3beta", null],
    ["^ 1.2 ^ 1", ">=1.2.0 <2.0.0-0 >=1.0.0"],
    [">X", "<0.0.0-0"],
    ["<X", "<0.0.0-0"],
    ["<x <* || >* 2.x", "<0.0.0-0"],
    */

    #[derive(Serialize, Deserialize, Eq, PartialEq)]
    struct WithVersionReq {
        req: Range,
    }

    #[test]
    fn read_version_req_from_string() {
        let v: WithVersionReq = serde_json::from_str(r#"{"req":"^1.2.3"}"#).unwrap();

        assert_eq!(v.req, "^1.2.3".parse().unwrap(),);
    }

    #[test]
    fn serialize_a_range_to_string() {
        let output = serde_json::to_string(&WithVersionReq {
            req: Range {
                predicates: vec![ComparatorSet::at_most(Predicate::Excluding(
                    "1.2.3".parse().unwrap(),
                ))
                .unwrap()],
            },
        })
        .unwrap();
        let expected: String = r#"{"req":"<1.2.3"}"#.into();

        assert_eq!(output, expected);
    }
}

#[cfg(test)]
mod ranges {
    use super::*;

    #[test]
    fn one() {
        let r = ComparatorSet::new(
            Bound::Lower(Predicate::Including((1, 2, 0).into())),
            Bound::Upper(Predicate::Excluding((3, 3, 4).into())),
        )
        .unwrap();

        assert_eq!(r.to_string(), ">=1.2.0 <3.3.4")
    }
}

*/
