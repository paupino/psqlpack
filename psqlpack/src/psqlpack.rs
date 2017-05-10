use std::ascii::AsciiExt;
use std::fmt;
use std::fs::{self,File};
use std::io::Read;
use std::io::prelude::*;
use std::path::Path;

use serde_json;
use walkdir::WalkDir;
use zip::{ZipArchive,ZipWriter};
use zip::write::FileOptions;

use ast::*;
use connection::Connection;
use profiles::PublishProfile;
use project::Project;
use errors::*;
use graph::{DependencyGraph,Edge,Node,ValidationResult as DependencyGraphValidationResult};
use lexer;
use sql;

macro_rules! ztry {
    ($expr:expr) => {{
        match $expr {
            Ok(_) => {},
            Err(e) => bail!(PsqlpackErrorKind::GenerationError(format!("Failed to write package: {}", e))),
        }
    }};
}

macro_rules! dbtry {
    ($expr:expr) => {
        match $expr {
            Ok(o) => o,
            Err(e) => bail!(PsqlpackErrorKind::DatabaseError(format!("{}", e))),
        }
    };
}

macro_rules! zip_collection {
    ($zip:ident, $package:ident, $collection:ident) => {{
        let collection_name = stringify!($collection);
        ztry!($zip.add_directory(format!("{}/", collection_name), FileOptions::default()));
        for item in $package.$collection {
            ztry!($zip.start_file(format!("{}/{}.json", collection_name, item.name), FileOptions::default()));
            let json = match serde_json::to_string_pretty(&item) {
                Ok(j) => j,
                Err(e) => bail!(PsqlpackErrorKind::GenerationError(format!("Failed to write package: {}", e))),
            };
            ztry!($zip.write_all(json.as_bytes()));
        }
    }};
}

static Q_DATABASE_EXISTS : &'static str = "SELECT 1 FROM pg_database WHERE datname=$1;";
static Q_EXTENSION_EXISTS : &'static str = "SELECT 1 FROM pg_catalog.pg_extension WHERE extname=$1;";
static Q_SCHEMA_EXISTS : &'static str = "SELECT 1 FROM information_schema.schemata WHERE schema_name=$1;";
static Q_TYPE_EXISTS : &'static str = "SELECT 1 FROM pg_catalog.pg_type where typcategory <> 'A' AND typname=$1;";
static Q_TABLE_EXISTS : &'static str = "SELECT 1
                                        FROM pg_catalog.pg_class c
                                        JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
                                        WHERE n.nspname = $1 AND c.relname = $2 AND c.relkind = 'r';";
static Q_DESCRIBE_COLUMNS : &'static str = "SELECT ordinal_position, column_name, column_default, is_nullable, data_type, character_maximum_length, numeric_precision, numeric_scale
                                            FROM information_schema.columns
                                            WHERE table_schema = $1 AND table_name = $2
                                            ORDER BY ordinal_position;";

pub struct Psqlpack;

