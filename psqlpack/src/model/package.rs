use std::fmt;
use std::path::Path;
use std::fs::File;
use std::io::Write;

use slog::Logger;
use serde_json;
use zip::{ZipArchive, ZipWriter};
use zip::write::FileOptions;
use petgraph;
use postgres::rows::Row;
use lalrpop_util;

use connection::Connection;
use sql::{lexer, parser};
use sql::ast::*;
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

static Q_EXTENSIONS : &'static str = "SELECT extname, extversion FROM pg_extension WHERE extowner <> 10";
impl<'row> From<Row<'row>> for ExtensionDefinition {
    fn from(row: Row) -> Self {
        ExtensionDefinition {
            name: row.get(0)
        }
    }
}

static Q_SCHEMAS : &'static str = "SELECT schema_name FROM information_schema.schemata
                                   WHERE catalog_name = $1 AND schema_owner <> 'postgres'";
impl<'row> From<Row<'row>> for SchemaDefinition {
    fn from(row: Row) -> Self {
        SchemaDefinition {
            name: row.get(0)
        }
    }
}

static Q_ENUMS : &'static str = "SELECT typname, array_agg(enumlabel)
                                 FROM pg_catalog.pg_type
                                 INNER JOIN pg_catalog.pg_namespace ON pg_namespace.oid=typnamespace
                                 INNER JOIN (
                                     SELECT enumtypid, enumlabel
                                     FROM pg_catalog.pg_enum
                                     ORDER BY enumtypid, enumsortorder
                                 ) labels ON labels.enumtypid=pg_type.oid
                                 WHERE typcategory IN ('E') AND nspname='public' AND substr(typname, 1, 1) <> '_'
                                 GROUP BY typname";
impl<'row> From<Row<'row>> for TypeDefinition {
    fn from(row: Row) -> Self {
        TypeDefinition {
            name: row.get(0),
            kind: TypeDefinitionKind::Enum(row.get(1))
        }
    }
}

static Q_FUNCTIONS : &'static str = "SELECT nspname, proname, prosrc, pg_get_function_arguments(pg_proc.oid), lanname, pg_get_function_result(pg_proc.oid)
                                     FROM pg_proc
                                     JOIN pg_namespace ON pg_namespace.oid = pg_proc.pronamespace
                                     JOIN pg_language ON pg_language.oid = pg_proc.prolang
                                     LEFT JOIN pg_depend ON pg_depend.objid = pg_proc.oid AND pg_depend.deptype = 'e'
                                     WHERE pg_depend.objid IS NULL AND nspname NOT IN ('pg_catalog', 'information_schema');";

static Q_TABLES : &'static str = "SELECT pg_class.oid, nspname, relname, conname, pg_get_constraintdef(pg_constraint.oid)
                                  FROM pg_class
                                  JOIN pg_namespace ON pg_namespace.oid = pg_class.relnamespace
                                  LEFT JOIN pg_depend ON pg_depend.objid = pg_class.oid AND pg_depend.deptype = 'e'
                                  LEFT JOIN pg_constraint ON pg_constraint.conrelid = pg_class.oid
                                  WHERE pg_class.relkind='r' AND pg_depend.objid IS NULL AND nspname NOT IN ('pg_catalog', 'information_schema')";
impl<'row> From<Row<'row>> for TableDefinition {
    fn from(row: Row) -> Self {
        TableDefinition {
            name: ObjectName {
                schema: Some(row.get(1)),
                name: row.get(2),
            },
            columns: Vec::new(), // TODO
            constraints: None, // TODO
        }
    }
}

macro_rules! map {
    ($expr:expr) => {{
        $expr.iter().map(|row| row.into()).collect()
    }};
}

/*static Q_COLUMNS : &'static str = "SELECT attrelid, attname, format_type(atttypid, atttypmod), attnotnull
                                   FROM pg_attribute
                                   WHERE attnum > 0 AND attrelid IN ({})";*/

pub struct Package {
    pub extensions: Vec<ExtensionDefinition>,
    pub functions: Vec<FunctionDefinition>,
    pub schemas: Vec<SchemaDefinition>,
    pub scripts: Vec<ScriptDefinition>,
    pub tables: Vec<TableDefinition>,
    pub types: Vec<TypeDefinition>,
}

