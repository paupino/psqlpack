use std::ascii::AsciiExt;
use std::fmt;
use std::fs::{self,File};
use std::io::Read;
use std::io::prelude::*;
use std::path::Path;
use std::path::MAIN_SEPARATOR as PATH_SEPARATOR;

use serde_json;
use walkdir::WalkDir;
use zip::{ZipArchive,ZipWriter};
use zip::write::FileOptions;

use ast::*;
use connection::Connection;
use errors::*;
use graph::{DependencyGraph,Edge,Node,ValidationResult as DependencyGraphValidationResult};
use lexer;
use sql;

macro_rules! ztry {
    ($expr:expr) => {{
        match $expr {
            Ok(_) => {},
            Err(e) => return Err(DacpacErrorKind::GenerationError(format!("Failed to write DACPAC: {}", e)).into()),
        }
    }};
}

macro_rules! dbtry {
    ($expr:expr) => {
        match $expr {
            Ok(o) => o,
            Err(e) => return Err(DacpacErrorKind::DatabaseError(format!("{}", e)).into()),
        }
    };
}

macro_rules! load_file {
    ($file_type:ty, $coll:ident, $file:ident) => {{
        let mut contents = String::new();
        $file.read_to_string(&mut contents).unwrap();
        let object : $file_type = serde_json::from_str(&contents).unwrap();
        $coll.push(object);
    }};
}

macro_rules! zip_collection {
    ($zip:ident, $project:ident, $collection:ident) => {{
        let collection_name = stringify!($collection);
        ztry!($zip.add_directory(format!("{}/", collection_name), FileOptions::default()));
        for item in $project.$collection {
            ztry!($zip.start_file(format!("{}/{}.json", collection_name, item.name), FileOptions::default()));
            let json = match serde_json::to_string_pretty(&item) {
                Ok(j) => j,
                Err(e) => return Err(DacpacErrorKind::GenerationError(format!("Failed to write DACPAC: {}", e)).into()),
            };
            ztry!($zip.write_all(json.as_bytes()));
        }
    }};
}

static Q_DATABASE_EXISTS : &'static str = "SELECT 1 FROM pg_database WHERE datname=$1;";
static Q_EXTENSION_EXISTS : &'static str = "SELECT 1 FROM pg_catalog.pg_extension WHERE extname=$1;";
static Q_SCHEMA_EXISTS : &'static str = "SELECT 1 FROM information_schema.schemata WHERE schema_name=$1;";
static Q_TABLE_EXISTS : &'static str = "SELECT 1
                                        FROM pg_catalog.pg_class c
                                        JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
                                        WHERE n.nspname = $1 AND c.relname = $2 AND c.relkind = 'r';";
static Q_DESCRIBE_COLUMNS : &'static str = "SELECT ordinal_position, column_name, column_default, is_nullable, data_type, character_maximum_length, numeric_precision, numeric_scale
                                            FROM information_schema.columns
                                            WHERE table_schema = $1 AND table_name = $2
                                            ORDER BY ordinal_position;";

pub struct Dacpac;

