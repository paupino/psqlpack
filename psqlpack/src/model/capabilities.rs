use std::collections::HashMap;
use std::str::FromStr;

use crate::ast::*;
use crate::connection::Connection;
use crate::errors::PsqlpackErrorKind::*;
use crate::errors::{PsqlpackError, PsqlpackResult, PsqlpackResultExt};
use crate::model::Extension;
use crate::semver::Semver;
use crate::sql::lexer;
use crate::sql::parser::{FunctionArgumentListParser, FunctionReturnTypeParser, SqlTypeParser};

use postgres::row::Row;
use postgres::types::{FromSql, Type};
use postgres::Client as PostgresClient;
use regex::Regex;
use slog::Logger;

pub struct Capabilities {
    pub server_version: Semver,
    pub extensions: Vec<Extension>,
    pub database_exists: bool,
}

impl Capabilities {
    pub fn from_connection(log: &Logger, connection: &Connection) -> PsqlpackResult<Capabilities> {
        let log = log.new(o!("capabilities" => "from_connection"));

        trace!(log, "Connecting to host");
        let mut client = connection.connect_host()?;

        let version = Self::server_version(&mut client)?;

        let db_result = dbtry!(client.query(Q_DATABASE_EXISTS, &[&connection.database()]));
        let exists = !db_result.is_empty();

        // If it exists, then connect directly as we'll get better results
        if exists {
            client = connection.connect_database()?;
        }

        let extensions = client
            .query(Q_EXTENSIONS, &[])
            .chain_err(|| QueryExtensionsError)?
            .iter()
            .map(|row| row.into())
            .collect();

        Ok(Capabilities {
            server_version: version,
            extensions,
            database_exists: exists,
        })
    }

    fn server_version(client: &mut PostgresClient) -> PsqlpackResult<Semver> {
        let rows = client
            .query("SHOW SERVER_VERSION;", &[])
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
        let mut available = self
            .extensions
            .iter()
            .filter(|x| x.name.eq(name) && (version.is_none() || version.unwrap().eq(&x.version)))
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
    fn schemata(&self, client: &mut PostgresClient, database: &str) -> PsqlpackResult<Vec<SchemaDefinition>>;
    fn types(&self, client: &mut PostgresClient) -> PsqlpackResult<Vec<TypeDefinition>>;
    fn functions(&self, client: &mut PostgresClient) -> PsqlpackResult<Vec<FunctionDefinition>>;
    fn tables(&self, client: &mut PostgresClient) -> PsqlpackResult<Vec<TableDefinition>>;
    fn indexes(&self, client: &mut PostgresClient) -> PsqlpackResult<Vec<IndexDefinition>>;
}

impl DefinableCatalog for Capabilities {
    fn schemata(&self, client: &mut PostgresClient, database: &str) -> PsqlpackResult<Vec<SchemaDefinition>> {
        let schemata = client
            .query(Q_SCHEMAS, &[&database])
            .chain_err(|| PackageQuerySchemasError)?
            .iter()
            .map(|row| row.into())
            .collect();
        Ok(schemata)
    }

    fn types(&self, client: &mut PostgresClient) -> PsqlpackResult<Vec<TypeDefinition>> {
        let types = client
            .query(&format!("{} {}", CTE_TYPES, Q_CTE_STANDARD)[..], &[])
            .chain_err(|| PackageQueryTypesError)?
            .iter()
            .map(|row| row.into())
            .collect();
        Ok(types)
    }

    fn functions(&self, client: &mut PostgresClient) -> PsqlpackResult<Vec<FunctionDefinition>> {
        let mut functions = Vec::new();
        let query = &client
            .query(&format!("{} {}", CTE_FUNCTIONS, Q_CTE_STANDARD)[..], &[])
            .chain_err(|| PackageQueryFunctionsError)?;
        for row in query {
            let function = parse_function(&row)?;
            functions.push(function);
        }
        Ok(functions)
    }