impl Package {
    pub fn from_path(source_path: &Path) -> PsqlpackResult<Package> {
        let mut archive =
            File::open(&source_path)
            .chain_err(|| PackageReadError(source_path.to_path_buf()))
            .and_then(|file| {
                ZipArchive::new(file)
                .chain_err(|| PackageUnarchiveError(source_path.to_path_buf()))
            })?;

        let mut extensions = Vec::new();
        let mut functions = Vec::new();
        let mut schemas = Vec::new();
        let mut scripts = Vec::new();
        let mut tables = Vec::new();
        let mut types = Vec::new();

        for i in 0..archive.len()
        {
            let file = archive.by_index(i).unwrap();
            if file.size() == 0 {
                continue;
            }
            let name = file.name().to_owned();
            if name.starts_with("extensions/") {
                extensions.push(
                    serde_json::from_reader(file)
                    .chain_err(|| PackageInternalReadError(name))?);
            } else if name.starts_with("functions/") {
                functions.push(
                    serde_json::from_reader(file)
                    .chain_err(||PackageInternalReadError(name))?);
            } else if name.starts_with("schemas/") {
                schemas.push(
                    serde_json::from_reader(file)
                    .chain_err(||PackageInternalReadError(name))?);
            } else if name.starts_with("scripts/") {
                scripts.push(
                    serde_json::from_reader(file)
                    .chain_err(|| PackageInternalReadError(name))?);
            } else if name.starts_with("tables/") {
                tables.push(
                    serde_json::from_reader(file)
                    .chain_err(|| PackageInternalReadError(name))?);
            } else if name.starts_with("types/") {
                types.push(
                    serde_json::from_reader(file)
                    .chain_err(|| PackageInternalReadError(name))?);
            }
        }

        Ok(Package {
            extensions: extensions,
            functions: functions,
            schemas: schemas,
            scripts: scripts,
            tables: tables,
            types: types,
        })
    }

    pub fn from_connection(connection: &Connection) -> PsqlpackResult<Package> {
        // We do five SQL queries to get the package details
        let db_conn = connection.connect_database()?;

        let extensions =
            db_conn.query(Q_EXTENSIONS, &[])
            .chain_err(|| PackageQueryExtensionsError)?;

        let schemas =
            db_conn.query(Q_SCHEMAS, &[&connection.database()])
            .chain_err(|| PackageQuerySchemasError)?;

        let types =
            db_conn.query(Q_ENUMS, &[])
            .chain_err(|| PackageQueryTypesError)?;

        let mut functions = Vec::new();
        for row in &db_conn.query(Q_FUNCTIONS, &[]).chain_err(|| PackageQueryFunctionsError)? {
            let schema_name : String = row.get(0);
            let function_name : String = row.get(1);
            let function_src : String = row.get(2);
            let raw_args : String = row.get(3);
            let lan_name : String = row.get(4);
            let raw_result : String = row.get(5);

            // Parse some of the results
            let language = match &lan_name[..] {
                "internal" => FunctionLanguage::Internal,
                "c" => FunctionLanguage::C,
                "sql" => FunctionLanguage::SQL,
                _ => FunctionLanguage::PostgreSQL,
            };

            fn lexical(err: lexer::LexicalError) -> PsqlpackError { LexicalError(err.line.to_owned(), err.line_number, err.start_pos, err.end_pos).into() };
            fn parse(err: lalrpop_util::ParseError<(), lexer::Token, ()>) -> PsqlpackError { InlineParseError(err).into() };

            let function_args =
                if raw_args.is_empty() {
                    Vec::new()
                } else {
                    lexer::tokenize(&raw_args).map_err(lexical)
                    .and_then(|tokens| parser::parse_function_argument_list(tokens).map_err(parse))
                    .chain_err(|| PackageFunctionArgsInspectError(raw_args))?
                };
            let return_type =
                lexer::tokenize(&raw_result).map_err(lexical)
                .and_then(|tokens| parser::parse_function_return_type(tokens).map_err(parse))
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

        let tables =
            db_conn.query(Q_TABLES, &[])
            .chain_err(|| PackageQueryTablesError)?;

        Ok(Package {
            extensions: map!(extensions),
            functions: functions, // functions,
            schemas: map!(schemas), // schemas,
            scripts: Vec::new(), // Scripts can't be known from a connection
            tables: map!(tables), // tables,
            types: map!(types), // types,
        })
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
        use std::ascii::AsciiExt;

        // Make sure the public schema exists
        let mut has_public = false;
        for schema in &mut self.schemas {
            if "public".eq_ignore_ascii_case(&schema.name[..]) {
                has_public = true;
                break;
            }
        }
        if !has_public {
            self.schemas.push(SchemaDefinition { name: "public".to_owned() });
        }

        // Set default schema's
        for table in &mut self.tables {
            if table.name.schema.is_none() {
                table.name.schema = Some(project.default_schema.clone());
            }
            if let Some(ref mut constraints) = table.constraints {
                for constraint in constraints.iter_mut() {
                    if let TableConstraint::Foreign { ref mut ref_table, .. } = *constraint {
                        if ref_table.schema.is_none() {
                            ref_table.schema = Some(project.default_schema.clone());
                        }
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
            if let Some(ref table_constaints) = table.constraints {
                let log = log.new(o!("table" => table.name.to_string()));
                let table_node = Node::Table(&table);
                trace!(log, "Scanning constraints");
                for constraint in table_constaints {
                    let constraint_node = constraint.graph(&log, &mut graph, Some(&table_node));
                    graph.add_edge(table_node, constraint_node, ());
                }
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
        // TODO: Validate references etc
        Ok(())
    }
}

#[derive(Clone,Copy,PartialEq,Eq,PartialOrd,Ord,Hash)]
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
            Node::Constraint(table, constraint) => write!(f, "Constraint: {}.{}", table.name.to_string(), constraint.name()),
            Node::Function(function) => write!(f, "Function:   {}", function.name.to_string()),
        }
    }
}

type Graph<'graph> = petgraph::graphmap::GraphMap<Node<'graph>, (), petgraph::Directed>;

trait Graphable {
    fn graph<'graph, 'def: 'graph>(&'def self, log: &Logger, graph: &mut Graph<'graph>, parent: Option<&Node<'graph>>) -> Node<'graph>;
}

impl Graphable for TableDefinition {
    fn graph<'graph, 'def: 'graph>(&'def self, log: &Logger, graph:&mut Graph<'graph>, _:Option<&Node<'graph>>) -> Node<'graph> {
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
    fn graph<'graph, 'def: 'graph>(&'def self, log: &Logger, graph: &mut Graph<'graph>, parent: Option<&Node<'graph>>) -> Node<'graph> {
        // Column does have a parent - namely the table
        let table = match *parent.unwrap() {
            Node::Table(table) => table,
            _ => panic!("Non table parent for column."),
        };
        trace!(log, "Adding");
        graph.add_node(Node::Column(table, &self))
    }
}

