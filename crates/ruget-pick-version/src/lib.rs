use ruget_semver::{Version, Range};

pub fn pick_version(req: &Range, versions: &[Version]) -> Option<Version> {
    VersionPicker::default().pick_version(req, versions)
}

#[derive(Debug, Clone, Default)]
pub struct VersionPicker {
    pub force_floating: bool,
}

impl VersionPicker {
    pub fn new(force_floating: bool) -> Self {
        Self { force_floating }
    }

    pub fn pick_version(&self, req: &Range, versions: &[Version]) -> Option<Version> {
        let floating = self.force_floating || req.is_floating();
        let include_pre = req.has_pre_release();
        let mut versions = versions
            .iter()
            .cloned()
            // If there's no prerelease in the VersionReq, don't check any prerelease versions.
            .filter(|v| include_pre || v.pre_release.is_empty())
            .collect::<Vec<_>>();
        versions.sort_unstable();

        if floating {
            versions.reverse();
        }
        versions.into_iter().find(|v| req.satisfies(v))
    }
}

#[cfg(test)]
mod tests {
    use super::VersionPicker;

    #[test]
    fn basic() {
        let picker = VersionPicker::default();
        let req = "[1.2.3,)".parse().unwrap();
        let versions = vec!["1.2.0", "1.2.2", "1.2.3", "1.2.3-alpha", "1.2.4", "2.0.0"]
            .into_iter()
            .map(|v| v.parse().unwrap())
            .collect::<Vec<_>>();
        let picked = picker.pick_version(&req, &versions);
        assert_eq!(Some("1.2.3".parse().unwrap()), picked);
    }

    #[test]
    fn partial() {
        let picker = VersionPicker::default();
        let req = "1".parse().unwrap();
        let versions = vec!["1.2.0", "1.2.0-beta", "2.0.0"]
            .into_iter()
            .map(|v| v.parse().unwrap())
            .collect::<Vec<_>>();
        let picked = picker.pick_version(&req, &versions);
        assert_eq!(Some("1.2.0".parse().unwrap()), picked);
    }
}
