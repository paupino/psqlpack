use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::fs::File;
use std::io::{BufReader, Read, Write};

use regex::Regex;
use slog::Logger;
use serde_json;
use zip::{ZipArchive, ZipWriter};
use zip::write::FileOptions;
use petgraph;
use postgres::rows::Row;
use lalrpop_util;

use connection::Connection;
use sql::lexer;
use sql::ast::*;
use sql::parser::{SqlTypeParser, FunctionArgumentListParser, FunctionReturnTypeParser};
use model::Project;
use errors::{PsqlpackError, PsqlpackResult, PsqlpackResultExt};
use errors::PsqlpackErrorKind::*;

macro_rules! ztry {
    ($expr:expr) => {{
        match $expr {
            Ok(_) => {},
            Err(e) => bail!(GenerationError(format!("Failed to write package: {}", e))),
        }
    }};
}

macro_rules! zip_collection {
    ($zip:ident, $package:ident, $collection:ident) => {{
        let collection_name = stringify!($collection);
        ztry!($zip.add_directory(format!("{}/", collection_name), FileOptions::default()));
        for item in &$package.$collection {
            ztry!($zip.start_file(format!("{}/{}.json", collection_name, item.name), FileOptions::default()));
            let json = match serde_json::to_string_pretty(&item) {
                Ok(j) => j,
                Err(e) => bail!(GenerationError(format!("Failed to write package: {}", e))),
            };
            ztry!($zip.write_all(json.as_bytes()));
        }
    }};
}

macro_rules! map {
    ($expr:expr) => {{
        $expr.iter().map(|row| row.into()).collect()
    }};
}

static Q_DATABASE_EXISTS: &'static str = "SELECT 1 FROM pg_database WHERE datname=$1;";

static Q_EXTENSIONS: &'static str = "SELECT extname, extversion
                                     FROM pg_extension
                                     WHERE extowner <> 10";
impl<'row> From<Row<'row>> for ExtensionDefinition {
    fn from(row: Row) -> Self {
        ExtensionDefinition { name: row.get(0) }
    }
}

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

static Q_FUNCTIONS: &'static str = "SELECT
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
                                    LEFT JOIN pg_depend ON
                                        pg_depend.objid = pg_proc.oid AND pg_depend.deptype = 'e'
                                    WHERE pg_depend.objid IS NULL AND
                                          nspname !~* 'pg_|information_schema';";

static Q_TABLES: &'static str = "SELECT
                                    pg_class.oid,
                                    nspname,
                                    relname,
                                    conname,
                                    pg_get_constraintdef(pg_constraint.oid)
                                FROM pg_class
                                JOIN pg_namespace ON
                                    pg_namespace.oid = pg_class.relnamespace
                                LEFT JOIN pg_depend ON
                                    pg_depend.objid = pg_class.oid AND pg_depend.deptype = 'e'
                                LEFT JOIN pg_constraint ON
                                    pg_constraint.conrelid = pg_class.oid
                                WHERE pg_class.relkind='r' AND
                                      pg_depend.objid IS NULL AND
                                      nspname !~* 'pg_|information_schema'";
impl<'row> From<Row<'row>> for TableDefinition {
    fn from(row: Row) -> Self {
        TableDefinition {
            name: ObjectName {
                schema: Some(row.get(1)),
                name: row.get(2),
            },
            columns: Vec::new(), // TODO
            constraints: Vec::new(),   // TODO
        }
    }
}

static Q_COLUMNS : &'static str =  "SELECT DISTINCT
                                        ns.nspname as schema_name,
                                        pgc.relname as table_name,
                                        a.attnum as num,
                                        a.attname as name,
                                        CASE WHEN a.atttypid = ANY ('{int,int8,int2}'::regtype[])
                                              AND def.adsrc = 'nextval('''
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
                                        def.adsrc as default
                                    FROM pg_attribute a
                                    INNER JOIN pg_class pgc ON pgc.oid = a.attrelid
                                    INNER JOIN pg_namespace ns ON ns.oid = pgc.relnamespace
                                    LEFT JOIN pg_index i ON pgc.oid = i.indrelid AND i.indkey[0] = a.attnum
                                    LEFT JOIN pg_attrdef def ON a.attrelid = def.adrelid AND a.attnum = def.adnum
                                    WHERE attnum > 0 AND pgc.relkind='r' AND NOT a.attisdropped AND ns.nspname !~* 'pg_|information_schema'
                                    ORDER BY pgc.relname, a.attnum";

impl<'row> From<Row<'row>> for ColumnDefinition {
    fn from(row: Row) -> Self {
        // Do the column constraints first
        let mut constraints = Vec::new();
        let not_null : bool = row.get(5);
        let primary_key : bool = row.get(6);
        // TODO: Default value + unique
        constraints.push(if not_null { ColumnConstraint::NotNull } else { ColumnConstraint::Null });
        if primary_key {
            constraints.push(ColumnConstraint::PrimaryKey);
        }
        let sql_type : String = row.get(4);

        ColumnDefinition {
            name: row.get(3),
            sql_type: sql_type.into(),
            constraints: constraints,
        }
    }
}