    fn tables(&self, client: &mut PostgresClient) -> PsqlpackResult<Vec<TableDefinition>> {
        let mut tables = HashMap::new();
        let query = &client
            .query(&format!("{} {}", CTE_TABLES, Q_CTE_STANDARD)[..], &[])
            .chain_err(|| PackageQueryTablesError)?;
        for row in query {
            let table: TableDefinition = row.into();
            tables.insert(table.name.to_string(), table);
        }

        // Get a list of columns and map them to the appropriate tables
        let query = &client
            .query(
                &format!("{} {} ORDER BY fqn, num", CTE_COLUMNS, Q_CTE_STANDARD)[..],
                &[],
            )
            .chain_err(|| PackageQueryColumnsError)?;
        for row in query {
            let fqn: String = row.get(1);
            if let Some(definition) = tables.get_mut(&fqn) {
                definition.columns.push(row.into());
            }
        }

        // Get a list of table constraints
        let query = &client
            .query(
                &format!("{} {} ORDER BY fqn", CTE_TABLE_CONSTRAINTS, Q_CTE_STANDARD)[..],
                &[],
            )
            .chain_err(|| PackageQueryTableConstraintsError)?;
        for row in query {
            let fqn: String = row.get(1);
            if let Some(definition) = tables.get_mut(&fqn) {
                definition.constraints.push(row.into());
            }
        }

        Ok(tables.into_iter().map(|(_, b)| b).collect())
    }

    fn indexes(&self, client: &mut PostgresClient) -> PsqlpackResult<Vec<IndexDefinition>> {
        // Get a list of indexes
        let mut indexes = Vec::new();
        let cte = match self.server_version.cmp(&Semver::new(9, 6, None)) {
            ::std::cmp::Ordering::Less => CTE_INDEXES_94_THRU_96,
            _ => CTE_INDEXES,
        };
        let query = &client
            .query(&format!("{} {}", cte, Q_CTE_STANDARD)[..], &[])
            .chain_err(|| PackageQueryIndexesError)?;
        for row in query {
            let index: IndexDefinition = row.into();
            indexes.push(index);
        }
        Ok(indexes)
    }
}

impl<'a> DefinableCatalog for ExtensionCapabilities<'a> {
    fn schemata(&self, _client: &mut PostgresClient, _database: &str) -> PsqlpackResult<Vec<SchemaDefinition>> {
        // Schema is hard to retrieve. Let's assume it's not necessary for extensions for now.
        Ok(Vec::new())
    }

    fn types(&self, client: &mut PostgresClient) -> PsqlpackResult<Vec<TypeDefinition>> {
        let types = client
            .query(
                &format!("{} {}", CTE_TYPES, Q_CTE_EXTENSION)[..],
                &[&self.extension.name],
            )
            .chain_err(|| PackageQueryTypesError)?
            .iter()
            .map(|row| row.into())
            .collect();
        Ok(types)
    }

    fn functions(&self, client: &mut PostgresClient) -> PsqlpackResult<Vec<FunctionDefinition>> {
        let mut functions = Vec::new();
        let query = &client
            .query(
                &format!("{} {}", CTE_FUNCTIONS, Q_CTE_EXTENSION)[..],
                &[&self.extension.name],
            )
            .chain_err(|| PackageQueryFunctionsError)?;
        for row in query {
            let function = parse_function(&row)?;
            functions.push(function);
        }
        Ok(functions)
    }

    fn tables(&self, client: &mut PostgresClient) -> PsqlpackResult<Vec<TableDefinition>> {
        let mut tables = HashMap::new();
        let query = &client
            .query(
                &format!("{} {}", CTE_TABLES, Q_CTE_EXTENSION)[..],
                &[&self.extension.name],
            )
            .chain_err(|| PackageQueryTablesError)?;
        for row in query {
            let table: TableDefinition = row.into();
            tables.insert(table.name.to_string(), table);
        }

        // Get a list of columns and map them to the appropriate tables
        let query = &client
            .query(
                &format!("{} {} ORDER BY fqn, num", CTE_COLUMNS, Q_CTE_EXTENSION)[..],
                &[&self.extension.name],
            )
            .chain_err(|| PackageQueryColumnsError)?;
        for row in query {
            let fqn: String = row.get(1);
            if let Some(definition) = tables.get_mut(&fqn) {
                definition.columns.push(row.into());
            }
        }

        // Get a list of table constraints
        let query = &client
            .query(
                &format!("{} {} ORDER BY fqn", CTE_TABLE_CONSTRAINTS, Q_CTE_EXTENSION)[..],
                &[&self.extension.name],
            )
            .chain_err(|| PackageQueryTableConstraintsError)?;
        for row in query {
            let fqn: String = row.get(1);
            if let Some(definition) = tables.get_mut(&fqn) {
                definition.constraints.push(row.into());
            }
        }

        Ok(tables.into_iter().map(|(_, b)| b).collect())
    }