impl Dacpac {
    pub fn package_project(source_project_file: String, output_file: String) -> DacpacResult<()> {

        // Load the project file
        let project_path = Path::new(&source_project_file[..]);
        if !project_path.is_file() {
            return Err(DacpacErrorKind::IOError(format!("{}", project_path.display()),"Project file does not exist".to_owned()).into());
        }
        let mut project_source = String::new();
        if let Err(err) = File::open(&project_path).and_then(|mut f| f.read_to_string(&mut project_source)) {
            return Err(DacpacErrorKind::IOError(format!("{}", project_path.display()), format!("Failed to read project file: {}", err)).into());
        }

        // Load the project config
        let project_config : ProjectConfig = match serde_json::from_str(&project_source) {
            Ok(c) => c,
            Err(e) => return Err(DacpacErrorKind::ProjectError(format!("{}", e)).into()),
        };
        // Turn the pre/post into paths to quickly check
        let mut predeploy_paths = Vec::new();
        let mut postdeploy_paths = Vec::new();
        let parent = project_path.parent().unwrap();
        for script in &project_config.pre_deploy_scripts {
            let path = match fs::canonicalize(Path::new(&format!("{}{}{}", parent.display(), PATH_SEPARATOR, script)[..])) {
                Ok(p) => p,
                Err(e) => return Err(DacpacErrorKind::ProjectError(format!("Invalid script found for pre-deployment: {} ({})", script, e)).into()),
            };
            predeploy_paths.push(format!("{}", path.display()));
        }
        for script in &project_config.post_deploy_scripts {
            let path = match fs::canonicalize(Path::new(&format!("{}{}{}", parent.display(), PATH_SEPARATOR, script)[..])) {
                Ok(p) => p,
                Err(e) => return Err(DacpacErrorKind::ProjectError(format!("Invalid script found for post-deployment: {} ({})", script, e)).into())
            };
            postdeploy_paths.push(format!("{}", path.display()));
        }

        // Start the project
        let mut project = Project::new();
        let mut errors = Vec::new();

        // Enumerate the directory
        for entry in WalkDir::new(project_path.parent().unwrap()).follow_links(false) {
            // Read in the file contents
            let e = entry.unwrap();
            let path = e.path();
            if path.extension().is_none() || path.extension().unwrap() != "sql" {
                continue;
            }

            let mut contents = String::new();
            if let Err(err) = File::open(&path).and_then(|mut f| f.read_to_string(&mut contents)) {
                errors.push(DacpacErrorKind::IOError(format!("{}", path.display()), format!("{}", err)));
                continue;
            }

            // Figure out if it's a pre/post deployment script
            let abs_path = format!("{}", fs::canonicalize(path).unwrap().display());
            if let Some(pos) = predeploy_paths.iter().position(|x| abs_path.eq(x)) {
                project.push_script(ScriptDefinition {
                    name: path.file_name().unwrap().to_str().unwrap().to_owned(),
                    kind: ScriptKind::PreDeployment,
                    order: pos,
                    contents: contents
                });
            } else if let Some(pos) = postdeploy_paths.iter().position(|x| abs_path.eq(x)) {
                project.push_script(ScriptDefinition {
                    name: path.file_name().unwrap().to_str().unwrap().to_owned(),
                    kind: ScriptKind::PostDeployment,
                    order: pos,
                    contents: contents
                });
            } else {
                let tokens = match lexer::tokenize(&contents[..]) {
                    Ok(t) => t,
                    Err(e) => {
                        errors.push(DacpacErrorKind::SyntaxError(
                            format!("{}", path.display()),
                            e.line.to_owned(),
                            e.line_number,
                            e.start_pos,
                            e.end_pos
                        ));
                        continue;
                    },
                };

                match sql::parse_statement_list(tokens) {
                    Ok(statement_list) => {
                        for statement in statement_list {
                            match statement {
                                Statement::Extension(extension_definition) => project.push_extension(extension_definition),
                                Statement::Function(function_definition) => project.push_function(function_definition),
                                Statement::Schema(schema_definition) => project.push_schema(schema_definition),
                                Statement::Table(table_definition) => project.push_table(table_definition),
                                Statement::Type(type_definition) => project.push_type(type_definition),
                            }
                        }
                    },
                    Err(err) => {
                        errors.push(DacpacErrorKind::ParseError(format!("{}", path.display()), vec!(err)));
                        continue;
                    }
                }
            }
        }

        // Early exit if errors
        if !errors.is_empty() {
            return Err(DacpacErrorKind::MultipleErrors(errors).into());
        }

        // Update any missing defaults, create a dependency graph and then try to validate the project
        project.set_defaults(&project_config);
        try!(project.generate_dependency_graph());
        try!(project.validate());

        // Now generate the dacpac
        let output_path = Path::new(&output_file[..]);
        if let Some(parent) = output_path.parent() {
            match fs::create_dir_all(format!("{}", parent.display())) {
                Ok(_) => {},
                Err(e) => return Err(DacpacErrorKind::GenerationError(format!("Failed to create DACPAC directory: {}", e)).into()),
            }
        }

        let output_file = match File::create(&output_path) {
            Ok(f) => f,
            Err(e) => return Err(DacpacErrorKind::GenerationError(format!("Failed to write DACPAC: {}", e)).into())
        };
        let mut zip = ZipWriter::new(output_file);

        zip_collection!(zip, project, extensions);
        zip_collection!(zip, project, functions);
        zip_collection!(zip, project, schemas);
        zip_collection!(zip, project, scripts);
        zip_collection!(zip, project, tables);
        zip_collection!(zip, project, types);

        // Also, do the order if we have it defined
        if let Some(order) = project.order {
            ztry!(zip.start_file("order.json", FileOptions::default()));
            let json = match serde_json::to_string_pretty(&order) {
                Ok(j) => j,
                Err(e) => return Err(DacpacErrorKind::GenerationError(format!("Failed to write DACPAC: {}", e)).into()),
            };
            ztry!(zip.write_all(json.as_bytes()));
        }

        ztry!(zip.finish());

        Ok(())
    }