static Q_TABLE_CONSTRAINTS : &'static str = "SELECT  
                                    tc.constraint_schema,
                                    tc.table_name, 
                                    tc.constraint_type,
                                    tc.constraint_name, 
                                    string_agg(DISTINCT kcu.column_name, ',') as column_names, 
                                    ccu.table_name as foreign_table_name, 
                                    string_agg(DISTINCT ccu.column_name, ',') as foreign_column_names,
                                    pgc.reloptions as pk_parameters
                                FROM 
                                    information_schema.table_constraints as tc  
                                    JOIN (SELECT DISTINCT column_name, constraint_name, table_name, ordinal_position 
                                        FROM information_schema.key_column_usage 
                                        ORDER BY ordinal_position ASC) kcu ON kcu.constraint_name = tc.constraint_name AND kcu.table_name = tc.table_name
                                    JOIN information_schema.constraint_column_usage as ccu on ccu.constraint_name = tc.constraint_name
                                    JOIN pg_namespace pgn ON pgn.nspname = tc.constraint_schema
                                    LEFT JOIN pg_class pgc ON pgc.relname=tc.constraint_name AND pgc.relnamespace = pgn.oid
                                WHERE 
                                    constraint_type in ('PRIMARY KEY','FOREIGN KEY')
                                GROUP BY
                                    tc.constraint_schema,
                                    tc.table_name, 
                                    tc.constraint_type,
                                    tc.constraint_name,
                                    ccu.table_name,
                                    pgc.reloptions";
lazy_static! {
    static ref FILL_FACTOR : Regex = Regex::new("fillfactor=(\\d+)").unwrap();
}

impl<'row> From<Row<'row>> for TableConstraint {
    fn from(row: Row) -> Self {
        let schema : String = row.get(0);
        let constraint_type : String = row.get(2);
        let constraint_name : String = row.get(3);
        
        let raw_column_names : String = row.get(4);
        let column_names : Vec<String> = raw_column_names
                                            .split_terminator(',')
                                            .map(|s| s.into())
                                            .collect();

        match &constraint_type[..] {
            "PRIMARY KEY" => {
                let raw_parameters: Option<Vec<String>> = row.get(7);
                let parameters = match raw_parameters {
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
                };
                TableConstraint::Primary {
                    name: constraint_name,
                    columns: column_names,
                    parameters: parameters,
                }
            },
            "FOREIGN KEY" => {
                let foreign_table_name : String = row.get(5);
                let raw_foreign_column_names : String = row.get(6);
                let foreign_column_names : Vec<String> = raw_foreign_column_names
                                                            .split_terminator(',')
                                                            .map(|s| s.into())
                                                            .collect();

                TableConstraint::Foreign {
                    name: constraint_name,
                    columns: column_names,
                    ref_table: ObjectName { 
                        schema: Some(schema),
                        name: foreign_table_name
                    },
                    ref_columns: foreign_column_names,
                    match_type: None, //TODO
                    events: None, //TODO
                }
            },
            unknown => panic!("Unknown constraint type: {}", unknown),
        }
    }
}

impl From<String> for SqlType {
    fn from(s: String) -> Self {
        // TODO: Error handling for this
        let tokens = lexer::tokenize(&s).unwrap();
        SqlTypeParser::new().parse(tokens).unwrap()
    }
}

#[derive(Debug, PartialEq)]
pub struct Package {
    pub extensions: Vec<ExtensionDefinition>,
    pub functions: Vec<FunctionDefinition>,
    pub schemas: Vec<SchemaDefinition>,
    pub scripts: Vec<ScriptDefinition>,
    pub tables: Vec<TableDefinition>,
    pub types: Vec<TypeDefinition>,
}

impl Package {
    fn maybe_packaged_file(source_path: &Path) -> PsqlpackResult<bool> {
        File::open(&source_path)
            .chain_err(|| IOError(source_path.to_str().unwrap().into(), "Failed to open file".into()))
            .and_then(|file| {
                let mut reader = BufReader::with_capacity(4, file);
                let mut buffer = [0; 2];
                let b = reader.read(&mut buffer[..])
                    .map_err(|e| IOError(source_path.to_str().unwrap().into(), format!("Failed to read file: {}", e)))?;
                if b != 2 {
                    bail!(IOError(source_path.to_str().unwrap().into(), "Invalid file provide (< 4 bytes)".into()));
                }

                Ok(
                    buffer[0] == 0x50 && 
                    buffer[1] == 0x4B
                )
            })
    }

    pub fn from_path(log: &Logger, source_path: &Path) -> PsqlpackResult<Package> {
        let log = log.new(o!("package" => "from_path"));
        // source_path could be either a project file or a psqlpack file
        // Try and guess which type it is first
        if Self::maybe_packaged_file(source_path)? {
            Self::from_packaged_file(&log, source_path)
        } else {
            // We'll optimistically load it as a project
            let project = Project::from_project_file(&log, source_path)?;
            project.to_package(&log)
        }
    }

    pub fn from_packaged_file(log: &Logger, source_path: &Path) -> PsqlpackResult<Package> {
        let _log = log.new(o!("package" => "from_packaged_file"));
        let mut archive = File::open(&source_path)
            .chain_err(|| PackageReadError(source_path.to_path_buf()))
            .and_then(|file| {
                ZipArchive::new(file).chain_err(|| PackageUnarchiveError(source_path.to_path_buf()))
            })?;

        let mut extensions = Vec::new();
        let mut functions = Vec::new();
        let mut schemas = Vec::new();
        let mut scripts = Vec::new();
        let mut tables = Vec::new();
        let mut types = Vec::new();

        for i in 0..archive.len() {
            let file = archive.by_index(i).unwrap();
            if file.size() == 0 {
                continue;
            }
            let name = file.name().to_owned();
            if name.starts_with("extensions/") {
                extensions.push(serde_json::from_reader(file)
                    .chain_err(|| PackageInternalReadError(name))?);
            } else if name.starts_with("functions/") {
                functions.push(serde_json::from_reader(file)
                    .chain_err(|| PackageInternalReadError(name))?);
            } else if name.starts_with("schemas/") {
                schemas.push(serde_json::from_reader(file)
                    .chain_err(|| PackageInternalReadError(name))?);
            } else if name.starts_with("scripts/") {
                scripts.push(serde_json::from_reader(file)
                    .chain_err(|| PackageInternalReadError(name))?);
            } else if name.starts_with("tables/") {
                tables.push(serde_json::from_reader(file)
                    .chain_err(|| PackageInternalReadError(name))?);
            } else if name.starts_with("types/") {
                types.push(serde_json::from_reader(file)
                    .chain_err(|| PackageInternalReadError(name))?);
            }
        }

        let mut package = Package {
            extensions: extensions,
            functions: functions,
            schemas: schemas,
            scripts: scripts,
            tables: tables,
            types: types,
        };
        package.promote_primary_keys_to_table_constraints();
        Ok(package)
    }

