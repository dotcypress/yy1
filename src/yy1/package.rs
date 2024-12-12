use regex::Regex;
use super::PackageMap;

pub struct PackageConverter {
    substitutions: Vec<PackageMap>,
    package_converters: Vec<(Regex, String)>,
}

impl PackageConverter {
    pub fn new(substitutions: Vec<PackageMap>) -> Self {
        let package_converters = vec![
            (
                Regex::new(r"Crystal_SMD_([0-9]+)[_-].+").unwrap(),
                r"XTAL-${1}".into(),
            ),
            (
                Regex::new(r"([VWDLTQ]?)F([NP]?)-([0-9]+)[-_].+").unwrap(),
                r"${1}F${2}-${3}".into(),
            ),
            (
                Regex::new(r"(.+)GA-([0-9]+)[_-].+").unwrap(),
                r"${1}GA-${2}".into(),
            ),
            (
                Regex::new(r"(.+)SO([DP]?)-([0-9]+)[_-]*.*").unwrap(),
                r"${1}SO${2}-${3}".into(),
            ),
            (Regex::new(r"LED_([0-9]+)_.+").unwrap(), r"${1}".into()),
            (Regex::new(r"[RCLD]_([0-9]+)_.+").unwrap(), r"${1}".into()),
        ];

        Self {
            package_converters,
            substitutions,
        }
    }

    pub fn rename(&self, package: &str) -> String {
        for package_map in &self.substitutions {
            if let Some(name) = package_map.rename(package) {
                return name
            }
        }
        for (re, replace) in &self.package_converters {
            if re.is_match(package) {
                return re.replace(package, replace).into();
            }
        }
        package.into()
    }
}