    pub fn publish(source_dacpac_file: String, target_connection_string: String, publish_profile: String) -> DacpacResult<()> {

        let project = try!(Dacpac::load_project(source_dacpac_file));
        let publish_profile = try!(Dacpac::load_publish_profile(publish_profile));
        let connection = try!(target_connection_string.parse());

        // Now we generate our instructions
        let changeset = project.generate_changeset(&connection, publish_profile)?;

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

    pub fn generate_sql(source_dacpac_file: String, target_connection_string: String, publish_profile: String, output_file: String) -> DacpacResult<()> {

        let project = try!(Dacpac::load_project(source_dacpac_file));
        let publish_profile = try!(Dacpac::load_publish_profile(publish_profile));
        let connection = try!(target_connection_string.parse());

        // Now we generate our instructions
        let changeset = project.generate_changeset(&connection, publish_profile)?;

        // These instructions turn into a single SQL file
        let mut out = match File::create(&output_file[..]) {
            Ok(o) => o,
            Err(e) => return Err(DacpacErrorKind::GenerationError(format!("Failed to generate SQL file: {}", e)).into())
        };

        for change in changeset {
            match out.write_all(change.to_sql().as_bytes()) {
                Ok(_) => {
                    // New line
                    match out.write(&[59u8, 10u8, 10u8]) {
                        Ok(_) => {},
                        Err(e) => return Err(DacpacErrorKind::GenerationError(format!("Failed to generate SQL file: {}", e)).into())
                    }
                },
                Err(e) => return Err(DacpacErrorKind::GenerationError(format!("Failed to generate SQL file: {}", e)).into())
            }
        }

        Ok(())
    }

    pub fn generate_report(source_dacpac_file: String, target_connection_string: String, publish_profile: String, output_file: String) -> DacpacResult<()> {

        let project = try!(Dacpac::load_project(source_dacpac_file));
        let publish_profile = try!(Dacpac::load_publish_profile(publish_profile));
        let connection = try!(target_connection_string.parse());

        // Now we generate our instructions
        let changeset = project.generate_changeset(&connection, publish_profile)?;

        // These instructions turn into a JSON report
        let json = match serde_json::to_string_pretty(&changeset) {
            Ok(j) => j,
            Err(e) => return Err(DacpacErrorKind::GenerationError(format!("Failed to generate report: {}", e)).into())
        };

        let mut out = match File::create(&output_file[..]) {
            Ok(o) => o,
            Err(e) => return Err(DacpacErrorKind::GenerationError(format!("Failed to generate report: {}", e)).into())
        };
        match out.write_all(json.as_bytes()) {
            Ok(_) => {},
            Err(e) => return Err(DacpacErrorKind::GenerationError(format!("Failed to generate report: {}", e)).into())
        }

        Ok(())
    }

    fn load_project(source_dacpac_file: String) -> DacpacResult<Project> {
        // Load the DACPAC
        let source_path = Path::new(&source_dacpac_file[..]);
        if !source_path.is_file() {
            return Err(DacpacErrorKind::IOError(format!("{}", source_path.display()), "DACPAC file does not exist".to_owned()).into())
        }
        let file = match fs::File::open(&source_path) {
            Ok(o) => o,
            Err(e) => return Err(DacpacErrorKind::IOError(format!("{}", source_path.display()), format!("Failed to open DACPAC file: {}", e)).into())
        };
        let mut archive = match ZipArchive::new(file) {
            Ok(o) => o,
            Err(e) => return Err(DacpacErrorKind::IOError(format!("{}", source_path.display()), format!("Failed to open DACPAC file: {}", e)).into())
        };

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
            if file.name().starts_with("extensions/") {
                load_file!(ExtensionDefinition, extensions, file);
            } else if file.name().starts_with("functions/") {
                load_file!(FunctionDefinition, functions, file);
            } else if file.name().starts_with("schemas/") {
                load_file!(SchemaDefinition, schemas, file);
            } else if file.name().starts_with("scripts/") {
                load_file!(ScriptDefinition, scripts, file);
            } else if file.name().starts_with("tables/") {
                load_file!(TableDefinition, tables, file);
            } else if file.name().starts_with("types/") {
                load_file!(TypeDefinition, types, file);
            } else if file.name().eq("order.json") {
                let mut contents = String::new();
                file.read_to_string(&mut contents).unwrap();
                let nodes : Vec<Node> = serde_json::from_str(&contents).unwrap();
                order = Some(nodes);
            }
        }

        Ok(Project {
            extensions: extensions,
            functions: functions,
            schemas: schemas,
            scripts: scripts,
            tables: tables,
            types: types,
            order: order,
        })
    }