    pub fn from_connection(log: &Logger, connection: &Connection) -> PsqlpackResult<Option<Package>> {
        let log = log.new(o!("package" => "from_connection"));

        trace!(log, "Connecting to host");
        let db_conn = connection.connect_host()?;
        trace!(
            log,
            "Checking for database `{}`",
            &connection.database()[..]
        );
        let db_result = dbtry!(db_conn.query(Q_DATABASE_EXISTS, &[&connection.database()]));
        if db_result.is_empty() {
            return Ok(None);
        }
        dbtry!(db_conn.finish());

        // We do five SQL queries to get the package details
        let db_conn = connection.connect_database()?;

        let extensions = db_conn
            .query(Q_EXTENSIONS, &[])
            .chain_err(|| PackageQueryExtensionsError)?;

        let schemas = db_conn
            .query(Q_SCHEMAS, &[&connection.database()])
            .chain_err(|| PackageQuerySchemasError)?;

        let types = db_conn
            .query(Q_ENUMS, &[])
            .chain_err(|| PackageQueryTypesError)?;

        let mut functions = Vec::new();
        for row in &db_conn
            .query(Q_FUNCTIONS, &[])
            .chain_err(|| PackageQueryFunctionsError)?
        {
            let schema_name: String = row.get(0);
            let function_name: String = row.get(1);
            let function_src: String = row.get(2);
            let raw_args: String = row.get(3);
            let lan_name: String = row.get(4);
            let raw_result: String = row.get(5);

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
            functions.push(FunctionDefinition {
                name: ObjectName {
                    schema: Some(schema_name),
                    name: function_name,
                },
                arguments: function_args,
                return_type: return_type,
                body: function_src,
                language: language,
            });
        }

        let mut tables = HashMap::new();
        for row in &db_conn.query(Q_TABLES, &[])
                           .chain_err(|| PackageQueryTablesError)? {
            let table : TableDefinition = row.into();
            tables.insert(table.name.to_string(), table);
        }

        // Get a list of columns and map them to the appropriate tables
        for row in &db_conn.query(Q_COLUMNS, &[])
                           .chain_err(|| PackageQueryColumnsError)? {
            // Get the table name and find it in our collection
            let schema : String = row.get(0);
            let table : String = row.get(1);
            let key = format!("{}.{}", schema, table);

            // Now look up the mutable key
            if let Some(definition) = tables.get_mut(&key) {
                definition.columns.push(row.into());
            }
        }

        // Get a list of table constraints
        for row in &db_conn.query(Q_TABLE_CONSTRAINTS, &[])
                            .chain_err(|| PackageQueryTableConstraintsError)? {
            // Get the table name and find it in our collection
            let schema : String = row.get(0);
            let table : String = row.get(1);
            let key = format!("{}.{}", schema, table);

            // Now look up the mutable key
            if let Some(definition) = tables.get_mut(&key) {
                definition.constraints.push(row.into());
            }            
        }

        // Close the connection
        dbtry!(db_conn.finish());

        // Get the package
        let mut package = Package {
            extensions: map!(extensions),
            functions: functions,   // functions,
            schemas: map!(schemas), // schemas,
            scripts: Vec::new(),    // Scripts can't be known from a connection
            tables: tables.into_iter().map(|(_,b)| b).collect(),   // tables,
            types: map!(types),     // types,
        };
        package.promote_primary_keys_to_table_constraints();

        Ok(Some(package))
    }

    pub fn write_to(&self, destination: &Path) -> PsqlpackResult<()> {
        File::create(&destination)
            .chain_err(|| GenerationError("Failed to write package".to_owned()))
            .and_then(|output_file| {
                let mut zip = ZipWriter::new(output_file);

                zip_collection!(zip, self, extensions);
                zip_collection!(zip, self, functions);
                zip_collection!(zip, self, schemas);
                zip_collection!(zip, self, scripts);
                zip_collection!(zip, self, tables);
                zip_collection!(zip, self, types);

                ztry!(zip.finish());

                Ok(())
            })
    }

    pub fn new() -> Self {
        Package {
            extensions: Vec::new(),
            functions: Vec::new(),
            schemas: Vec::new(),
            scripts: Vec::new(),
            tables: Vec::new(),
            types: Vec::new(),
        }
    }

    pub fn push_extension(&mut self, extension: ExtensionDefinition) {
        self.extensions.push(extension);
    }

    pub fn push_function(&mut self, function: FunctionDefinition) {
        self.functions.push(function);
    }

    pub fn push_script(&mut self, script: ScriptDefinition) {
        self.scripts.push(script);
    }

    pub fn push_schema(&mut self, schema: SchemaDefinition) {
        self.schemas.push(schema);
    }

    pub fn push_table(&mut self, table: TableDefinition) {
        self.tables.push(table);
    }

