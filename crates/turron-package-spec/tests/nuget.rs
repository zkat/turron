use dotnet_semver::Range;
use turron_package_spec::{PackageSpec, PackageSpecError};

type Result<T> = std::result::Result<T, PackageSpecError>;

fn parse(input: &str) -> Result<PackageSpec> {
    input.parse()
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