impl Graphable for FunctionDefinition {
    fn graph<'graph, 'def: 'graph>(&'def self, log: &Logger, graph: &mut Graph<'graph>, _: Option<&Node<'graph>>) -> Node<'graph> {
        // It will not have a parent - the schema is embedded in the name
        trace!(log, "Adding");
        graph.add_node(Node::Function(self))
    }
}

impl Graphable for TableConstraint {
    fn graph<'graph, 'def: 'graph>(&'def self, log: &Logger, graph: &mut Graph<'graph>, parent: Option<&Node<'graph>>) -> Node<'graph>  {
        // We currently have two types of table constraints: Primary and Foreign
        // Primary is easy with a direct dependency to the column
        // Foreign requires a weighted dependency
        // This does have a parent - namely the table
        let table = match *parent.unwrap() {
            Node::Table(table) => table,
            _ => panic!("Non table parent for column."),
        };
        match *self {
            TableConstraint::Primary { ref name, ref columns, .. } => {
                let log = log.new(o!("primary constraint" => name.to_owned()));
                // Primary relies on the columns existing (of course)
                trace!(log, "Adding");
                let constraint = graph.add_node(Node::Constraint(table, self));
                for column_name in columns {
                    trace!(log, "Adding edge to column"; "column" => &column_name);
                    let column = table.columns.iter().find(|x| &x.name == column_name).unwrap();
                    graph.add_edge(Node::Column(table, column), constraint, ());
                }
                constraint
            },
            TableConstraint::Foreign { ref name, ref columns, ref ref_table, ref ref_columns, .. } => {
                let log = log.new(o!("foreign constraint" => name.to_owned()));
                // Foreign has two types of edges
                trace!(log, "Adding");
                let constraint = graph.add_node(Node::Constraint(table, self));
                // Add edges to the columns in this table.
                for column_name in columns {
                    trace!(log, "Adding edge to column"; "column" => &column_name);
                    let column = table.columns.iter().find(|x| &x.name == column_name).unwrap();
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
                    trace!(log, "Adding edge to refrenced column"; "table" => ref_table.to_string(), "column" => &ref_column_name);

                    let ref_column = table_def.columns.iter().find(|x| &x.name == ref_column_name).unwrap();
                    graph.add_edge(Node::Column(table_def, ref_column), constraint, ());

                    // If required, add an edge to any primary keys.
                    if let Some(ref constraints) = table_def.constraints {
                        for primary in constraints {
                            if let TableConstraint::Primary { ref columns, .. } = *primary {
                                if columns.contains(ref_column_name) {
                                    graph.add_edge(Node::Constraint(table_def, &primary), constraint, ());
                                }
                            }
                        }
                    }
                }
                constraint
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::connection::*;
    use spectral::prelude::*;

    #[test]
    fn it_can_create_a_package_from_a_database() {
        //TODO: Create database before test
        let connection = ConnectionBuilder::new("taxengine", "localhost", "paul").build().unwrap();
        let package = Package::from_connection(&connection).unwrap();
        assert_that(&package.extensions).has_length(4);
    }

}