    pub fn push_type(&mut self, def: TypeDefinition) {
        self.types.push(def);
    }

    pub fn set_defaults(&mut self, project: &Project) {
        // Make sure the public schema exists
        let mut has_public = false;
        for schema in &mut self.schemas {
            if project
                .default_schema
                .eq_ignore_ascii_case(&schema.name[..])
            {
                has_public = true;
                break;
            }
        }
        if !has_public {
            self.schemas.push(SchemaDefinition {
                name: project.default_schema.to_owned(),
            });
        }

        fn ensure_not_null_column(column: &mut ColumnDefinition) {
            // Remove null for primary keys
            let pos = column.constraints.iter().position(|c| c.eq(&ColumnConstraint::Null));
            if let Some(pos) = pos {
                column.constraints.remove(pos);
            }

            // Add not null for primary keys
            let pos = column.constraints.iter().position(|c| c.eq(&ColumnConstraint::NotNull));
            if pos.is_none() {
                column.constraints.push(ColumnConstraint::NotNull);
            }
        }

        // Set default schema's as well as marking primary key columns as not null
        for table in &mut self.tables {
            if table.name.schema.is_none() {
                table.name.schema = Some(project.default_schema.clone());
            }
            
            for constraint in table.constraints.iter_mut() {
                match *constraint {
                    TableConstraint::Primary { ref columns, .. } => {
                        for column in columns {
                            let item = table.columns.iter_mut().find(|item| item.name.eq(column));
                            if let Some(item) = item {
                                ensure_not_null_column(item);
                            }
                        }
                    }
                    TableConstraint::Foreign { ref mut ref_table, .. } => {
                        if ref_table.schema.is_none() {
                            ref_table.schema = Some(project.default_schema.clone());
                        }
                    }
                }
            }

            // Primary keys may also be specified against the column directly. We promote these to table constraints.`
            for column in table.columns.iter_mut() {
                let pk = column.constraints.iter().position(|c| c.eq(&ColumnConstraint::PrimaryKey));
                if pk.is_some() {
                    // Make sure it is not null
                    ensure_not_null_column(column);
                }
            }
        }

        // We also do the promotion here
        self.promote_primary_keys_to_table_constraints();
    }

    fn promote_primary_keys_to_table_constraints(&mut self) {
        // Set default schema's as well as marking primary key columns as not null
        for table in &mut self.tables {
            // Primary keys may also be specified against the column directly. We promote these to table constraints.`
            for column in table.columns.iter_mut() {
                let pk_pos = column.constraints.iter().position(|c| c.eq(&ColumnConstraint::PrimaryKey));
                if let Some(pk_pos) = pk_pos {
                    // Remove the PK constraint
                    column.constraints.remove(pk_pos);
                    
                    // Add a table constraint if it doesn't exist
                    let found = table.constraints.iter().position(|c| match c {
                        TableConstraint::Primary { .. } => true,
                        _ => false,
                    });
                    if found.is_none() {
                        let name = format!("{}_pkey", table.name.name);
                        table.constraints.push(TableConstraint::Primary {
                            name: name,
                            columns: vec![column.name.to_owned()],
                            parameters: None,
                        });
                    }
                }
            }
        }
    }

