use std::str::FromStr;

use ast::*;
use model::Extension;
use semver::Semver;
use connection::Connection;
use errors::{PsqlpackResult, PsqlpackResultExt};
use errors::PsqlpackErrorKind::*;

use slog::Logger;
use postgres::{Connection as PostgresConnection};
use postgres::rows::Row;
use postgres::types::{FromSql, Type, TEXT};

pub struct Capabilities {
    pub server_version: Semver,
    pub extensions: Vec<Extension>,
    pub database_exists: bool,
}

impl Capabilities {
    pub fn from_connection(log: &Logger, connection: &Connection) -> PsqlpackResult<Capabilities> {
        let log = log.new(o!("capabilities" => "from_connection"));

        trace!(log, "Connecting to host");
        let mut db_conn = connection.connect_host()?;

        let version = Self::server_version(&db_conn)?;

        let db_result = dbtry!(db_conn.query(Q_DATABASE_EXISTS, &[&connection.database()]));
        let exists = !db_result.is_empty();

        // If it exists, then connect directly as we'll get better results
        if exists {
            dbtry!(db_conn.finish());
            db_conn = connection.connect_database()?;
        }

        let extensions = db_conn
            .query(Q_EXTENSIONS, &[])
            .chain_err(|| QueryExtensionsError)?;

        dbtry!(db_conn.finish());

        Ok(Capabilities {
            server_version: version,
            extensions: map!(extensions),
            database_exists: exists,
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

    pub fn available_extensions(&self, name: &str, version: Option<Semver>) -> Vec<&Extension> {
        let mut available = self.extensions
                            .iter()
                            .filter(|x| x.name.eq(name) && (version.is_none() ||
                                version.unwrap().eq(&x.version)))
                            .collect::<Vec<_>>();
        available.sort_by(|a, b| b.version.cmp(&a.version));
        available
    }

    // I'm not incredibly happy with this name, but it'll work for now
    pub(crate) fn with_context<'a>(&'a self, extension: &'a Extension) -> ExtensionCapabilities<'a> {
        ExtensionCapabilities {
            capabilities: self,
            extension,
        }
    }
}

pub(crate) struct ExtensionCapabilities<'a> {
    capabilities: &'a Capabilities,
    extension: &'a Extension,
}

pub trait DefinableCatalog {
    fn query_schemata(&self, conn: &PostgresConnection, database: &str) -> PsqlpackResult<Vec<SchemaDefinition>>;
    fn query_types(&self, conn: &PostgresConnection) -> PsqlpackResult<Vec<TypeDefinition>>;
}

impl DefinableCatalog for Capabilities {
    fn query_schemata(&self, conn: &PostgresConnection, database: &str) -> PsqlpackResult<Vec<SchemaDefinition>> {
        let schemata = conn
            .query(Q_SCHEMAS, &[&database])
            .chain_err(|| PackageQuerySchemasError)?;
        Ok(map!(schemata))
    }

    fn query_types(&self, conn: &PostgresConnection) -> PsqlpackResult<Vec<TypeDefinition>> {
        let types = conn
            .query(Q_ENUMS, &[])
            .chain_err(|| PackageQueryTypesError)?;
        Ok(map!(types))
    }
}

impl<'a> DefinableCatalog for ExtensionCapabilities<'a> {
    fn query_schemata(&self, conn: &PostgresConnection, database: &str) -> PsqlpackResult<Vec<SchemaDefinition>> {
        // Schema is hard to retrieve. Let's assume it's not necessary for extensions for now.
        Ok(Vec::new())
    }

    fn query_types(&self, conn: &PostgresConnection) -> PsqlpackResult<Vec<TypeDefinition>> {
        // TODO
        Ok(Vec::new())
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

static Q_DATABASE_EXISTS: &'static str = "SELECT 1 FROM pg_database WHERE datname=$1;";
static Q_EXTENSIONS: &'static str = "SELECT name, version, installed, requires
                                     FROM pg_available_extension_versions ";

static Q_SCHEMAS: &'static str = "SELECT schema_name FROM information_schema.schemata
                                  WHERE catalog_name = $1 AND schema_name !~* 'pg_|information_schema'";
impl<'row> From<Row<'row>> for SchemaDefinition {
    fn from(row: Row) -> Self {
        SchemaDefinition { name: row.get(0) }
    }
}

static Q_ENUMS: &'static str = "SELECT typname, array_agg(enumlabel)
                                FROM pg_catalog.pg_type
                                INNER JOIN pg_catalog.pg_namespace ON
                                    pg_namespace.oid=typnamespace
                                INNER JOIN (
                                    SELECT enumtypid, enumlabel
                                    FROM pg_catalog.pg_enum
                                    ORDER BY enumtypid, enumsortorder
                                 ) labels ON
                                    labels.enumtypid=pg_type.oid
                                WHERE typcategory IN ('E') AND
                                      nspname='public' AND
                                      substr(typname, 1, 1) <> '_'
                                GROUP BY typname";
impl<'row> From<Row<'row>> for TypeDefinition {
    fn from(row: Row) -> Self {
        TypeDefinition {
            name: row.get(0),
            kind: TypeDefinitionKind::Enum(row.get(1)),
        }
    }
}