impl Psqlpack {
    pub fn package_project(project_path: &Path, output_path: &Path) -> PsqlpackResult<()> {
        // Load the project
        let project = Project::from_path(project_path)?;

        // Turn the pre/post into paths to quickly check
        let parent = project_path.parent().unwrap();
        let make_path = |script: &str| {
            parent
                .join(Path::new(script))
                .canonicalize()
                .chain_err(|| PsqlpackErrorKind::InvalidScriptPath(script.to_owned()))
        };

        let mut predeploy_paths = Vec::new();
        for script in &project.pre_deploy_scripts {
            predeploy_paths.push(make_path(script)?);
        }

        let mut postdeploy_paths = Vec::new();
        for script in &project.post_deploy_scripts {
            postdeploy_paths.push(make_path(script)?);
        }

        // Start the package
        let mut package = Package::new();
        let mut errors: Vec<PsqlpackError> = Vec::new();

        // Enumerate the directory
        for entry in WalkDir::new(parent).follow_links(false) {
            // Read in the file contents
            let e = entry.unwrap();
            let path = e.path();
            if path.extension().is_none() || path.extension().unwrap() != "sql" {
                continue;
            }

            let mut contents = String::new();
            if let Err(err) = File::open(&path).and_then(|mut f| f.read_to_string(&mut contents)) {
                errors.push(PsqlpackErrorKind::IOError(format!("{}", path.display()), format!("{}", err)).into());
                continue;
            }

            // Figure out if it's a pre/post deployment script
            let real_path = path.to_path_buf().canonicalize().unwrap();
            if let Some(pos) = predeploy_paths.iter().position(|x| real_path.eq(x)) {
                package.push_script(ScriptDefinition {
                    name: path.file_name().unwrap().to_str().unwrap().to_owned(),
                    kind: ScriptKind::PreDeployment,
                    order: pos,
                    contents: contents
                });
            } else if let Some(pos) = postdeploy_paths.iter().position(|x| real_path.eq(x)) {
                package.push_script(ScriptDefinition {
                    name: path.file_name().unwrap().to_str().unwrap().to_owned(),
                    kind: ScriptKind::PostDeployment,
                    order: pos,
                    contents: contents
                });
            } else {
                let tokens = match lexer::tokenize(&contents[..]) {
                    Ok(t) => t,
                    Err(e) => {
                        errors.push(PsqlpackErrorKind::SyntaxError(
                            format!("{}", path.display()),
                            e.line.to_owned(),
                            e.line_number as usize,
                            e.start_pos as usize,
                            e.end_pos as usize,
                        ).into());
                        continue;
                    },
                };

                match sql::parse_statement_list(tokens) {
                    Ok(statement_list) => {
                        for statement in statement_list {
                            match statement {
                                Statement::Extension(extension_definition) => package.push_extension(extension_definition),
                                Statement::Function(function_definition) => package.push_function(function_definition),
                                Statement::Schema(schema_definition) => package.push_schema(schema_definition),
                                Statement::Table(table_definition) => package.push_table(table_definition),
                                Statement::Type(type_definition) => package.push_type(type_definition),
                            }
                        }
                    },
                    Err(err) => {
                        errors.push(PsqlpackErrorKind::ParseError(format!("{}", path.display()), vec!(err)).into());
                        continue;
                    }
                }
            }
        }

        // Early exit if errors
        if !errors.is_empty() {
            bail!(PsqlpackErrorKind::MultipleErrors(errors));
        }

        // Update any missing defaults, create a dependency graph and then try to validate the project
        package.set_defaults(&project);
        try!(package.generate_dependency_graph());
        try!(package.validate());

        // Now generate the prackage
        if let Some(parent) = output_path.parent() {
            match fs::create_dir_all(format!("{}", parent.display())) {
                Ok(_) => {},
                Err(e) => bail!(PsqlpackErrorKind::GenerationError(format!("Failed to create package directory: {}", e))),
            }
        }

        let output_file = match File::create(&output_path) {
            Ok(f) => f,
            Err(e) => bail!(PsqlpackErrorKind::GenerationError(format!("Failed to write package: {}", e)))
        };
        let mut zip = ZipWriter::new(output_file);

        zip_collection!(zip, package, extensions);
        zip_collection!(zip, package, functions);
        zip_collection!(zip, package, schemas);
        zip_collection!(zip, package, scripts);
        zip_collection!(zip, package, tables);
        zip_collection!(zip, package, types);

        // Also, do the order if we have it defined
        if let Some(order) = package.order {
            ztry!(zip.start_file("order.json", FileOptions::default()));
            let json = match serde_json::to_string_pretty(&order) {
                Ok(j) => j,
                Err(e) => bail!(PsqlpackErrorKind::GenerationError(format!("Failed to write package: {}", e))),
            };
            ztry!(zip.write_all(json.as_bytes()));
        }

        ztry!(zip.finish());

        Ok(())
    }