    pub fn generate_dependency_graph<'out>(&'out self, log: &Logger) -> PsqlpackResult<Vec<Node<'out>>> {
        let log = log.new(o!("graph" => "generate"));

        let mut graph = Graph::new();

        // Go through and add each object and add it to the graph
        // Extensions, schemas and types are always implied
        trace!(log, "Scanning table dependencies");
        for table in &self.tables {
            let log = log.new(o!("table" => table.name.to_string()));
            table.graph(&log, &mut graph, None);
        }
        trace!(log, "Scanning table constraints");
        for table in &self.tables {
            let log = log.new(o!("table" => table.name.to_string()));
            let table_node = Node::Table(table);
            trace!(log, "Scanning constraints");
            for constraint in &table.constraints {
                let constraint_node = constraint.graph(&log, &mut graph, Some(&table_node));
                graph.add_edge(table_node, constraint_node, ());
            }
        }

        trace!(log, "Scanning function dependencies");
        for function in &self.functions {
            let log = log.new(o!("function" => function.name.to_string()));
            function.graph(&log, &mut graph, None);
        }

        // Then generate the order
        trace!(log, "Sorting graph");
        match petgraph::algo::toposort(&graph, None) {
            Err(_) => bail!(GenerationError("Circular reference detected".to_owned())),
            Ok(index_order) => {
                let log = log.new(o!("order" => "sorted"));
                for node in &index_order {
                    trace!(log, ""; "node" => node.to_string());
                }
                Ok(index_order)
            }
        }
    }

    pub fn validate(&self) -> PsqlpackResult<()> {
        // 1. Validate schema existance
        let schemata = self.schemas
            .iter()
            .map(|schema| &schema.name[..])
            .collect::<Vec<_>>();
        let names = self.tables
            .iter()
            .map(|t| &t.name)
            .chain(self.functions.iter().map(|f| &f.name))
            .collect::<Vec<_>>();
        let mut errors = names
            .iter()
            .filter(|o| if let Some(ref s) = o.schema {
                !schemata.contains(&&s[..])
            } else {
                false
            })
            .map(|o| {
                ValidationKind::SchemaMissing {
                    schema: o.schema.clone().unwrap(),
                    object: o.name.to_owned(),
                }
            })
            .collect::<Vec<_>>();

        // 2. Validate custom type are known
        let custom_types = self.types.iter().map(|ty| &ty.name[..]).collect::<Vec<_>>();
        errors.extend(self.tables.iter().flat_map(|t| {
            t.columns
                .iter()
                .filter_map(|c| match c.sql_type {
                    SqlType::Custom(ref name, _) => if !custom_types.contains(&&name[..]) {
                        Some(ValidationKind::UnknownType {
                            ty: name.to_owned(),
                            table: t.name.to_string(),
                        })
                    } else {
                        None
                    },
                    _ => None,
                })
                .collect::<Vec<_>>()
        }));

        // 3. Validate constraints map to known tables
        let foreign_keys = self.tables
            .iter()
            .flat_map(|t| t.constraints.clone())
            .filter_map(|c| match c {
                TableConstraint::Foreign {
                    name,
                    columns,
                    ref_table,
                    ref_columns,
                    ..
                } => Some((name, columns, ref_table, ref_columns)),
                _ => None,
            })
            .collect::<Vec<_>>();

        // Four types here:
        // i. Reference table doesn't exist
        errors.extend(
            foreign_keys
                .iter()
                .filter(|&&(_, _, ref table, _)| {
                    !self.tables.iter().any(|t| t.name.eq(table))
                })
                .map(|&(ref name, _, ref table, _)| {
                    ValidationKind::TableConstraintInvalidReferenceTable {
                        constraint: name.to_owned(),
                        table: table.to_string(),
                    }
                }),
        );
        // ii. Reference table exists, but the reference column doesn't.
        errors.extend(
            foreign_keys
                .iter()
                .filter(|&&(_, _, ref table, ref columns)| {
                    let table = self.tables.iter().find(|t| t.name.eq(table));
                    match table {
                        Some(t) => !columns
                            .iter()
                            .all(|rc| t.columns.iter().any(|c| c.name.eq(rc))),
                        None => false,
                    }
                })
                .map(|&(ref name, _, ref table, ref columns)| {
                    ValidationKind::TableConstraintInvalidReferenceColumns {
                        constraint: name.to_owned(),
                        table: table.to_string(),
                        columns: columns.clone(),
                    }
                }),
        );
        // iii. Source column doesn't exist
        errors.extend(
            foreign_keys
                .iter()
                .filter(|&&(ref constraint, ref columns, _, _)| {
                    let table = self.tables
                        .iter()
                        .find(|t| t.constraints.iter().any(|c| c.name() == constraint));
                    match table {
                        Some(t) => !columns
                            .iter()
                            .all(|rc| t.columns.iter().any(|c| c.name.eq(rc))),
                        None => false,
                    }
                })
                .map(|&(ref name, ref columns, _, _)| {
                    ValidationKind::TableConstraintInvalidSourceColumns {
                        constraint: name.to_owned(),
                        columns: columns.clone(),
                    }
                }),
        );
        // iv. (Future) Source column match type is not compatible with reference column type

        // If there are no errors then we're "ok"
        if errors.is_empty() {
            Ok(())
        } else {
            bail!(ValidationError(errors))
        }
    }
}

#[derive(Debug)]
pub enum ValidationKind {
    TableConstraintInvalidReferenceTable { constraint: String, table: String },
    TableConstraintInvalidReferenceColumns {
        constraint: String,
        table: String,
        columns: Vec<String>,
    },
    TableConstraintInvalidSourceColumns {
        constraint: String,
        columns: Vec<String>,
    },
    SchemaMissing { schema: String, object: String },
    UnknownType { ty: String, table: String },
}

impl fmt::Display for ValidationKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ValidationKind::TableConstraintInvalidReferenceTable {
                ref constraint,
                ref table,
            } => write!(
                f,
                "Foreign Key constraint `{}` uses unknown reference table `{}`",
                constraint,
                table
            ),
            ValidationKind::TableConstraintInvalidReferenceColumns {
                ref constraint,
                ref table,
                ref columns,
            } => write!(
                f,
                "Foreign Key constraint `{}` uses unknown reference column(s) on table `{}` (`{}`)",
                constraint,
                table,
                columns.join("`, `")
            ),
            ValidationKind::TableConstraintInvalidSourceColumns {
                ref constraint,
                ref columns,
            } => write!(
                f,
                "Foreign Key constraint `{}` uses unknown source column(s) (`{}`)",
                constraint,
                columns.join("`, `")
            ),
            ValidationKind::SchemaMissing {
                ref schema,
                ref object,
            } => write!(f, "Schema `{}` missing for object `{}`", schema, object),
            ValidationKind::UnknownType { ref ty, ref table } => write!(f, "Unknown type `{}` used on table `{}`", ty, table),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Node<'def> {
    Table(&'def TableDefinition),
    Column(&'def TableDefinition, &'def ColumnDefinition),
    Constraint(&'def TableDefinition, &'def TableConstraint),
    Function(&'def FunctionDefinition),
}

impl<'def> fmt::Display for Node<'def> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Node::Table(table) => write!(f, "Table:      {}", table.name.to_string()),
            Node::Column(table, column) => write!(f, "Column:     {}.{}", table.name.to_string(), column.name),
            Node::Constraint(table, constraint) => write!(
                f,
                "Constraint: {}.{}",
                table.name.to_string(),
                constraint.name()
            ),
            Node::Function(function) => write!(f, "Function:   {}", function.name.to_string()),
        }
    }
}

type Graph<'graph> = petgraph::graphmap::GraphMap<Node<'graph>, (), petgraph::Directed>;

trait Graphable {
    fn graph<'graph, 'def: 'graph>(
        &'def self,
        log: &Logger,
        graph: &mut Graph<'graph>,
        parent: Option<&Node<'graph>>,
    ) -> Node<'graph>;
}