    fn indexes(&self, client: &mut PostgresClient) -> PsqlpackResult<Vec<IndexDefinition>> {
        // Get a list of indexes
        let mut indexes = Vec::new();
        let cte = match self.capabilities.server_version.cmp(&Semver::new(9, 6, None)) {
            ::std::cmp::Ordering::Less => CTE_INDEXES_94_THRU_96,
            _ => CTE_INDEXES,
        };
        let query = &client
            .query(&format!("{} {}", cte, Q_CTE_EXTENSION)[..], &[&self.extension.name])
            .chain_err(|| PackageQueryIndexesError)?;
        for row in query {
            let index: IndexDefinition = row.into();
            indexes.push(index);
        }
        Ok(indexes)
    }
}

impl<'a> FromSql<'a> for Semver {
    // TODO: Better error handling
    fn from_sql(_: &Type, raw: &[u8]) -> Result<Semver, Box<dyn ::std::error::Error + Sync + Send>> {
        let version = String::from_utf8_lossy(raw);
        Ok(Semver::from_str(&version).unwrap())
    }

    fn accepts(ty: &Type) -> bool {
        *ty == Type::TEXT
    }
}

static Q_DATABASE_EXISTS: &str = "SELECT 1 FROM pg_database WHERE datname=$1;";
static Q_EXTENSIONS: &str = "SELECT name, version, installed, requires
                                     FROM pg_available_extension_versions ";
static Q_CTE_STANDARD: &str = "
    SELECT c.*
    FROM cte c
    WHERE NOT EXISTS (SELECT 1 FROM pg_depend WHERE pg_depend.objid=c.oid AND deptype IN ('e','i'))";
static Q_CTE_EXTENSION: &str = "
    SELECT c.*
    FROM cte c
    INNER JOIN pg_depend d ON d.objid=c.oid
    INNER JOIN pg_extension e ON d.refobjid = e.oid
    WHERE d.deptype = 'e' and e.extname = $1";

impl<'row> From<&Row> for Extension {
    fn from(row: &Row) -> Self {
        Extension {
            name: row.get(0),
            version: row.get(1),
            installed: row.get(2),
        }
    }
}

static Q_SCHEMAS: &str = "SELECT schema_name FROM information_schema.schemata
                                  WHERE catalog_name = $1 AND schema_name !~* 'pg_|information_schema'";
impl<'row> From<&Row> for SchemaDefinition {
    fn from(row: &Row) -> Self {
        SchemaDefinition { name: row.get(0) }
    }
}

// Types: https://www.postgresql.org/docs/9.6/sql-createtype.html
// typcategory: https://www.postgresql.org/docs/9.6/catalog-pg-type.html#CATALOG-TYPCATEGORY-TABLE
static CTE_TYPES: &str = "
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

impl<'row> From<&Row> for TypeDefinition {
    fn from(row: &Row) -> Self {
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
            name: ObjectName { schema, name },
            kind,
        }
    }
}

static CTE_FUNCTIONS: &str = "
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
        WHERE nspname !~* 'pg_|information_schema' AND
            proname !~ '^_'
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
            err.reason.to_owned(),
            err.line.to_owned(),
            err.line_number,
            err.start_pos,
            err.end_pos,
        )
        .into()
    }
    fn parse(err: lalrpop_util::ParseError<(), lexer::Token, &'static str>) -> PsqlpackError {
        InlineParseError(err).into()
    }

    let function_args = if raw_args.is_empty() {
        Vec::new()
    } else {
        lexer::tokenize_body(&raw_args)
            .map_err(lexical)
            .and_then(|tokens| FunctionArgumentListParser::new().parse(tokens).map_err(parse))
            .chain_err(|| PackageFunctionArgsInspectError(raw_args))?
    };
    let return_type = lexer::tokenize_body(&raw_result)
        .map_err(&lexical)
        .and_then(|tokens| FunctionReturnTypeParser::new().parse(tokens).map_err(parse))
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

static CTE_TABLES: &str = "
    WITH cte AS (
        SELECT
            pg_class.oid,
            nspname,
            relname
        FROM pg_class
        JOIN pg_namespace ON pg_namespace.oid = pg_class.relnamespace
        WHERE pg_class.relkind='r' AND
              nspname !~* 'pg_|information_schema'
    )";

impl<'row> From<&Row> for TableDefinition {
    fn from(row: &Row) -> Self {
        TableDefinition {
            name: ObjectName {
                schema: Some(row.get(1)),
                name: row.get(2),
            },
            columns: Vec::new(),     // This gets loaded later
            constraints: Vec::new(), // This gets loaded later
        }
    }
}