    pub fn publish(source_package_path: &Path, target_connection_string: String, publish_profile: &Path) -> PsqlpackResult<()> {

        let package = try!(Psqlpack::load_package(source_package_path));
        let publish_profile = PublishProfile::from_path(publish_profile)?;
        let connection = try!(target_connection_string.parse());

        // Now we generate our instructions
        let changeset = package.generate_changeset(&connection, publish_profile)?;

        // These instructions turn into SQL statements that get executed
        let mut conn = dbtry!(connection.connect_host());
        for change in &changeset {
            if let ChangeInstruction::UseDatabase(..) = *change {
                dbtry!(conn.finish());
                conn = dbtry!(connection.connect_database());
                continue;
            }

            // Execute SQL directly
            info!("{}", change.to_progress_message());
            dbtry!(conn.execute(&change.to_sql()[..], &[]));
        }
        // Close the connection
        dbtry!(conn.finish());

        Ok(())
    }

    pub fn generate_sql(source_package_path: &Path, target_connection_string: String, publish_profile: &Path, output_file: &Path) -> PsqlpackResult<()> {

        let package = try!(Psqlpack::load_package(source_package_path));
        let publish_profile = PublishProfile::from_path(publish_profile)?;
        let connection = try!(target_connection_string.parse());

        // Now we generate our instructions
        let changeset = package.generate_changeset(&connection, publish_profile)?;

        // These instructions turn into a single SQL file
        let mut out = match File::create(output_file) {
            Ok(o) => o,
            Err(e) => bail!(PsqlpackErrorKind::GenerationError(format!("Failed to generate SQL file: {}", e)))
        };

        for change in changeset {
            match out.write_all(change.to_sql().as_bytes()) {
                Ok(_) => {
                    // New line
                    match out.write(&[59u8, 10u8, 10u8]) {
                        Ok(_) => {},
                        Err(e) => bail!(PsqlpackErrorKind::GenerationError(format!("Failed to generate SQL file: {}", e)))
                    }
                },
                Err(e) => bail!(PsqlpackErrorKind::GenerationError(format!("Failed to generate SQL file: {}", e)))
            }
        }

        Ok(())
    }

    pub fn generate_report(source_package_path: &Path, target_connection_string: String, publish_profile: &Path, output_file: &Path) -> PsqlpackResult<()> {

        let package = try!(Psqlpack::load_package(source_package_path));
        let publish_profile = PublishProfile::from_path(publish_profile)?;
        let connection = try!(target_connection_string.parse());

        // Now we generate our instructions
        let changeset = package.generate_changeset(&connection, publish_profile)?;

        // These instructions turn into a JSON report
        let json = match serde_json::to_string_pretty(&changeset) {
            Ok(j) => j,
            Err(e) => bail!(PsqlpackErrorKind::GenerationError(format!("Failed to generate report: {}", e)))
        };

        let mut out = match File::create(output_file) {
            Ok(o) => o,
            Err(e) => bail!(PsqlpackErrorKind::GenerationError(format!("Failed to generate report: {}", e)))
        };
        match out.write_all(json.as_bytes()) {
            Ok(_) => {},
            Err(e) => bail!(PsqlpackErrorKind::GenerationError(format!("Failed to generate report: {}", e)))
        }

        Ok(())
    }

