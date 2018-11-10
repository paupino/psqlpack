use std::cmp;
use std::fmt;
use postgres::{Connection as PostgresConnection};
use postgres::types::{FromSql, Type, TEXT};
use regex::Regex;

use errors::PsqlpackResult;
use errors::PsqlpackErrorKind::*;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Semver {
    major: u32,
    minor: u32,
    revision: Option<u32>,
}

impl Semver {
    pub fn new(major: u32, minor: u32, rev: Option<u32>) -> Self {
        Semver {
            major: major,
            minor: minor,
            revision: rev,
        }
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

pub trait ServerVersion {
    fn server_version(&self) -> PsqlpackResult<Semver>;
}

lazy_static! {
    static ref VERSION : Regex = Regex::new("(\\d+)(\\.(\\d+))?(\\.(\\d+))?").unwrap();
}

fn parse_version_string(version: &str) -> Semver {
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

    let caps = VERSION.captures(version).unwrap();
    let major = get_u32(&caps, 1, false).unwrap();
    let minor = get_u32(&caps, 3, false).unwrap();
    let revision = get_u32(&caps, 5, true);
    Semver {
        major: major,
        minor: minor,
        revision: revision,
    }
}

impl ServerVersion for PostgresConnection {
    fn server_version(&self) -> PsqlpackResult<Semver> {
        let rows = self.query("SHOW SERVER_VERSION;", &[])
                      .map_err(|e| DatabaseError(format!("Failed to retrieve server version: {}", e)))?;
        let row = rows.iter().last();
        if let Some(row) = row {
            let version: String = row.get(0);
            Ok(parse_version_string(&version[..]))
        } else {
            bail!(DatabaseError("Failed to retrieve version from server".into()))
        }
    }
}

impl FromSql for Semver {
    // TODO: Better error handling
    fn from_sql(_: &Type, raw: &[u8]) -> Result<Semver, Box<::std::error::Error + Sync + Send>> {
        let version = String::from_utf8_lossy(raw);
        Ok(parse_version_string(&version))
    }

    fn accepts(ty: &Type) -> bool {
        *ty == TEXT
    }
}

#[cfg(test)]
mod tests {
    use super::parse_version_string;
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
            let parsed = parse_version_string(given);
            assert_that!(parsed.to_string()).is_equal_to(expected.to_string());
        }
    }
}
