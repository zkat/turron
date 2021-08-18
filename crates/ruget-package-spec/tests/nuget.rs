use ruget_package_spec::{PackageSpec, PackageSpecError};
use ruget_semver::Range;

type Result<T> = std::result::Result<T, PackageSpecError>;

fn parse(input: &str) -> Result<PackageSpec> {
    input.parse()
}

fn version_req(input: &str) -> Option<Range> {
    Some(Range::parse(input).unwrap())
}

#[test]
fn nuget_pkg_basic() -> Result<()> {
    let res = parse("hello-world")?;
    assert_eq!(
        res,
        PackageSpec::NuGet {
            name: "hello-world".into(),
            requested: None
        }
    );
    Ok(())
}

#[test]
fn nuget_pkg_prefixed() -> Result<()> {
    let res = parse("nuget:hello-world")?;
    assert_eq!(
        res,
        PackageSpec::NuGet {
            name: "hello-world".into(),
            requested: None
        }
    );
    Ok(())
}

#[test]
fn nuget_pkg_with_req() -> Result<()> {
    let res = parse("hello-world@1.2.3")?;
    assert_eq!(
        res,
        PackageSpec::NuGet {
            name: "hello-world".into(),
            requested: Some(Range::parse("1.2.3").unwrap())
        }
    );
    Ok(())
}

#[test]
fn odd_nuget_example_with_prerelease() -> Result<()> {
    let res = parse("world@>1.1.0-beta-10")?;
    assert_eq!(
        res,
        PackageSpec::NuGet {
            name: "world".into(),
            requested: version_req(">1.1.0-beta-10"),
        }
    );
    Ok(())
}

#[test]
fn approximately_equivalent_version() -> Result<()> {
    let res = parse("world@~1.1.0")?;
    assert_eq!(
        res,
        PackageSpec::NuGet {
            name: "world".into(),
            requested: version_req("~1.1.0"),
        }
    );
    Ok(())
}

#[test]
fn compatible_equivalent_version() -> Result<()> {
    let res = parse("world@^1.1.0")?;
    assert_eq!(
        res,
        PackageSpec::NuGet {
            name: "world".into(),
            requested: version_req("^1.1.0"),
        }
    );
    Ok(())
}

#[test]
fn x_version() -> Result<()> {
    let res = parse("world@1.1.x")?;
    assert_eq!(
        res,
        PackageSpec::NuGet {
            name: "world".into(),
            requested: version_req("1.1.x"),
        }
    );
    Ok(())
}

#[test]
fn hyphen_version_range() -> Result<()> {
    let res = parse("world@1.5.0 - 2.1.0")?;
    assert_eq!(
        res,
        PackageSpec::NuGet {
            name: "world".into(),
            requested: version_req("1.5.0 - 2.1.0"),
        }
    );
    Ok(())
}

#[test]
fn alternate_version_ranges() -> Result<()> {
    let res = parse("world@1.5.0 - 2.1.0 || 2.3.x")?;
    assert_eq!(
        res,
        PackageSpec::NuGet {
            name: "world".into(),
            requested: version_req("1.5.0 - 2.1.0 || 2.3.x"),
        }
    );
    Ok(())
}

#[test]
fn nuget_pkg_bad_tag() -> Result<()> {
    let res = parse("hello-world@%&W$@#$");
    assert!(res.is_err());
    Ok(())
}
