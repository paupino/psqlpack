use std::cmp;
use std::fmt;
use postgres::{Connection as PostgresConnection};
use regex::Regex;

use errors::PsqlpackResult;
use errors::PsqlpackErrorKind::*;

#[derive(Debug)]
pub struct Semver {
    major: u32,
    minor: u32,
    revision: u32,
}

impl Semver {
    pub fn new(major: u32, minor: u32, rev: u32) -> Self {
        Semver {
            major: major,
            minor: minor,
            revision: rev,
        }
    }
}

impl fmt::Display for Semver {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.revision)
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

        if self.revision > other.revision {
            return cmp::Ordering::Greater;
        } else if self.revision < other.revision {
            return cmp::Ordering::Less;
        }

        cmp::Ordering::Equal
    }
}

pub trait ServerVersion {
    fn server_version(&self) -> PsqlpackResult<Semver>;
}

lazy_static! {
    static ref VERSION : Regex = Regex::new("(\\d+)\\.(\\d+)\\.(\\d+)").unwrap();
}

impl ServerVersion for PostgresConnection {
    fn server_version(&self) -> PsqlpackResult<Semver> {
        let rows = self.query("SELECT version();", &[])
                      .map_err(|e| DatabaseError(format!("Failed to retrieve server version: {}", e)))?;
        let row = rows.iter().last();
        if let Some(row) = row {
            let version: String = row.get(0);
            let caps = VERSION.captures(&version[..]).unwrap();
            Ok(Semver {
                major: caps.get(1).unwrap().as_str().parse::<u32>().unwrap(),
                minor: caps.get(2).unwrap().as_str().parse::<u32>().unwrap(),
                revision: caps.get(3).unwrap().as_str().parse::<u32>().unwrap(),
            })
        } else {
            bail!(DatabaseError("Failed to retrieve version from server".into()))
        }
    }
}
