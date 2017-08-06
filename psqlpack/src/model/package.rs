use std::path::Path;
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;

use connection::Connection;
use serde_json;
use zip::{ZipArchive, ZipWriter};
use zip::write::FileOptions;

use sql::ast::*;
use graph::{DependencyGraph, Node, Edge, ValidationResult};
use model::Project;
use errors::{PsqlpackResult, PsqlpackResultExt};
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
static Q_SCHEMAS : &'static str = "SELECT schema_name FROM information_schema.schemata
                                   WHERE catalog_name = $1 AND schema_owner <> 'postgres'";
static Q_TYPES : &'static str = "SELECT typname, typcategory FROM pg_catalog.pg_type
                                 INNER JOIN pg_catalog.pg_namespace ON pg_namespace.oid=typnamespace
                                 WHERE typcategory IN ('E') AND nspname='public' AND substr(typname, 1, 1) <> '_'";
static Q_ENUMS : &'static str = "SELECT enumtypid, enumlabel
                                 FROM pg_catalog.pg_enum
                                 WHERE enumtypid IN ({})
                                 ORDER BY enumtypid, enumsortorder";
static Q_FUNCTIONS : &'static str = "SELECT *--routines.routine_name, parameters.data_type, parameters.ordinal_position
                                    FROM information_schema.routines
                                        JOIN information_schema.parameters ON routines.specific_name=parameters.specific_name
                                    WHERE routines.specific_catalog='taxengine'
                                    ORDER BY routines.routine_name, parameters.ordinal_position;";

