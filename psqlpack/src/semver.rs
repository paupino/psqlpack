use std::cmp;
use std::fmt;
use std::str::FromStr;

use regex::Regex;

#[derive(Clone, Copy, Debug)]
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

impl cmp::PartialEq for Semver {
    fn eq(&self, other: &Semver) -> bool {
        self.cmp(other) == cmp::Ordering::Equal
    }
}

impl Eq for Semver {}

impl cmp::PartialOrd for Semver {
    fn partial_cmp(&self, other: &Semver) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl cmp::Ord for Semver {
    fn cmp(&self, other: &Semver) -> cmp::Ordering {
        if self.major > other.major {
            return cmp::Ordering::Greater;
        } else if self.major < other.major {
            return cmp::Ordering::Less;
        }

        if self.minor > other.minor {
            return cmp::Ordering::Greater;
        } else if self.minor < other.minor {
            return cmp::Ordering::Less;
        }

        let my_rev = self.revision.unwrap_or(0);
        let other_rev = other.revision.unwrap_or(0);
        if my_rev > other_rev {
            return cmp::Ordering::Greater;
        } else if my_rev < other_rev {
            return cmp::Ordering::Less;
        }

        cmp::Ordering::Equal
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

impl<'a> Into<Semver> for &'a str {
    fn into(self) -> Semver {
        Semver::from_str(self).unwrap()
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
            } else {
                if optional {
                    None
                } else {
                    Some(0)
                }
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