    fn load_package(source_path: &Path) -> PsqlpackResult<Package> {
        let mut archive =
            File::open(&source_path)
            .chain_err(|| PsqlpackErrorKind::PackageReadError(source_path.to_path_buf()))
            .and_then(|file| {
                ZipArchive::new(file)
                .chain_err(|| PsqlpackErrorKind::PackageUnarchiveError(source_path.to_path_buf()))
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
            let mut file = archive.by_index(i).unwrap();
            if file.size() == 0 {
                continue;
            }
            let name = file.name().to_owned();
            if name.starts_with("extensions/") {
                extensions.push(
                    serde_json::from_reader(file)
                    .chain_err(|| PsqlpackErrorKind::PackageInternalReadError(name))?);
            } else if name.starts_with("functions/") {
                functions.push(
                    serde_json::from_reader(file)
                    .chain_err(|| PsqlpackErrorKind::PackageInternalReadError(name))?);
            } else if name.starts_with("schemas/") {
                schemas.push(
                    serde_json::from_reader(file)
                    .chain_err(|| PsqlpackErrorKind::PackageInternalReadError(name))?);
            } else if name.starts_with("scripts/") {
                scripts.push(
                    serde_json::from_reader(file)
                    .chain_err(|| PsqlpackErrorKind::PackageInternalReadError(name))?);
            } else if name.starts_with("tables/") {
                tables.push(
                    serde_json::from_reader(file)
                    .chain_err(|| PsqlpackErrorKind::PackageInternalReadError(name))?);
            } else if name.starts_with("types/") {
                types.push(
                    serde_json::from_reader(file)
                    .chain_err(|| PsqlpackErrorKind::PackageInternalReadError(name))?);
            } else if name.eq("order.json") {
                order = Some(
                    serde_json::from_reader(file)
                    .chain_err(|| PsqlpackErrorKind::PackageInternalReadError(name))?);
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
        match self.constraints {
            Some(ref table_constaints) => {
                for constraint in table_constaints {
                    let table_constraint_node = constraint.generate_dependencies(graph, Some(full_name.clone()));
                    graph.add_edge(&table_constraint_node, Edge::new(&table_node, 1.0));
                }
            },
            None => {}
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

enum DbObject<'a> {
    Extension(&'a ExtensionDefinition), // 2
    Function(&'a FunctionDefinition), // 6 (ordered)
    Schema(&'a SchemaDefinition), // 3
    Script(&'a ScriptDefinition), // 1, 7
    Table(&'a TableDefinition), // 5 (ordered)
    Type(&'a TypeDefinition), // 4
}

struct Package {
    extensions: Vec<ExtensionDefinition>,
    functions: Vec<FunctionDefinition>,
    schemas: Vec<SchemaDefinition>,
    scripts: Vec<ScriptDefinition>,
    tables: Vec<TableDefinition>,
    types: Vec<TypeDefinition>,
    order: Option<Vec<Node>>,
}

impl Package {
    fn new() -> Self {
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

    fn push_extension(&mut self, extension: ExtensionDefinition) {
        self.extensions.push(extension);
    }

    fn push_function(&mut self, function: FunctionDefinition) {
        self.functions.push(function);
    }

    fn push_script(&mut self, script: ScriptDefinition) {
        self.scripts.push(script);
    }

    fn push_schema(&mut self, schema: SchemaDefinition) {
        self.schemas.push(schema);
    }

    fn push_table(&mut self, table: TableDefinition) {
        self.tables.push(table);
    }

    fn push_type(&mut self, def: TypeDefinition) {
        self.types.push(def);
    }

    fn set_defaults(&mut self, project: &Project) {
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

    fn generate_dependency_graph(&mut self) -> PsqlpackResult<()> {
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
            DependencyGraphValidationResult::Valid => {},
            DependencyGraphValidationResult::CircularReference => bail!(PsqlpackErrorKind::GenerationError("Circular reference detected".to_owned())),
            // TODO: List out unresolved references
            DependencyGraphValidationResult::UnresolvedDependencies => bail!(PsqlpackErrorKind::GenerationError("Unresolved dependencies detected".to_owned())),
        }

        // Then generate the order
        let order = graph.topological_sort();
        // Should we also add schema etc in there? Not really necessary...
        self.order = Some(order);
        Ok(())
    }

    fn validate(&self) -> PsqlpackResult<()> {
        // TODO: Validate references etc
        Ok(())
    }

    fn generate_changeset(&self, connection: &Connection, publish_profile: PublishProfile) -> PsqlpackResult<Vec<ChangeInstruction>> {
        // Start the changeset
        let mut changeset = Vec::new();

        // Create the build order - including all document types outside the topological sort.
        let mut build_order = Vec::new();

        // Pre deployment scripts
        for script in &self.scripts {
            if script.kind == ScriptKind::PreDeployment {
                build_order.push(DbObject::Script(script));
            }
        }

        // Extensions
        for extension in &self.extensions {
            build_order.push(DbObject::Extension(extension));
        }

        // Schemas
        for schema in &self.schemas {
            build_order.push(DbObject::Schema(schema));
        }

        // Types
        for t in &self.types {
            build_order.push(DbObject::Type(t));
        }

        // Now add everything else per the topological sort
        if let Some(ref ordered_items) = self.order {
            for item in ordered_items {
                // Not the most efficient algorithm, perhaps something to cleanup
                match *item {
                    Node::Column(_) => { /* Necessary for ordering however unused here for now */ },
                    Node::Constraint(_) => { /* Necessary for ordering however unused here for now */ },
                    Node::Function(ref name) => {
                        if let Some(function) = self.functions.iter().find(|x| x.name.to_string() == *name) {
                            build_order.push(DbObject::Function(function));
                        } else {
                            // Warning?
                        }
                    },
                    Node::Table(ref name) => {
                        if let Some(table) = self.tables.iter().find(|x| x.name.to_string() == *name) {
                            build_order.push(DbObject::Table(table));
                        } else {
                            // Warning?
                        }
                    },
                }
            }
        } else {
            panic!("Internal state error: order was not generated");
        }

        // Add in post deployment scripts
        for script in &self.scripts {
            if script.kind == ScriptKind::PostDeployment {
                build_order.push(DbObject::Script(script));
            }
        }

        // First up, detect if there is no database (or it needs to be recreated)
        // If so, we assume everything is new
        let db_conn = dbtry!(connection.connect_host());
        let db_result = dbtry!(db_conn.query(Q_DATABASE_EXISTS, &[ &connection.database() ]));
        let mut has_db = !db_result.is_empty();

        // If we always recreate then add a drop and set to false
        if has_db && publish_profile.always_recreate_database {
            changeset.push(ChangeInstruction::DropDatabase(connection.database().to_owned()));
            has_db = false;
        }

        // If we have the DB we generate an actual change set, else we generate new instructions
        if has_db {

            // Set the connection instruction
            changeset.push(ChangeInstruction::UseDatabase(connection.database().to_owned()));

            // Connect to the database
            let conn = dbtry!(connection.connect_database());

            // Go through each item in order and figure out what to do with it
            for item in &build_order {
                match *item {
                    DbObject::Extension(ref extension) => {
                        // Only add the extension if it does not already exist
                        let mut extension_exists = false;
                        for _ in &conn.query(Q_EXTENSION_EXISTS, &[ &extension.name ]).unwrap() {
                            extension_exists = true;
                            break;
                        }
                        if !extension_exists {
                            changeset.push(ChangeInstruction::AddExtension(extension));
                        }
                    },
                    DbObject::Function(ref function) => {
                        // Since we don't really need to worry about this in PG we just
                        // add it as is and rely on CREATE OR REPLACE. In the future, it'd
                        // be good to check the hash or something to only do this when required
                        changeset.push(ChangeInstruction::ModifyFunction(function));
                    },
                    DbObject::Schema(ref schema) => {
                        // Only add schema's, we do not drop them at this point
                        let mut schema_exists = false;
                        for _ in &conn.query(Q_SCHEMA_EXISTS, &[ &schema.name ]).unwrap() {
                            schema_exists = true;
                            break;
                        }
                        if !schema_exists {
                            changeset.push(ChangeInstruction::AddSchema(schema));
                        }
                    },
                    DbObject::Script(ref script) => {
                        changeset.push(ChangeInstruction::RunScript(script));
                    },
                    DbObject::Table(ref table) => {
                        let mut table_exists = false;
                        for _ in &conn.query(Q_TABLE_EXISTS, &[ &table.name.schema, &table.name.name ]).unwrap() {
                            table_exists = true;
                            break;
                        }
                        if table_exists {
                            // Check the columns
                            for _ in &conn.query(Q_DESCRIBE_COLUMNS, &[ &table.name.schema, &table.name.name ]).unwrap() {
                                //let column_name : String = column.get(1);
                            }

                            // Check the constraints
                        } else {
                            changeset.push(ChangeInstruction::AddTable(table));
                        }
                    },
                    DbObject::Type(ref t) => {
                        let mut type_exists = false;
                        for _ in &conn.query(Q_TYPE_EXISTS, &[ &t.name ]).unwrap() {
                            type_exists = true;
                            break;
                        }
                        if type_exists {
                            // TODO: Need to figure out if it's changed and also perhaps how it's changed. I don't think a blanket modify is enough.
                        } else {
                            changeset.push(ChangeInstruction::AddType(t));
                        }
                    }
                }
            }
        } else {
            changeset.push(ChangeInstruction::CreateDatabase(connection.database().to_owned()));
            changeset.push(ChangeInstruction::UseDatabase(connection.database().to_owned()));

            // Since this is a new database add everything (in order)
            for item in &build_order {
                match *item {
                    DbObject::Extension(ref extension) => {
                        changeset.push(ChangeInstruction::AddExtension(extension));
                    },
                    DbObject::Function(ref function) => {
                        changeset.push(ChangeInstruction::AddFunction(function));
                    },
                    DbObject::Schema(ref schema) => {
                        changeset.push(ChangeInstruction::AddSchema(schema));
                    },
                    DbObject::Script(ref script) => {
                        changeset.push(ChangeInstruction::RunScript(script));
                    },
                    DbObject::Table(ref table) => {
                        changeset.push(ChangeInstruction::AddTable(table));
                    },
                    DbObject::Type(ref t) => {
                        changeset.push(ChangeInstruction::AddType(t));
                    }
                }
            }
        }
        Ok(changeset)
    }
}

#[allow(dead_code)]
#[derive(Serialize)]
enum ChangeInstruction<'input> {
    // Databases
    DropDatabase(String),
    CreateDatabase(String),
    UseDatabase(String),

    // Extensions
    AddExtension(&'input ExtensionDefinition),

    // Schema
    AddSchema(&'input SchemaDefinition),
    //RemoveSchema(String),

    // Scripts
    RunScript(&'input ScriptDefinition),

    // Types
    AddType(&'input TypeDefinition),
    RemoveType(String),

    // Tables
    AddTable(&'input TableDefinition),
    RemoveTable(String),

    // Columns
    AddColumn(&'input ColumnDefinition),
    ModifyColumn(&'input ColumnDefinition),
    RemoveColumn(String),

    // Functions
    AddFunction(&'input FunctionDefinition),
    ModifyFunction(&'input FunctionDefinition), // This is identical to add however it's for future possible support
    DropFunction(String),

}

impl<'input> ChangeInstruction<'input> {
    fn to_sql(&self) -> String {
        match *self {
            // Database level
            ChangeInstruction::CreateDatabase(ref db) => {
                format!("CREATE DATABASE {}", db)
            },
            ChangeInstruction::DropDatabase(ref db) => {
                format!("DROP DATABASE {}", db)
            },
            ChangeInstruction::UseDatabase(ref db) => {
                format!("-- Using database `{}`", db)
            },

            // Extension level
            ChangeInstruction::AddExtension(ref ext) => {
                format!("CREATE EXTENSION {}", ext.name)
            },

            // Schema level
            ChangeInstruction::AddSchema(ref schema) => {
                format!("CREATE SCHEMA {}", schema.name)
            },

            // Type level
            ChangeInstruction::AddType(ref t) => {
                let mut def = String::new();
                def.push_str(&format!("CREATE TYPE {} AS ", t.name)[..]);
                match t.kind {
                    TypeDefinitionKind::Alias(ref sql_type) => {
                        def.push_str(&sql_type.to_string()[..]);
                    },
                    TypeDefinitionKind::Enum(ref values) => {
                        def.push_str("ENUM (\n");
                        let mut enum_comma_required = false;
                        for value in values {
                            if enum_comma_required {
                                def.push_str(",\n");
                            } else {
                                enum_comma_required = true;
                            }
                            def.push_str(&format!("  '{}'", value)[..]);
                        }
                        def.push_str("\n)");
                    }
                }
                def
            },

            // Function level
            ChangeInstruction::AddFunction(ref function) | ChangeInstruction::ModifyFunction(ref function) => {
                let mut func = String::new();
                func.push_str(&format!("CREATE OR REPLACE FUNCTION {} (", function.name)[..]);
                let mut arg_comma_required = false;
                for arg in &function.arguments {
                    if arg_comma_required {
                        func.push_str(", ");
                    } else {
                        arg_comma_required = true;
                    }

                    func.push_str(&format!("{} {}", arg.name, arg.sql_type)[..]);
                }
                func.push_str(")\n");
                func.push_str("RETURNS ");
                match function.return_type {
                    FunctionReturnType::Table(ref columns) => {
                        func.push_str("TABLE (\n");
                        let mut column_comma_required = false;
                        for column in columns {
                            if column_comma_required {
                                func.push_str(",\n");
                            } else {
                                column_comma_required = true;
                            }
                            func.push_str(&format!("  {} {}", column.name, column.sql_type)[..]);
                        }
                        func.push_str("\n)\n");
                    },
                    FunctionReturnType::SqlType(ref sql_type) => {
                        func.push_str(&format!("{} ", sql_type)[..]);
                    }
                }
                func.push_str("AS $$");
                func.push_str(&function.body[..]);
                func.push_str("$$\n");
                func.push_str("LANGUAGE ");
                match function.language {
                    FunctionLanguage::C => func.push_str("C"),
                    FunctionLanguage::Internal => func.push_str("INTERNAL"),
                    FunctionLanguage::PostgreSQL => func.push_str("PGSQL"),
                    FunctionLanguage::SQL => func.push_str("SQL")
                }
                func
            },

            // Table level
            ChangeInstruction::AddTable(def) => {
                let mut instr = String::new();
                instr.push_str(&format!("CREATE TABLE {} (\n", def.name)[..]);
                for (index, column) in def.columns.iter().enumerate() {
                    if index > 0 {
                        instr.push_str(",\n");
                    }
                    instr.push_str(&format!("  {} {}", column.name, column.sql_type)[..]);
                    // Evaluate column constraints
                    if let Some(ref constraints) = column.constraints {
                        for constraint in constraints.iter() {
                            match *constraint {
                                ColumnConstraint::Default(ref any_type) => instr.push_str(&format!(" DEFAULT {}", any_type)),
                                ColumnConstraint::NotNull => instr.push_str(" NOT NULL"),
                                ColumnConstraint::Null => instr.push_str(" NULL"),
                                ColumnConstraint::Unique => instr.push_str(" UNIQUE"),
                                ColumnConstraint::PrimaryKey => instr.push_str(" PRIMARY KEY"),
                            }
                        }
                    }
                }
                if let Some(ref constraints) = def.constraints {
                    instr.push_str(",\n");
                    for (index, constraint) in constraints.iter().enumerate() {
                        if index > 0 {
                            instr.push_str(",\n");
                        }
                        match *constraint {
                            TableConstraint::Primary {
                                ref name,
                                ref columns,
                                ref parameters
                            } => {
                                instr.push_str(&format!("  CONSTRAINT {} PRIMARY KEY ({})", name, columns.join(", "))[..]);

                                // Do the WITH options too
                                if let Some(ref unwrapped) = *parameters {
                                    instr.push_str(" WITH (");
                                    for (position, value) in unwrapped.iter().enumerate() {
                                        if position > 0 {
                                            instr.push_str(", ");
                                        }
                                        match *value {
                                            IndexParameter::FillFactor(i) => instr.push_str(&format!("FILLFACTOR={}", i)[..]),
                                        }
                                    }
                                    instr.push_str(")");
                                }
                            },
                            TableConstraint::Foreign {
                                ref name,
                                ref columns,
                                ref ref_table,
                                ref ref_columns,
                                ref match_type,
                                ref events,
                            } => {
                                instr.push_str(&format!("  CONSTRAINT {} FOREIGN KEY ({})", name, columns.join(", "))[..]);
                                instr.push_str(&format!(" REFERENCES {} ({})", ref_table, ref_columns.join(", "))[..]);
                                if let Some(ref m) = *match_type {
                                    instr.push_str(&format!(" {}", m));
                                }
                                if let Some(ref events) = *events {
                                    for e in events {
                                        match *e {
                                            ForeignConstraintEvent::Delete(ref action) => instr.push_str(&format!(" ON DELETE {}", action)),
                                            ForeignConstraintEvent::Update(ref action) => instr.push_str(&format!(" ON UPDATE {}", action)),
                                        }
                                    }
                                }
                            },
                        }
                    }
                }
                instr.push_str("\n)");
                instr
            },

            // Raw scripts
            ChangeInstruction::RunScript(script) => {
                let mut instr = String::new();
                instr.push_str(&format!("-- Script: {}\n", script.name)[..]);
                instr.push_str(&script.contents[..]);
                instr.push('\n');
                instr
            }

            _ => {
                "TODO".to_owned()
            }
        }

    }

    fn to_progress_message(&self) -> String {
        match *self {
            // Database level
            ChangeInstruction::CreateDatabase(ref db) => format!("Creating database {}", db),
            ChangeInstruction::DropDatabase(ref db) => format!("Dropping database {}", db),
            ChangeInstruction::UseDatabase(ref db) => format!("Using database {}", db),

            // Table level
            ChangeInstruction::AddTable(def) => format!("Adding table {}", def.name),
            _ => "TODO".to_owned(),
        }

    }
}

impl fmt::Display for AnyValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AnyValue::Boolean(ref b) => write!(f, "{}", b),
            AnyValue::Integer(ref i) => write!(f, "{}", i),
            AnyValue::String(ref s) => write!(f, "'{}'", s),
        }
    }
}

impl fmt::Display for ForeignConstraintMatchType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ForeignConstraintMatchType::Simple => write!(f, "MATCH SIMPLE"),
            ForeignConstraintMatchType::Partial => write!(f, "MATCH PARTIAL"),
            ForeignConstraintMatchType::Full => write!(f, "MATCH FULL"),
        }
    }
}

impl fmt::Display for ForeignConstraintAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ForeignConstraintAction::NoAction => write!(f, "NO ACTION"),
            ForeignConstraintAction::Restrict => write!(f, "RESTRICT"),
            ForeignConstraintAction::Cascade => write!(f, "CASCADE"),
            ForeignConstraintAction::SetNull => write!(f, "SET NULL"),
            ForeignConstraintAction::SetDefault => write!(f, "SET DEFAULT"),
        }
    }
}

impl fmt::Display for ObjectName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.schema {
            Some(ref s) => write!(f, "{}.{}", s, self.name),
            None => write!(f, "{}", self.name),
        }
    }
}

impl fmt::Display for SqlType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SqlType::Simple(ref simple_type) => {
                write!(f, "{}", simple_type)
            },
            SqlType::Array(ref simple_type, dim) => {
                write!(f, "{}{}", simple_type, (0..dim).map(|_| "[]").collect::<String>())
            },
            SqlType::Custom(ref custom_type, ref options) => {
                if let Some(ref opt) = *options {
                    write!(f, "{}({})", custom_type, opt)
                } else {
                    write!(f, "{}", custom_type)
                }
            },
        }
    }
}

impl fmt::Display for SimpleSqlType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SimpleSqlType::FixedLengthString(size) => write!(f, "char({})", size),
            SimpleSqlType::VariableLengthString(size) => write!(f, "varchar({})", size),
            SimpleSqlType::Text => write!(f, "text"),

            SimpleSqlType::FixedLengthBitString(size) => write!(f, "bit({})", size),
            SimpleSqlType::VariableLengthBitString(size) => write!(f, "varbit({})", size),

            SimpleSqlType::SmallInteger => write!(f, "smallint"),
            SimpleSqlType::Integer => write!(f, "int"),
            SimpleSqlType::BigInteger => write!(f, "bigint"),

            SimpleSqlType::SmallSerial => write!(f, "smallserial"),
            SimpleSqlType::Serial => write!(f, "serial"),
            SimpleSqlType::BigSerial => write!(f, "bigserial"),

            SimpleSqlType::Numeric(m, d) => write!(f, "numeric({},{})", m, d),
            SimpleSqlType::Double => write!(f, "double precision"),
            SimpleSqlType::Single => write!(f, "real"),
            SimpleSqlType::Money => write!(f, "money"),

            SimpleSqlType::Boolean => write!(f, "bool"),

            SimpleSqlType::Date => write!(f, "date"),
            SimpleSqlType::DateTime => write!(f, "timestamp without time zone"),
            SimpleSqlType::DateTimeWithTimeZone => write!(f, "timestamp with time zone"),
            SimpleSqlType::Time => write!(f, "time"),
            SimpleSqlType::TimeWithTimeZone => write!(f, "time with time zone"),

            SimpleSqlType::Uuid => write!(f, "uuid"),
        }
    }
}