    fn load_publish_profile(publish_profile: String) -> DacpacResult<PublishProfile> {
        // Load the publish profile
        let path = Path::new(&publish_profile[..]);
        if !path.is_file() {
            return Err(DacpacErrorKind::IOError(format!("{}", path.display()), "Publish profile does not exist".to_owned()).into());
        }
        let mut publish_profile_raw = String::new();
        if let Err(err) = File::open(&path).and_then(|mut f| f.read_to_string(&mut publish_profile_raw)) {
            return Err(DacpacErrorKind::IOError(format!("{}", path.display()), format!("Failed to read publish profile: {}", err)).into());
        }

        // Deserialize
        let publish_profile : PublishProfile = match serde_json::from_str(&publish_profile_raw) {
            Ok(p) => p,
            Err(e) => return Err(DacpacErrorKind::FormatError(format!("{}", path.display()), format!("Publish profile was not well formed: {}", e)).into())
        };
        Ok(publish_profile)
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
        let name = self.name.clone();
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

#[derive(Deserialize)]
struct ProjectConfig {
    version: String,
    #[serde(rename = "defaultSchema")]
    default_schema: String,
    #[serde(rename = "preDeployScripts")]
    pre_deploy_scripts: Vec<String>,
    #[serde(rename = "postDeployScripts")]
    post_deploy_scripts: Vec<String>,
}

enum DbObject<'a> {
    Extension(&'a ExtensionDefinition), // 2
    Function(&'a FunctionDefinition), // 6 (ordered)
    Schema(&'a SchemaDefinition), // 3
    Script(&'a ScriptDefinition), // 1, 7
    Table(&'a TableDefinition), // 5 (ordered)
    Type(&'a TypeDefinition), // 4
}

struct Project {
    extensions: Vec<ExtensionDefinition>,
    functions: Vec<FunctionDefinition>,
    schemas: Vec<SchemaDefinition>,
    scripts: Vec<ScriptDefinition>,
    tables: Vec<TableDefinition>,
    types: Vec<TypeDefinition>,
    order: Option<Vec<Node>>,
}

impl Project {

    fn new() -> Self {
        Project {
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

    fn set_defaults(&mut self, config: &ProjectConfig) {

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
                table.name.schema = Some(config.default_schema.clone());
            }
            if let Some(ref mut constraints) = table.constraints {
                for constraint in constraints.iter_mut() {
                    if let TableConstraint::Foreign { ref mut ref_table, .. } = *constraint {
                        if ref_table.schema.is_none() {
                            ref_table.schema = Some(config.default_schema.clone());
                        }
                    }
                }
            }
        }
    }

    fn generate_dependency_graph(&mut self) -> DacpacResult<()> {

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
            DependencyGraphValidationResult::CircularReference => return Err(DacpacErrorKind::GenerationError("Circular reference detected".to_owned()).into()),
            // TODO: List out unresolved references
            DependencyGraphValidationResult::UnresolvedDependencies => return Err(DacpacErrorKind::GenerationError("Unresolved dependencies detected".to_owned()).into()),
        }

        // Then generate the order
        let order = graph.topological_sort();
        // Should we also add schema etc in there? Not really necessary...
        self.order = Some(order);
        Ok(())
    }

    fn validate(&self) -> DacpacResult<()> {

        // TODO: Validate references etc
        Ok(())
    }

    fn generate_changeset(&self, connection: &Connection, publish_profile: PublishProfile) -> DacpacResult<Vec<ChangeInstruction>> {

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
                        // TODO: Figure out if it exists and drop if necessary
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

#[derive(Deserialize)]
struct PublishProfile {
    version: String,
    #[serde(rename = "alwaysRecreateDatabase")]
    always_recreate_database: bool,
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
                instr.push_str(&format!("/* {} */\n", script.name)[..]);
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