pub struct Package {
    pub extensions: Vec<ExtensionDefinition>,
    pub functions: Vec<FunctionDefinition>,
    pub schemas: Vec<SchemaDefinition>,
    pub scripts: Vec<ScriptDefinition>,
    pub tables: Vec<TableDefinition>,
    pub types: Vec<TypeDefinition>,
    pub order: Option<Vec<Node>>,
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
        let mut order = None;

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
            } else if name.eq("order.json") {
                order = Some(
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
            order: order,
        })
    }

    pub fn from_connection(connection: &Connection) -> PsqlpackResult<Package> {
        // We do five SQL queries to get the package details
        let db_conn = dbtry!(connection.connect_database());

        let mut extensions = Vec::new();
        for row in &db_conn.query(Q_EXTENSIONS, &[]).unwrap() {
            let ext_name : String = row.get(0);
            extensions.push(ExtensionDefinition {
                name: ext_name
            });
        }

        let mut schemas = Vec::new();
        for row in &db_conn.query(Q_SCHEMAS, &[&connection.database()]).unwrap() {
            let schema_name : String = row.get(0);
            schemas.push(SchemaDefinition {
                name: schema_name
            });
        }

        let mut types = Vec::new();
        let mut enums = HashMap::new();
        for row in &db_conn.query(Q_TYPES, &[]).unwrap() {
            let oid : i32 = row.get(0);
            let typname : String = row.get(1);
            enums.insert(oid, typname);
        }
        let mut enum_values = Vec::new();
        let mut last_oid : i32 = -1;
        let mut oids = String::new();
        for oid in enums.keys() {
            if oids.is_empty() {
                oids.push_str(&format!("{}", oid)[..]);
            } else {
                oids.push_str(&format!(", {}", oid)[..]);
            }
        }
        if oids.len() > 0 {
            let query = Q_ENUMS.replace("{}", &oids[..]);
            for row in &db_conn.query(&query[..], &[]).unwrap() {
                let oid : i32 = row.get(0);
                let value : String = row.get(1);
                if oid != last_oid {
                    if last_oid > 0 {
                        types.push(TypeDefinition {
                            name: enums.get(&last_oid).unwrap().to_owned(),
                            kind: TypeDefinitionKind::Enum(enum_values.clone())
                        });
                        enum_values.clear();
                    }
                    last_oid = oid;
                }
                enum_values.push(value);
            }
            if last_oid > 0 {
                types.push(TypeDefinition {
                    name: enums.get(&last_oid).unwrap().to_owned(),
                    kind: TypeDefinitionKind::Enum(enum_values)
                });
            }
        }

        let mut functions = Vec::new();

        let mut tables = Vec::new();

        Ok(Package {
            extensions: extensions,
            functions: functions, // functions,
            schemas: schemas, // schemas,
            scripts: Vec::new(), // Scripts can't be known from a connection
            tables: tables, // tables,
            types: types, // types,
            order: None,
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

            // Also, do the order if we have it defined
            if let Some(ref order) = self.order {
                ztry!(zip.start_file("order.json", FileOptions::default()));
                let json = match serde_json::to_string_pretty(&order) {
                    Ok(j) => j,
                    Err(e) => bail!(GenerationError(format!("Failed to write package: {}", e))),
                };
                ztry!(zip.write_all(json.as_bytes()));
            }

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
            order: None,
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

    pub fn generate_dependency_graph(&mut self) -> PsqlpackResult<()> {
        let mut graph = DependencyGraph::new();

        // Go through and add each object and add it to the graph
        // Extensions, schemas and types are always implied
        for table in &self.tables {
            table.generate_dependencies(&mut graph, None);
        }
        for function in &self.functions {
            function.generate_dependencies(&mut graph, None);
        }

        // Make sure it's valid first up
        match graph.validate() {
            ValidationResult::Valid => {},
            ValidationResult::CircularReference => bail!(GenerationError("Circular reference detected".to_owned())),
            // TODO: List out unresolved references
            ValidationResult::UnresolvedDependencies => bail!(GenerationError("Unresolved dependencies detected".to_owned())),
        }

        // Then generate the order
        let order = graph.topological_sort();
        // Should we also add schema etc in there? Not really necessary...
        self.order = Some(order);
        Ok(())
    }

    pub fn validate(&self) -> PsqlpackResult<()> {
        // TODO: Validate references etc
        Ok(())
    }
}


trait GenerateDependencyGraph {
    fn generate_dependencies(&self, graph:&mut DependencyGraph, parent:Option<String>) -> Node;
}

impl GenerateDependencyGraph for TableDefinition {
    fn generate_dependencies(&self, graph:&mut DependencyGraph, _:Option<String>) -> Node {
        // Table is dependent on a schema, so add the edge
        // It will not have a parent - the schema is embedded in the name
        let full_name = self.name.to_string();
        let table_node = Node::Table(full_name.clone());
        graph.add_node(&table_node);
        for column in &self.columns {
            // Column doesn't know that it's dependent on this table so add it here
            let col_node = column.generate_dependencies(graph, Some(full_name.clone()));
            graph.add_edge(&col_node, Edge::new(&table_node, 1.0));
        }
        if let Some(ref table_constaints) = self.constraints {
            for constraint in table_constaints {
                let table_constraint_node = constraint.generate_dependencies(graph, Some(full_name.clone()));
                graph.add_edge(&table_constraint_node, Edge::new(&table_node, 1.0));
            }
        }
        table_node
    }
}

impl GenerateDependencyGraph for ColumnDefinition {
    fn generate_dependencies(&self, graph:&mut DependencyGraph, parent:Option<String>) -> Node {
        // Column does have a parent - namely the table
        let column_node = Node::Column(format!("{}.{}", parent.unwrap(), self.name));
        graph.add_node(&column_node);
        column_node
    }
}

impl GenerateDependencyGraph for FunctionDefinition {
    fn generate_dependencies(&self, graph:&mut DependencyGraph, _:Option<String>) -> Node {
        // Function is dependent on a schema, so add the edge
        // It will not have a parent - the schema is embedded in the name
        let function_node = Node::Function(self.name.to_string());
        graph.add_node(&function_node);
        function_node
    }
}

impl GenerateDependencyGraph for TableConstraint {
    fn generate_dependencies(&self, graph:&mut DependencyGraph, parent:Option<String>) -> Node {
        // We currently have two types of table constraints: Primary and Foreign
        // Primary is easy with a direct dependency to the column
        // Foreign requires a weighted dependency
        // This does have a parent - namely the table
        let table = parent.unwrap();
        match *self {
            TableConstraint::Primary { ref name, ref columns, .. } => {
                // Primary relies on the columns existing (of course)
                let node = Node::Constraint(format!("{}.{}", table.clone(), name));
                graph.add_node(&node);
                for column in columns {
                    graph.add_edge(&node, Edge::new(&Node::Column(format!("{}.{}", table.clone(), column)), 1.0));
                }
                node
            },
            TableConstraint::Foreign { ref name, ref columns, ref ref_table, ref ref_columns, .. } => {
                // Foreign has two types of edges
                let node = Node::Constraint(format!("{}.{}", table.clone(), name));
                graph.add_node(&node);
                for column in columns {
                    graph.add_edge(&node, Edge::new(&Node::Column(format!("{}.{}", table.clone(), column)), 1.0));
                }
                for column in ref_columns {
                    graph.add_edge(&node, Edge::new(
                        &Node::Column(
                            format!("{}.{}", ref_table.to_string(), column)
                            ), 1.1));
                }
                node
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
