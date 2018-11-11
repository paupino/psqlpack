use std::str::FromStr;

use semver::Semver;
use connection::Connection;
use errors::{PsqlpackResult, PsqlpackResultExt};
use errors::PsqlpackErrorKind::*;

use slog::Logger;
use postgres::{Connection as PostgresConnection};
use postgres::rows::Row;
use postgres::types::{FromSql, Type, TEXT};

#[derive(Clone, Debug, PartialEq)]
pub struct Extension {
    pub name: String,
    pub version: Semver,
    pub installed: bool,
}

pub struct Capabilities {
    pub server_version: Semver,
    pub extensions: Vec<Extension>,
}

impl Capabilities {
    pub fn from_connection(log: &Logger, connection: &Connection) -> PsqlpackResult<Capabilities> {
        let log = log.new(o!("capabilities" => "from_connection"));

        trace!(log, "Connecting to host");
        let db_conn = connection.connect_host()?;

        let version = Self::server_version(&db_conn)?;

        let extensions = db_conn
            .query(Q_EXTENSIONS, &[])
            .chain_err(|| QueryExtensionsError)?;

        dbtry!(db_conn.finish());

        Ok(Capabilities {
            server_version: version,
            extensions: map!(extensions),
        })
    }

    fn server_version(conn: &PostgresConnection) -> PsqlpackResult<Semver> {
        let rows = conn.query("SHOW SERVER_VERSION;", &[])
            .map_err(|e| DatabaseError(format!("Failed to retrieve server version: {}", e)))?;
        let row = rows.iter().last();
        if let Some(row) = row {
            let version: Semver = row.get(0);
            Ok(version)
        } else {
            bail!(DatabaseError("Failed to retrieve version from server".into()))
        }
    }
}

impl<'row> From<Row<'row>> for Extension {
    fn from(row: Row) -> Self {
        Extension {
            name: row.get(0),
            version: row.get(1),
            installed: row.get(2),
        }
    }
}

impl FromSql for Semver {
    // TODO: Better error handling
    fn from_sql(_: &Type, raw: &[u8]) -> Result<Semver, Box<::std::error::Error + Sync + Send>> {
        let version = String::from_utf8_lossy(raw);
        Ok(Semver::from_str(&version).unwrap())
    }

    fn accepts(ty: &Type) -> bool {
        *ty == TEXT
    }
}

static Q_EXTENSIONS: &'static str = "SELECT name, version, installed, requires
                                     FROM pg_available_extension_versions ";
