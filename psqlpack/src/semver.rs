use std::cmp;
use std::fmt;
use std::str::FromStr;

use regex::Regex;
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Semver {
    major: u32,
    minor: u32,
    revision: Option<u32>,
}

impl Semver {
    pub fn new(major: u32, minor: u32, revision: Option<u32>) -> Self {
        Semver { major, minor, revision }
    }
}

impl fmt::Display for Semver {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(revision) = self.revision {
            write!(f, "{}.{}.{}", self.major, self.minor, revision)
        } else {
            write!(f, "{}.{}", self.major, self.minor)
        }
    }
}

impl cmp::PartialOrd for Semver {
    fn partial_cmp(&self, other: &Semver) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl cmp::Ord for Semver {
    fn cmp(&self, other: &Semver) -> cmp::Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Less => return Ordering::Less,
            Ordering::Greater => return Ordering::Greater,
            Ordering::Equal => {}
        }

        match self.minor.cmp(&other.minor) {
            Ordering::Less => return Ordering::Less,
            Ordering::Greater => return Ordering::Greater,
            Ordering::Equal => {}
        }

        let my_rev = self.revision.unwrap_or(0);
        let other_rev = other.revision.unwrap_or(0);
        my_rev.cmp(&other_rev)
    }
}

impl serde::Serialize for Semver {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for Semver {
    fn deserialize<D>(deserializer: D) -> Result<Semver, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        deserializer.deserialize_str(SemverVisitor)
    }
}

struct SemverVisitor;

impl<'de> serde::de::Visitor<'de> for SemverVisitor {
    type Value = Semver;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a semantically versioned string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Semver::from_str(value).map_err(|_| E::invalid_value(serde::de::Unexpected::Str(value), &self))
    }
}

impl From<&str> for Semver {
    fn from(value: &str) -> Self {
        Semver::from_str(value).unwrap()
    }
}

lazy_static! {
    static ref VERSION: Regex = Regex::new("(\\d+)(\\.(\\d+))?(\\.(\\d+))?").unwrap();
}

impl FromStr for Semver {
    type Err = String;

    fn from_str(version: &str) -> Result<Self, Self::Err> {
        fn get_u32(caps: &::regex::Captures<'_>, pos: usize, optional: bool) -> Option<u32> {
            if let Some(rev) = caps.get(pos) {
                Some(rev.as_str().parse::<u32>().unwrap())
            } else if !optional {
                Some(0)
            } else {
                None
            }
        }

        let caps = match VERSION.captures(version) {
            Some(x) => x,
            None => return Err("Unexpected version format".into()),
        };
        let major = match get_u32(&caps, 1, false) {
            Some(x) => x,
            None => return Err("Unexpected major part".into()),
        };
        let minor = match get_u32(&caps, 3, false) {
            Some(x) => x,
            None => return Err("Unexpected minor part".into()),
        };
        let revision = get_u32(&caps, 5, true);
        Ok(Semver { major, minor, revision })
    }
}

#[cfg(test)]
mod tests {
    use super::Semver;
    use std::str::FromStr;

    use spectral::prelude::*;

    #[test]
    fn it_can_parse_version_strings() {
        let tests = &[
            ("11", "11.0"),
            ("10.4", "10.4"),
            ("9.4.18", "9.4.18"),
            ("9.6.9", "9.6.9"),
        ];
        for &(given, expected) in tests {
            let parsed = Semver::from_str(given);
            assert_that!(parsed).is_ok();
            assert_that!(parsed.unwrap().to_string()).is_equal_to(expected.to_string());
        }
    }
}