impl Graphable for TableDefinition {
    fn graph<'graph, 'def: 'graph>(
        &'def self,
        log: &Logger,
        graph: &mut Graph<'graph>,
        _: Option<&Node<'graph>>,
    ) -> Node<'graph> {
        // Table is dependent on a schema, so add the edge
        // It will not have a parent - the schema is embedded in the name
        trace!(log, "Adding");
        let table_node = graph.add_node(Node::Table(self));

        trace!(log, "Scanning columns");
        for column in &self.columns {
            let log = log.new(o!("column" => column.name.to_string()));
            let column_node = column.graph(&log, graph, Some(&table_node));
            graph.add_edge(table_node, column_node, ());
        }

        table_node
    }
}

impl Graphable for ColumnDefinition {
    fn graph<'graph, 'def: 'graph>(
        &'def self,
        log: &Logger,
        graph: &mut Graph<'graph>,
        parent: Option<&Node<'graph>>,
    ) -> Node<'graph> {
        // Column does have a parent - namely the table
        let table = match *parent.unwrap() {
            Node::Table(table) => table,
            _ => panic!("Non table parent for column."),
        };
        trace!(log, "Adding");
        graph.add_node(Node::Column(table, self))
    }
}

impl Graphable for FunctionDefinition {
    fn graph<'graph, 'def: 'graph>(
        &'def self,
        log: &Logger,
        graph: &mut Graph<'graph>,
        _: Option<&Node<'graph>>,
    ) -> Node<'graph> {
        // It will not have a parent - the schema is embedded in the name
        trace!(log, "Adding");
        graph.add_node(Node::Function(self))
    }
}

