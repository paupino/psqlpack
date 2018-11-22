use std::str::FromStr;

use ast::*;
use connection::Connection;
use errors::{PsqlpackError, PsqlpackResult, PsqlpackResultExt};
use errors::PsqlpackErrorKind::*;
use model::Extension;
use semver::Semver;
use sql::lexer;
use sql::parser::{FunctionArgumentListParser, FunctionReturnTypeParser};

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
            .chain_err(|| QueryExtensionsError)?
            .iter()
            .map(|row| row.into())
            .collect();

        dbtry!(db_conn.finish());

        Ok(Capabilities {
            server_version: version,
            extensions,
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
    fn schemata(&self, conn: &PostgresConnection, database: &str) -> PsqlpackResult<Vec<SchemaDefinition>>;
    fn types(&self, conn: &PostgresConnection) -> PsqlpackResult<Vec<TypeDefinition>>;
    fn functions(&self, conn: &PostgresConnection) -> PsqlpackResult<Vec<FunctionDefinition>>;
}

impl DefinableCatalog for Capabilities {
    fn schemata(&self, conn: &PostgresConnection, database: &str) -> PsqlpackResult<Vec<SchemaDefinition>> {
        let schemata = conn
            .query(Q_SCHEMAS, &[&database])
            .chain_err(|| PackageQuerySchemasError)?
            .iter()
            .map(|row| row.into())
            .collect();
        Ok(schemata)
    }

    fn types(&self, conn: &PostgresConnection) -> PsqlpackResult<Vec<TypeDefinition>> {
        let types = conn
            .query(&format!("{} {}", CTE_TYPES, Q_CTE_STANDARD), &[])
            .chain_err(|| PackageQueryTypesError)?
            .iter()
            .map(|row| row.into())
            .collect();
        Ok(types)
    }

    fn functions(&self, conn: &PostgresConnection) -> PsqlpackResult<Vec<FunctionDefinition>> {
        let mut functions = Vec::new();
        let query = &conn
            .query(&format!("{} {}", CTE_FUNCTIONS, Q_CTE_STANDARD), &[])
            .chain_err(|| PackageQueryFunctionsError)?;
        for row in query {
            let function = parse_function(&row)?;
            functions.push(function);
        }
        Ok(functions)
    }
}

impl<'a> DefinableCatalog for ExtensionCapabilities<'a> {
    fn schemata(&self, _conn: &PostgresConnection, _database: &str) -> PsqlpackResult<Vec<SchemaDefinition>> {
        // Schema is hard to retrieve. Let's assume it's not necessary for extensions for now.
        Ok(Vec::new())
    }

    fn types(&self, conn: &PostgresConnection) -> PsqlpackResult<Vec<TypeDefinition>> {
        let types = conn
            .query(&format!("{} {}", CTE_TYPES, Q_CTE_EXTENSION), &[&self.extension.name])
            .chain_err(|| PackageQueryTypesError)?
            .iter()
            .map(|row| row.into())
            .collect();
        Ok(types)
    }

    fn functions(&self, conn: &PostgresConnection) -> PsqlpackResult<Vec<FunctionDefinition>> {
        let mut functions = Vec::new();
        let query = &conn
            .query(&format!("{} {}", CTE_FUNCTIONS, Q_CTE_EXTENSION), &[&self.extension.name])
            .chain_err(|| PackageQueryFunctionsError)?;
        for row in query {
            let function = parse_function(&row)?;
            functions.push(function);
        }
        Ok(functions)
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
static Q_CTE_STANDARD: &'static str = "
    SELECT c.*
    FROM cte c
    WHERE NOT EXISTS (SELECT 1 FROM pg_depend WHERE pg_depend.objid=c.oid AND deptype IN ('e','i'))";
static Q_CTE_EXTENSION: &'static str = "
    SELECT c.*
    FROM cte c
    INNER JOIN pg_depend d ON d.objid=c.oid
    INNER JOIN pg_extension e ON d.refobjid = e.oid
    WHERE d.deptype = 'e' and e.extname = $1";

impl<'row> From<Row<'row>> for Extension {
    fn from(row: Row) -> Self {
        Extension {
            name: row.get(0),
            version: row.get(1),
            installed: row.get(2),
        }
    }
}

static Q_SCHEMAS: &'static str = "SELECT schema_name FROM information_schema.schemata
                                  WHERE catalog_name = $1 AND schema_name !~* 'pg_|information_schema'";
impl<'row> From<Row<'row>> for SchemaDefinition {
    fn from(row: Row) -> Self {
        SchemaDefinition { name: row.get(0) }
    }
}

// Types: https://www.postgresql.org/docs/9.6/sql-createtype.html
// typcategory: https://www.postgresql.org/docs/9.6/catalog-pg-type.html#CATALOG-TYPCATEGORY-TABLE
static CTE_TYPES: &'static str = "
    WITH cte AS (
        SELECT pg_type.oid, typcategory, nspname, typname, array_agg(labels.enumlabel) AS enumlabels
        FROM pg_type
        INNER JOIN pg_namespace ON pg_namespace.oid=typnamespace
        LEFT JOIN (
            SELECT enumtypid, enumlabel
            FROM pg_catalog.pg_enum
            ORDER BY enumtypid, enumsortorder
        ) labels ON labels.enumtypid=pg_type.oid
        WHERE
            -- exclude pg schemas and information catalog
            nspname !~* 'pg_|information_schema' AND
            -- Types beginning with _ are auto created (e.g. arrays)
            typname !~ '^_'
        GROUP BY pg_type.oid, typcategory, nspname, typname
        ORDER BY pg_type.oid, typcategory, nspname, typname
    )
";

impl<'row> From<Row<'row>> for TypeDefinition {
    fn from(row: Row) -> Self {
        let category: i8 = row.get(1);
        let category = category as u8;
        let schema = row.get(2);
        let name = row.get(3);
        let kind = match category as char {
            // TODO: All types
            'C' => TypeDefinitionKind::Composite, // TODO add composite details
            'E' => TypeDefinitionKind::Enum(row.get(4)),
            'R' => TypeDefinitionKind::Range, // TODO add range details
            'U' => TypeDefinitionKind::UserDefined,
            kind => panic!("Unexpected kind: {}", kind),
        };

        TypeDefinition {
            name: ObjectName {
                schema,
                name,
            },
            kind,
        }
    }
}

static CTE_FUNCTIONS: &'static str = "
    WITH cte AS (
        SELECT
            pg_proc.oid,
            nspname,
            proname,
            prosrc,
            pg_get_function_arguments(pg_proc.oid),
            lanname,
            pg_get_function_result(pg_proc.oid)
        FROM pg_proc
        JOIN pg_namespace ON
            pg_namespace.oid = pg_proc.pronamespace
        JOIN pg_language ON
            pg_language.oid = pg_proc.prolang
        WHERE nspname !~* 'pg_|information_schema'
    )";

fn parse_function(row: &Row) -> PsqlpackResult<FunctionDefinition> {
    let schema_name: String = row.get(1);
    let function_name: String = row.get(2);
    let function_src: String = row.get(3);
    let raw_args: String = row.get(4);
    let lan_name: String = row.get(5);
    let raw_result: String = row.get(6);

    // Parse some of the results
    let language = match &lan_name[..] {
        "internal" => FunctionLanguage::Internal,
        "c" => FunctionLanguage::C,
        "sql" => FunctionLanguage::SQL,
        _ => FunctionLanguage::PostgreSQL,
    };

    fn lexical(err: lexer::LexicalError) -> PsqlpackError {
        LexicalError(
            err.line.to_owned(),
            err.line_number,
            err.start_pos,
            err.end_pos,
        ).into()
    };
    fn parse(err: lalrpop_util::ParseError<(), lexer::Token, &'static str>) -> PsqlpackError {
        InlineParseError(err).into()
    };

    let function_args = if raw_args.is_empty() {
        Vec::new()
    } else {
        lexer::tokenize(&raw_args)
            .map_err(lexical)
            .and_then(|tokens| {
                FunctionArgumentListParser::new().parse(tokens).map_err(parse)
            })
            .chain_err(|| PackageFunctionArgsInspectError(raw_args))?
    };
    let return_type = lexer::tokenize(&raw_result)
        .map_err(&lexical)
        .and_then(|tokens| {
            FunctionReturnTypeParser::new().parse(tokens).map_err(parse)
        })
        .chain_err(|| PackageFunctionReturnTypeInspectError(raw_result))?;

    // Set up the function definition
    Ok(FunctionDefinition {
        name: ObjectName {
            schema: Some(schema_name),
            name: function_name,
        },
        arguments: function_args,
        return_type,
        body: function_src,
        language,
    })
}