static CTE_COLUMNS: &str = "
    WITH cte AS (
        SELECT DISTINCT
            pgc.oid,
            CONCAT(ns.nspname, '.', pgc.relname) as fqn,
            ns.nspname as schema_name,
            pgc.relname as table_name,
            a.attnum as num,
            a.attname as name,
            CASE WHEN a.atttypid = ANY ('{int,int8,int2}'::regtype[])
                  AND pg_get_expr(def.adbin, def.adrelid) = 'nextval('''
                        || (pg_get_serial_sequence (a.attrelid::regclass::text, a.attname))::regclass
                        || '''::regclass)'
                THEN CASE a.atttypid
                        WHEN 'int'::regtype  THEN 'serial'
                        WHEN 'int8'::regtype THEN 'bigserial'
                        WHEN 'int2'::regtype THEN 'smallserial'
                     END
                ELSE format_type(a.atttypid, a.atttypmod)
            END AS data_type,
            a.attnotnull as notnull,
            coalesce(i.indisprimary,false) as primary_key,
            pg_get_expr(def.adbin, def.adrelid) as default
        FROM pg_attribute a
        INNER JOIN pg_class pgc ON pgc.oid = a.attrelid
        INNER JOIN pg_namespace ns ON ns.oid = pgc.relnamespace
        LEFT JOIN pg_index i ON pgc.oid = i.indrelid AND i.indkey[0] = a.attnum
        LEFT JOIN pg_attrdef def ON a.attrelid = def.adrelid AND a.attnum = def.adnum
        WHERE attnum > 0 AND pgc.relkind='r' AND NOT a.attisdropped AND ns.nspname !~* 'pg_|information_schema'
        ORDER BY pgc.relname, a.attnum
    )";

impl<'row> From<&Row> for ColumnDefinition {
    fn from(row: &Row) -> Self {
        // Do the column constraints first
        let mut constraints = Vec::new();
        let not_null: bool = row.get(7);
        let primary_key: bool = row.get(8);
        // TODO: Default value + unique
        constraints.push(if not_null {
            ColumnConstraint::NotNull
        } else {
            ColumnConstraint::Null
        });
        if primary_key {
            constraints.push(ColumnConstraint::PrimaryKey);
        }
        let sql_type: String = row.get(6);

        ColumnDefinition {
            name: row.get(5),
            sql_type: sql_type.into(),
            constraints,
        }
    }
}

static CTE_TABLE_CONSTRAINTS: &str = "
    WITH cte AS (
        SELECT
            tcls.oid,
            CONCAT(tc.table_schema, '.', tc.table_name) fqn,
            tc.constraint_schema,
            tc.table_name,
            tc.constraint_type,
            tc.constraint_name,
            string_agg(DISTINCT kcu.column_name, ',') as column_names,
            ccu.table_name as foreign_table_name,
            string_agg(DISTINCT ccu.column_name, ',') as foreign_column_names,
            pgcls.reloptions as pk_parameters,
            confupdtype,
            confdeltype,
            confmatchtype::text
        FROM information_schema.table_constraints as tc
        JOIN (SELECT DISTINCT column_name, constraint_name, table_name, ordinal_position
            FROM information_schema.key_column_usage
            ORDER BY ordinal_position ASC) kcu ON kcu.constraint_name = tc.constraint_name AND kcu.table_name = tc.table_name
        JOIN information_schema.constraint_column_usage as ccu on ccu.constraint_name = tc.constraint_name
        JOIN pg_catalog.pg_namespace pgn ON pgn.nspname = tc.constraint_schema
        JOIN pg_catalog.pg_namespace tn ON tn.nspname = tc.table_schema
        JOIN pg_catalog.pg_class tcls ON tcls.relname=tc.table_name AND tcls.relnamespace=tn.oid
        LEFT JOIN pg_catalog.pg_class pgcls ON pgcls.relname=tc.constraint_name AND pgcls.relnamespace = pgn.oid
        LEFT JOIN pg_catalog.pg_constraint pgcon ON pgcon.conname=tc.constraint_name AND pgcon.connamespace = pgn.oid
        WHERE
            constraint_type in ('PRIMARY KEY','FOREIGN KEY')
        GROUP BY
            tcls.oid,
            fqn,
            tc.constraint_schema,
            tc.table_name,
            tc.constraint_type,
            tc.constraint_name,
            ccu.table_name,
            pgcls.reloptions,
            confupdtype,
            confdeltype,
            confmatchtype::text
    )";
lazy_static! {
    static ref FILL_FACTOR: Regex = Regex::new("fillfactor=(\\d+)").unwrap();
}

fn parse_index_parameters(raw_parameters: Option<Vec<String>>) -> Option<Vec<IndexParameter>> {
    match raw_parameters {
        Some(parameters) => {
            // We only have one type at the moment
            if let Some(parameters) = parameters.first() {
                let ff = FILL_FACTOR.captures(&parameters[..]);
                if let Some(ff) = ff {
                    Some(vec![IndexParameter::FillFactor(ff[1].parse::<u32>().unwrap())])
                } else {
                    None
                }
            } else {
                None
            }
        }
        None => None,
    }
}

impl<'row> From<&Row> for TableConstraint {
    fn from(row: &Row) -> Self {
        let schema: String = row.get(2);
        let constraint_type: String = row.get(4);
        let constraint_name: String = row.get(5);

        let raw_column_names: String = row.get(6);
        let column_names: Vec<String> = raw_column_names.split_terminator(',').map(|s| s.into()).collect();

        match &constraint_type[..] {
            "PRIMARY KEY" => TableConstraint::Primary {
                name: constraint_name,
                columns: column_names,
                parameters: parse_index_parameters(row.get(9)),
            },
            "FOREIGN KEY" => {
                let foreign_table_name: String = row.get(7);
                let raw_foreign_column_names: String = row.get(8);
                let foreign_column_names: Vec<String> = raw_foreign_column_names
                    .split_terminator(',')
                    .map(|s| s.into())
                    .collect();
                let ev: String = row.get(12);
                let match_type = match &ev[..] {
                    "f" => Some(ForeignConstraintMatchType::Full),
                    "s" => Some(ForeignConstraintMatchType::Simple),
                    "p" => Some(ForeignConstraintMatchType::Partial),
                    _ => None,
                };

                let mut events = Vec::new();
                let update_event: i8 = row.get(10);
                match update_event as u8 as char {
                    'r' => events.push(ForeignConstraintEvent::Update(ForeignConstraintAction::Restrict)),
                    'c' => events.push(ForeignConstraintEvent::Update(ForeignConstraintAction::Cascade)),
                    'd' => events.push(ForeignConstraintEvent::Update(ForeignConstraintAction::SetDefault)),
                    'n' => events.push(ForeignConstraintEvent::Update(ForeignConstraintAction::SetNull)),
                    'a' => events.push(ForeignConstraintEvent::Update(ForeignConstraintAction::NoAction)),
                    _ => {}
                }
                let delete_event: i8 = row.get(11);
                match delete_event as u8 as char {
                    'r' => events.push(ForeignConstraintEvent::Delete(ForeignConstraintAction::Restrict)),
                    'c' => events.push(ForeignConstraintEvent::Delete(ForeignConstraintAction::Cascade)),
                    'd' => events.push(ForeignConstraintEvent::Delete(ForeignConstraintAction::SetDefault)),
                    'n' => events.push(ForeignConstraintEvent::Delete(ForeignConstraintAction::SetNull)),
                    'a' => events.push(ForeignConstraintEvent::Delete(ForeignConstraintAction::NoAction)),
                    _ => {}
                }

                TableConstraint::Foreign {
                    name: constraint_name,
                    columns: column_names,
                    ref_table: ObjectName {
                        schema: Some(schema),
                        name: foreign_table_name,
                    },
                    ref_columns: foreign_column_names,
                    match_type,
                    events: if events.is_empty() { None } else { Some(events) },
                }
            }
            unknown => panic!("Unknown constraint type: {}", unknown),
        }
    }
}

static CTE_INDEXES_94_THRU_96: &str = "
    WITH cte AS (
        SELECT
            tc.oid,
            ns.nspname AS schema_name,
            tc.relname AS table_name,
            ic.relname AS index_name,
            idx.indisunique AS is_unique,
            am.amname AS index_type,
            ARRAY(
                SELECT json_build_object(
                    'colname', pg_get_indexdef(idx.indexrelid, k + 1, TRUE),
                    'orderable', am.amcanorder,
                    'asc', CASE WHEN idx.indoption[k] & 1 = 0 THEN true ELSE false END,
                    'desc', CASE WHEN idx.indoption[k] & 1 = 1 THEN true ELSE false END,
                    'nulls_first', CASE WHEN idx.indoption[k] & 2 = 2 THEN true ELSE false END,
                    'nulls_last', CASE WHEN idx.indoption[k] & 2 = 0 THEN true ELSE false END
                )
                FROM
                    generate_subscripts(idx.indkey, 1) AS k
                ORDER BY k
            ) AS index_keys,
            ic.reloptions AS storage_parameters
        FROM pg_index AS idx
        JOIN pg_class AS ic ON ic.oid = idx.indexrelid
        JOIN pg_am AS am ON ic.relam = am.oid
        JOIN pg_namespace AS ns ON ic.relnamespace = ns.OID
        JOIN pg_class AS tc ON tc.oid = idx.indrelid
        WHERE ns.nspname !~* 'pg_|information_schema' AND idx.indisprimary = false
    )
";

// Index query >= 9.6
static CTE_INDEXES: &str = "
    WITH cte AS (
        SELECT
            tc.oid,
            ns.nspname AS schema_name,
            tc.relname AS table_name,
            ic.relname AS index_name,
            idx.indisunique AS is_unique,
            am.amname AS index_type,
            ARRAY(
                SELECT json_build_object(
                    'colname', pg_get_indexdef(idx.indexrelid, k + 1, TRUE),
                    'orderable', pg_index_column_has_property(idx.indexrelid, k + 1, 'orderable'),
                    'asc', pg_index_column_has_property(idx.indexrelid, k + 1, 'asc'),
                    'desc', pg_index_column_has_property(idx.indexrelid, k + 1, 'desc'),
                    'nulls_first', pg_index_column_has_property(idx.indexrelid, k + 1, 'nulls_first'),
                    'nulls_last', pg_index_column_has_property(idx.indexrelid, k + 1, 'nulls_last')
                )
                FROM
                    generate_subscripts(idx.indkey, 1) AS k
                ORDER BY k
            ) AS index_keys,
            ic.reloptions AS storage_parameters
        FROM pg_index AS idx
        JOIN pg_class AS ic ON ic.oid = idx.indexrelid
        JOIN pg_am AS am ON ic.relam = am.oid
        JOIN pg_namespace AS ns ON ic.relnamespace = ns.OID
        JOIN pg_class AS tc ON tc.oid = idx.indrelid
        WHERE ns.nspname !~* 'pg_|information_schema' AND idx.indisprimary = false
    )
";

impl<'row> From<&Row> for IndexDefinition {
    fn from(row: &Row) -> Self {
        let schema: String = row.get(1);
        let table: String = row.get(2);
        let name: String = row.get(3);
        let unique: bool = row.get(4);
        let index_type: String = row.get(5);
        let index_type = match &index_type[..] {
            "btree" => Some(IndexType::BTree),
            "gin" => Some(IndexType::Gin),
            "gist" => Some(IndexType::Gist),
            "hash" => Some(IndexType::Hash),
            _ => None,
        };
        let columns: Vec<serde_json::Value> = row.get(6);
        let columns = columns
            .iter()
            .map(|c| c.as_object().unwrap())
            .map(|map| IndexColumn {
                name: map["colname"].as_str().unwrap().to_owned(),
                order: if map["orderable"].as_bool().unwrap_or(false) {
                    if map["asc"].as_bool().unwrap_or(false) {
                        Some(IndexOrder::Ascending)
                    } else if map["desc"].as_bool().unwrap_or(false) {
                        Some(IndexOrder::Descending)
                    } else {
                        None
                    }
                } else {
                    None
                },
                null_position: if map["orderable"].as_bool().unwrap_or(false) {
                    if map["nulls_first"].as_bool().unwrap_or(false) {
                        Some(IndexPosition::First)
                    } else if map["nulls_last"].as_bool().unwrap_or(false) {
                        Some(IndexPosition::Last)
                    } else {
                        None
                    }
                } else {
                    None
                },
            })
            .collect();
        let storage_parameters = parse_index_parameters(row.get(7));

        IndexDefinition {
            name,
            table: ObjectName {
                schema: Some(schema),
                name: table,
            },
            columns,

            unique,
            index_type,

            storage_parameters,
        }
    }
}

impl From<String> for SqlType {
    fn from(s: String) -> Self {
        // TODO: Error handling for this
        let tokens = lexer::tokenize_body(&s).unwrap();
        SqlTypeParser::new().parse(tokens).unwrap()
    }
}