impl Graphable for TableConstraint {
    fn graph<'graph, 'def: 'graph>(
        &'def self,
        log: &Logger,
        graph: &mut Graph<'graph>,
        parent: Option<&Node<'graph>>,
    ) -> Node<'graph> {
        // We currently have two types of table constraints: Primary and Foreign
        // Primary is easy with a direct dependency to the column
        // Foreign requires a weighted dependency
        // This does have a parent - namely the table
        let table = match *parent.unwrap() {
            Node::Table(table) => table,
            _ => panic!("Non table parent for column."),
        };
        match *self {
            TableConstraint::Primary {
                ref name,
                ref columns,
                ..
            } => {
                let log = log.new(o!("primary constraint" => name.to_owned()));
                // Primary relies on the columns existing (of course)
                trace!(log, "Adding");
                let constraint = graph.add_node(Node::Constraint(table, self));
                for column_name in columns {
                    trace!(log, "Adding edge to column"; "column" => &column_name);
                    let column = table
                        .columns
                        .iter()
                        .find(|x| &x.name == column_name)
                        .unwrap();
                    graph.add_edge(Node::Column(table, column), constraint, ());
                }
                constraint
            }
            TableConstraint::Foreign {
                ref name,
                ref columns,
                ref ref_table,
                ref ref_columns,
                ..
            } => {
                let log = log.new(o!("foreign constraint" => name.to_owned()));
                // Foreign has two types of edges
                trace!(log, "Adding");
                let constraint = graph.add_node(Node::Constraint(table, self));
                // Add edges to the columns in this table.
                for column_name in columns {
                    trace!(log, "Adding edge to column"; "column" => &column_name);
                    let column = table
                        .columns
                        .iter()
                        .find(|x| &x.name == column_name)
                        .unwrap();
                    graph.add_edge(Node::Column(table, column), constraint, ());
                }
                // Find the details of the referenced table.
                let table_named = |node: &Node| match *node {
                    Node::Table(table) => &table.name == ref_table,
                    _ => false,
                };
                let table_def = match graph.nodes().find(table_named) {
                    Some(Node::Table(table_def)) => table_def,
                    _ => panic!("Non table node found"),
                };
                // Add edges to the referenced columns.
                for ref_column_name in ref_columns {
                    trace!(log, "Adding edge to refrenced column";
                                "table" => ref_table.to_string(),
                                "column" => &ref_column_name);

                    let ref_column = table_def
                        .columns
                        .iter()
                        .find(|x| &x.name == ref_column_name)
                        .unwrap();
                    graph.add_edge(Node::Column(table_def, ref_column), constraint, ());

                    // If required, add an edge to any primary keys.
                    for primary in &table_def.constraints {
                        if let TableConstraint::Primary { ref columns, .. } = *primary {
                            if columns.contains(ref_column_name) {
                                graph.add_edge(Node::Constraint(table_def, primary), constraint, ());
                            }
                        }
                    }
                }
                constraint
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use errors::PsqlpackError;
    use errors::PsqlpackErrorKind::*;
    use model::*;
    use sql::{ast, lexer};
    use sql::parser::StatementListParser;

    use slog::{Discard, Drain, Logger};
    use spectral::prelude::*;

    fn package_sql(sql: &str) -> Package {
        let tokens = match lexer::tokenize(sql) {
            Ok(t) => t,
            Err(e) => panic!("Syntax error: {}", e.line),
        };
        let mut package = Package::new();
        match StatementListParser::new().parse(tokens) {
            Ok(statement_list) => for statement in statement_list {
                match statement {
                    ast::Statement::Extension(_) => panic!("Extension statement found"),
                    ast::Statement::Function(function_definition) => package.push_function(function_definition),
                    ast::Statement::Schema(schema_definition) => package.push_schema(schema_definition),
                    ast::Statement::Table(table_definition) => package.push_table(table_definition),
                    ast::Statement::Type(type_definition) => package.push_type(type_definition),
                }
            },
            Err(err) => panic!("Failed to parse sql: {:?}", err),
        }
        package
    }

    fn empty_logger() -> Logger {
        Logger::root(Discard.fuse(), o!())
    }

    macro_rules! assert_table {
        ($graph:ident,$index:expr,$name:expr) => {
            match $graph[$index] {
                Node::Table(table) => {
                    assert_that!(table.name.to_string()).is_equal_to($name.to_owned());
                },
                _ => panic!("Expected a table at index {}", $index)
            }
        };
    }

    macro_rules! assert_column {
        ($graph:ident,$index:expr,$table_name:expr,$column_name:expr) => {
            match $graph[$index] {
                Node::Column(table, column) => {
                    assert_that!(table.name.to_string()).is_equal_to($table_name.to_owned());
                    assert_that!(column.name.to_string()).is_equal_to($column_name.to_owned());
                },
                _ => panic!("Expected a column at index {}", $index)
            }
        };
    }

    macro_rules! assert_fk_constraint {
        ($graph:ident,$index:expr,$table_name:expr,$constraint_name:expr) => {
            match $graph[$index] {
                Node::Constraint(table, constraint) => {
                    assert_that!(table.name.to_string()).is_equal_to($table_name.to_owned());
                    match *constraint {
                        ast::TableConstraint::Foreign { ref name, .. } => {
                            assert_that!(name.to_string()).is_equal_to($constraint_name.to_owned());
                        },
                        _ => panic!("Expected a foreign key constraint at index {}", $index)
                    }
                },
                _ => panic!("Expected a constraint at index {}", $index)
            }
        };
    }

    #[test]
    fn it_sets_defaults() {
        let mut package = package_sql("CREATE TABLE hello_world(id int);");
        let project = Project::default();

        // Pre-condition checks
        {
            assert_that!(package.schemas).is_empty();
            assert_that!(package.tables).has_length(1);
            let table = &package.tables[0];
            assert_that!(table.name.schema).is_none();
            assert_that!(table.name.name).is_equal_to("hello_world".to_owned());
        }

        // Set the defaults and assert again
        package.set_defaults(&project);
        assert_that!(package.schemas).has_length(1);
        assert_that!(package.tables).has_length(1);
        let schema = &package.schemas[0];
        assert_that!(schema.name).is_equal_to("public".to_owned());
        let table = &package.tables[0];
        assert_that!(table.name.schema)
            .is_some()
            .is_equal_to("public".to_owned());
        assert_that!(table.name.name).is_equal_to("hello_world".to_owned());
    }

    #[test]
    fn it_generates_a_simple_ordering() {
        let package = package_sql(
            "CREATE TABLE my.parents(id int);
             CREATE SCHEMA my;",
        );
        let logger = empty_logger();
        let graph = package.generate_dependency_graph(&logger);

        // Make sure we generated two nodes.
        // We don't generate schema's so it's just going to be table/column
        assert_that!(graph).is_ok().has_length(2);
        let graph = graph.unwrap();
        assert_table!(graph, 0, "my.parents");
        assert_column!(graph, 1, "my.parents", "id");
    }

    #[test]
    fn it_generates_a_complex_ordering() {
        let package = package_sql(
            "CREATE TABLE my.child(id int, parent_id int,
               CONSTRAINT fk_parent_child FOREIGN KEY (parent_id)
               REFERENCES my.parent(id)
               MATCH SIMPLE ON UPDATE NO ACTION ON DELETE NO ACTION);
               CREATE TABLE my.parent(id int);",
        );
        let logger = empty_logger();
        let graph = package.generate_dependency_graph(&logger);

        // Make sure we generated enough nodes (two tables + three columns + one constraint).
        assert_that!(graph).is_ok().has_length(6);
        let graph = graph.unwrap();
        assert_table!(graph, 0, "my.parent");
        assert_column!(graph, 1, "my.parent", "id");
        assert_table!(graph, 2, "my.child");
        assert_column!(graph, 3, "my.child", "id");
        assert_column!(graph, 4, "my.child", "parent_id");
        assert_fk_constraint!(graph, 5, "my.child", "fk_parent_child");
    }

    #[test]
    fn it_validates_missing_schema_references() {
        let mut package = package_sql("CREATE TABLE my.items(id int);");
        let result = package.validate();

        // `my` schema is missing
        assert_that!(result).is_err();
        let validation_errors = match result.err().unwrap() {
            PsqlpackError(ValidationError(errors), _) => errors,
            unexpected => panic!("Expected validation error however saw {:?}", unexpected),
        };
        assert_that!(validation_errors).has_length(1);
        match validation_errors[0] {
            ValidationKind::SchemaMissing {
                ref schema,
                ref object,
            } => {
                assert_that!(*schema).is_equal_to("my".to_owned());
                assert_that!(*object).is_equal_to("items".to_owned());
            }
            ref unexpected => panic!("Unexpected validation type: {:?}", unexpected),
        }

        // Add the schema and try again
        package.schemas.push(ast::SchemaDefinition {
            name: "my".to_owned(),
        });
        assert_that!(package.validate()).is_ok();
    }

    #[test]
    fn it_validates_unknown_types() {
        let mut package = package_sql(
            "CREATE SCHEMA my;
             CREATE TABLE my.items(id mytype);",
        );
        let result = package.validate();

        // `mytype` is missing
        assert_that!(result).is_err();
        let validation_errors = match result.err().unwrap() {
            PsqlpackError(ValidationError(errors), _) => errors,
            unexpected => panic!("Expected validation error however saw {:?}", unexpected),
        };
        assert_that!(validation_errors).has_length(1);
        match validation_errors[0] {
            ValidationKind::UnknownType { ref ty, ref table } => {
                assert_that!(*ty).is_equal_to("mytype".to_owned());
                assert_that!(*table).is_equal_to("my.items".to_owned());
            }
            ref unexpected => panic!("Unexpected validation type: {:?}", unexpected),
        }

        // Add the type and try again
        package.types.push(ast::TypeDefinition {
            name: "mytype".to_owned(),
            kind: ast::TypeDefinitionKind::Enum(Vec::new()),
        });
        assert_that!(package.validate()).is_ok();
    }

    #[test]
    fn it_validates_missing_reference_table_in_constraint() {
        let mut package = package_sql(
            "CREATE SCHEMA my;
             CREATE TABLE my.child(id int, parent_id int,
               CONSTRAINT fk_parent_child FOREIGN KEY (parent_id)
               REFERENCES my.parent(id)
               MATCH SIMPLE ON UPDATE NO ACTION ON DELETE NO ACTION);",
        );
        let result = package.validate();

        // `my.parent` does not exist
        assert_that!(result).is_err();
        let validation_errors = match result.err().unwrap() {
            PsqlpackError(ValidationError(errors), _) => errors,
            unexpected => panic!("Expected validation error however saw {:?}", unexpected),
        };
        assert_that!(validation_errors).has_length(1);
        match validation_errors[0] {
            ValidationKind::TableConstraintInvalidReferenceTable {
                ref constraint,
                ref table,
            } => {
                assert_that!(*constraint).is_equal_to("fk_parent_child".to_owned());
                assert_that!(*table).is_equal_to("my.parent".to_owned());
            }
            ref unexpected => panic!("Unexpected validation type: {:?}", unexpected),
        }

        // Add the table and try again
        package.tables.push(ast::TableDefinition {
            name: ast::ObjectName {
                schema: Some("my".to_owned()),
                name: "parent".to_owned(),
            },
            columns: vec![
                ast::ColumnDefinition {
                    name: "id".to_owned(),
                    sql_type: ast::SqlType::Simple(ast::SimpleSqlType::Serial),
                    constraints: Vec::new(),
                },
            ],
            constraints: Vec::new(),
        });
        assert_that!(package.validate()).is_ok();
    }

    #[test]
    fn it_validates_missing_reference_column_in_constraint() {
        let mut package = package_sql(
            "CREATE SCHEMA my;
             CREATE TABLE my.child(id int, parent_id int,
               CONSTRAINT fk_parent_child FOREIGN KEY (parent_id)
               REFERENCES my.parent(parent_id)
               MATCH SIMPLE ON UPDATE NO ACTION ON DELETE NO ACTION);
               CREATE TABLE my.parent(id int);",
        );
        let result = package.validate();

        // Column `parent_id` is invalid
        assert_that!(result).is_err();
        let validation_errors = match result.err().unwrap() {
            PsqlpackError(ValidationError(errors), _) => errors,
            unexpected => panic!("Expected validation error however saw {:?}", unexpected),
        };
        assert_that!(validation_errors).has_length(1);
        match validation_errors[0] {
            ValidationKind::TableConstraintInvalidReferenceColumns {
                ref constraint,
                ref table,
                ref columns,
            } => {
                assert_that!(*constraint).is_equal_to("fk_parent_child".to_owned());
                assert_that!(*table).is_equal_to("my.parent".to_owned());
                assert_that!(*columns).has_length(1);
                assert_that!(columns[0]).is_equal_to("parent_id".to_owned());
            }
            ref unexpected => panic!("Unexpected validation type: {:?}", unexpected),
        }

        // Add the column and try again
        {
            let parent = package
                .tables
                .iter_mut()
                .find(|t| t.name.name.eq("parent"))
                .unwrap();
            parent.columns.push(ast::ColumnDefinition {
                name: "parent_id".to_owned(),
                sql_type: ast::SqlType::Simple(ast::SimpleSqlType::Integer),
                constraints: Vec::new(),
            });
        }
        assert_that!(package.validate()).is_ok();
    }

    #[test]
    fn it_validates_missing_source_column_in_constraint() {
        let mut package = package_sql(
            "CREATE SCHEMA my;
             CREATE TABLE my.child(id int, parent_id int,
               CONSTRAINT fk_parent_child FOREIGN KEY (par_id)
               REFERENCES my.parent(id)
               MATCH SIMPLE ON UPDATE NO ACTION ON DELETE NO ACTION);
               CREATE TABLE my.parent(id int);",
        );
        let result = package.validate();

        // Column `par_id` is invalid
        assert_that!(result).is_err();
        let validation_errors = match result.err().unwrap() {
            PsqlpackError(ValidationError(errors), _) => errors,
            unexpected => panic!("Expected validation error however saw {:?}", unexpected),
        };
        assert_that!(validation_errors).has_length(1);
        match validation_errors[0] {
            ValidationKind::TableConstraintInvalidSourceColumns {
                ref constraint,
                ref columns,
            } => {
                assert_that!(*constraint).is_equal_to("fk_parent_child".to_owned());
                assert_that!(*columns).has_length(1);
                assert_that!(columns[0]).is_equal_to("par_id".to_owned());
            }
            ref unexpected => panic!("Unexpected validation type: {:?}", unexpected),
        }

        // Add the column and try again
        {
            let child = package
                .tables
                .iter_mut()
                .find(|t| t.name.name.eq("child"))
                .unwrap();
            child.columns.push(ast::ColumnDefinition {
                name: "par_id".to_owned(),
                sql_type: ast::SqlType::Simple(ast::SimpleSqlType::Integer),
                constraints: Vec::new(),
            });
        }
        assert_that!(package.validate()).is_ok();
    }
}
