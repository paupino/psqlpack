use std::fmt;
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::str::FromStr;

use chrono::prelude::*;
use glob::glob;
use slog::Logger;
use zip::write::FileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::connection::Connection;
use crate::errors::PsqlpackErrorKind::*;
use crate::errors::{PsqlpackResult, PsqlpackResultExt};
use crate::model::{Capabilities, DefinableCatalog, Dependency, Project};
use crate::semver::Semver;
use crate::sql::ast::*;

macro_rules! ztry {
    ($expr:expr) => {{
        match $expr {
            Ok(_) => {}
            Err(e) => bail!(GenerationError(format!("Failed to write package: {}", e))),
        }
    }};
}

macro_rules! zip_collection {
    ($zip:ident, $package:ident, $collection:ident) => {{
        let collection_name = stringify!($collection);
        ztry!($zip.add_directory(format!("{}/", collection_name), FileOptions::default()));
        for item in &$package.$collection {
            ztry!($zip.start_file(
                format!("{}/{}.json", collection_name, item.name),
                FileOptions::default()
            ));
            let json = match serde_json::to_string_pretty(&item) {
                Ok(j) => j,
                Err(e) => bail!(GenerationError(format!("Failed to write package: {}", e))),
            };
            ztry!($zip.write_all(json.as_bytes()));
        }
    }};
}

// Search paths for extensions
const DEFAULT_SEARCH_PATHS: [&str; 2] = ["./lib", "~/.psqlpack/lib"];

#[derive(Debug)]
pub struct Package {
    pub meta: MetaInfo,
    pub extensions: Vec<Dependency>,
    pub functions: Vec<FunctionDefinition>,
    pub indexes: Vec<IndexDefinition>,
    pub schemas: Vec<SchemaDefinition>,
    pub scripts: Vec<ScriptDefinition>,
    pub tables: Vec<TableDefinition>,
    pub types: Vec<TypeDefinition>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetaInfo {
    version: Semver,
    generated_at: DateTime<Utc>,
    source: SourceInfo,
    publishable: bool,
}

impl MetaInfo {
    pub fn new(source: SourceInfo) -> Self {
        let publishable = !matches!(source, SourceInfo::Extension(..));
        MetaInfo {
            version: crate_version(),
            generated_at: Utc::now(),
            source,
            publishable,
        }
    }
}

fn crate_version() -> Semver {
    Semver::from_str(&format!(
        "{}.{}.{}",
        env!("CARGO_PKG_VERSION_MAJOR"),
        env!("CARGO_PKG_VERSION_MINOR"),
        env!("CARGO_PKG_VERSION_PATCH")
    ))
    .unwrap()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SourceInfo {
    Database,
    Extension(String),
    Project,
}

impl fmt::Display for SourceInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SourceInfo::Database => write!(f, "database"),
            SourceInfo::Extension(ref name) => write!(f, "extension {}", name),
            SourceInfo::Project => write!(f, "project"),
        }
    }
}

impl Package {
    fn maybe_packaged_file(source_path: &Path) -> PsqlpackResult<bool> {
        File::open(&source_path)
            .chain_err(|| IOError(source_path.to_str().unwrap().into(), "Failed to open file".into()))
            .and_then(|file| {
                let mut reader = BufReader::with_capacity(4, file);
                let mut buffer = [0; 2];
                let b = reader.read(&mut buffer[..]).map_err(|e| {
                    IOError(
                        source_path.to_str().unwrap().into(),
                        format!("Failed to read file: {}", e),
                    )
                })?;
                if b != 2 {
                    bail!(IOError(
                        source_path.to_str().unwrap().into(),
                        "Invalid file provide (< 4 bytes)".into()
                    ));
                }

                Ok(buffer[0] == 0x50 && buffer[1] == 0x4B)
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
            project.build_package(&log)
        }
    }

    pub fn from_packaged_file(log: &Logger, source_path: &Path) -> PsqlpackResult<Package> {
        let _log = log.new(o!("package" => "from_packaged_file"));
        let mut archive = File::open(&source_path)
            .chain_err(|| PackageReadError(source_path.to_path_buf()))
            .and_then(|file| ZipArchive::new(file).chain_err(|| PackageUnarchiveError(source_path.to_path_buf())))?;

        let mut meta: Option<MetaInfo> = None;
        let mut extensions = Vec::new();
        let mut functions = Vec::new();
        let mut indexes = Vec::new();
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
            if name.starts_with("meta") {
                if meta.is_some() {
                    bail!(PackageReadError(source_path.to_path_buf()));
                }
                let m = serde_json::from_reader(file).chain_err(|| PackageInternalReadError(name))?;
                meta = Some(m);
            } else if name.starts_with("extensions/") {
                extensions.push(serde_json::from_reader(file).chain_err(|| PackageInternalReadError(name))?);
            } else if name.starts_with("functions/") {
                functions.push(serde_json::from_reader(file).chain_err(|| PackageInternalReadError(name))?);
            } else if name.starts_with("indexes") {
                indexes.push(serde_json::from_reader(file).chain_err(|| PackageInternalReadError(name))?);
            } else if name.starts_with("schemas/") {
                schemas.push(serde_json::from_reader(file).chain_err(|| PackageInternalReadError(name))?);
            } else if name.starts_with("scripts/") {
                scripts.push(serde_json::from_reader(file).chain_err(|| PackageInternalReadError(name))?);
            } else if name.starts_with("tables/") {
                tables.push(serde_json::from_reader(file).chain_err(|| PackageInternalReadError(name))?);
            } else if name.starts_with("types/") {
                types.push(serde_json::from_reader(file).chain_err(|| PackageInternalReadError(name))?);
            }
        }

        let mut package = Package {
            meta: match meta {
                Some(m) => m,
                // Temporary - this will be an error in the future
                // For now, it assumes a standard project
                None => MetaInfo::new(SourceInfo::Project),
            },
            extensions,
            functions,
            indexes,
            schemas,
            scripts,
            tables,
            types,
        };
        package.promote_primary_keys_to_table_constraints();
        Ok(package)
    }

    pub fn from_connection(
        log: &Logger,
        connection: &Connection,
        capabilities: &Capabilities,
    ) -> PsqlpackResult<Option<Package>> {
        let log = log.new(o!("package" => "from_connection"));

        trace!(log, "Checking for database `{}`", connection.database());
        if !capabilities.database_exists {
            return Ok(None);
        }

        // We do a few SQL queries to get the package details
        trace!(log, "Connecting to database");
        let mut client = connection.connect_database()?;

        let extensions = capabilities
            .extensions
            .iter()
            .filter(|e| e.installed)
            .map(|e| Dependency {
                name: e.name.clone(),
                version: Some(e.version),
            })
            .collect::<Vec<_>>();

        // TODO: Refactor connection so we only need to pass through that
        let schemas = capabilities.schemata(&mut client, connection.database())?;
        let types = capabilities.types(&mut client)?;
        let functions = capabilities.functions(&mut client)?;
        let tables = capabilities.tables(&mut client)?;
        let indexes = capabilities.indexes(&mut client)?;

        let mut package = Package {
            meta: MetaInfo::new(SourceInfo::Database),
            extensions,
            functions,
            indexes,
            schemas,
            scripts: Vec::new(), // Scripts can't be known from a connection
            tables,
            types,
        };
        package.promote_primary_keys_to_table_constraints();

        Ok(Some(package))
    }

    pub fn write_to(&self, destination: &Path) -> PsqlpackResult<()> {
        if let Some(parent) = destination.parent() {
            match fs::create_dir_all(parent) {
                Ok(_) => {}
                Err(e) => bail!(GenerationError(format!("Failed to create package directory: {}", e))),
            }
        }

        File::create(&destination)
            .chain_err(|| GenerationError("Failed to write package".to_owned()))
            .and_then(|output_file| {
                let mut zip = ZipWriter::new(output_file);

                ztry!(zip.start_file("meta.json", FileOptions::default()));
                let json = match serde_json::to_string_pretty(&self.meta) {
                    Ok(j) => j,
                    Err(e) => bail!(GenerationError(format!("Failed to write package: {}", e))),
                };
                ztry!(zip.write_all(json.as_bytes()));
                zip_collection!(zip, self, extensions);
                zip_collection!(zip, self, functions);
                zip_collection!(zip, self, indexes);
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
            // By default, our source is a project file
            meta: MetaInfo::new(SourceInfo::Project),
            extensions: Vec::new(),
            functions: Vec::new(),
            indexes: Vec::new(),
            schemas: Vec::new(),
            scripts: Vec::new(),
            tables: Vec::new(),
            types: Vec::new(),
        }
    }

    pub fn push_extension(&mut self, extension: Dependency) {
        self.extensions.push(extension);
    }

    pub fn push_function(&mut self, function: FunctionDefinition) {
        self.functions.push(function);
    }

    pub fn push_index(&mut self, index: IndexDefinition) {
        self.indexes.push(index);
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
            if project.default_schema.eq_ignore_ascii_case(&schema.name[..]) {
                has_public = true;
                break;
            }
        }
        if !has_public {
            self.schemas.push(SchemaDefinition {
                name: project.default_schema.to_owned(),
            });
        }
        for typ in &mut self.types {
            if typ.name.schema.is_none() {
                typ.name.schema = Some(project.default_schema.clone());
            }
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
                let pk = column
                    .constraints
                    .iter()
                    .position(|c| c.eq(&ColumnConstraint::PrimaryKey));
                if pk.is_some() {
                    // Make sure it is not null
                    ensure_not_null_column(column);
                }

                // Also, if the type is custom, then assume the default search path
                if let SqlType::Custom(ref mut custom_type, ref _opts, _dim) = column.sql_type {
                    if custom_type.schema.is_none() {
                        custom_type.schema = Some(project.default_schema.clone());
                    }
                }
            }
        }

        // Set missing schema's and default values in indexes
        for index in &mut self.indexes {
            // Set default schema
            if index.table.schema.is_none() {
                index.table.schema = Some(project.default_schema.clone());
            }

            // Set default storage type
            if index.index_type.is_none() {
                index.index_type = Some(IndexType::BTree);
            }

            // Set default column sorts
            for col in &mut index.columns {
                if col.order.is_none() {
                    col.order = Some(IndexOrder::Ascending);
                }
                if col.null_position.is_none() {
                    if let Some(ref order) = col.order {
                        col.null_position = Some(match order {
                            IndexOrder::Ascending => IndexPosition::Last,
                            IndexOrder::Descending => IndexPosition::First,
                        });
                    }
                }
            }
        }

        // We also do the promotion here
        self.promote_primary_keys_to_table_constraints();
    }

    pub fn promote_primary_keys_to_table_constraints(&mut self) {
        // Set default schema's as well as marking primary key columns as not null
        for table in &mut self.tables {
            // Primary keys may also be specified against the column directly. We promote these to table constraints.`
            for column in table.columns.iter_mut() {
                let pk_pos = column
                    .constraints
                    .iter()
                    .position(|c| c.eq(&ColumnConstraint::PrimaryKey));
                if let Some(pk_pos) = pk_pos {
                    // Remove the PK constraint
                    column.constraints.remove(pk_pos);

                    // Add a table constraint if it doesn't exist
                    let found = table
                        .constraints
                        .iter()
                        .position(|c| matches!(c, TableConstraint::Primary { .. }));
                    if found.is_none() {
                        let name = format!("{}_pkey", table.name.name);
                        table.constraints.push(TableConstraint::Primary {
                            name,
                            columns: vec![column.name.to_owned()],
                            parameters: None,
                        });
                    }
                }
            }
        }
    }

    pub fn generate_dependency_graph(&self, log: &Logger) -> PsqlpackResult<Vec<Node>> {
        let log = log.new(o!("graph" => "generate"));

        let mut graph = Graph::new();

        // Go through and add each object and add it to the graph
        // Schemas and types are always implied
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
                constraint.graph(&log, &mut graph, Some(&table_node));
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

    // TODO: Stop moving string, consider making this a utility
    fn expand_tilde(input: &str) -> String {
        if let Some(after_tilde) = input.strip_prefix('~') {
            if after_tilde.is_empty() || after_tilde.starts_with('/') {
                if let Some(hd) = dirs::home_dir() {
                    format!("{}{}", hd.display(), after_tilde)
                } else {
                    input.into()
                }
            } else {
                input.into()
            }
        } else {
            input.into()
        }
    }

    pub fn load_references(&self, project: &Project, log: &Logger) -> Vec<Package> {
        let log = log.new(o!("package" => "load_references"));

        let mut references = Vec::new();

        // Get the search paths to look for. We favor project level paths first if they exist.
        trace!(log, "Setting up search paths");
        let mut search_paths = Vec::new();
        if let Some(ref user_search_paths) = project.reference_search_paths {
            for path in user_search_paths {
                if let Ok(p) = Path::new(&Self::expand_tilde(path)).canonicalize() {
                    search_paths.push(p);
                } else {
                    warn!(log, "Path not found: {}", path);
                }
            }
        }
        for path in &DEFAULT_SEARCH_PATHS {
            if let Ok(p) = Path::new(&Self::expand_tilde(path)).canonicalize() {
                search_paths.push(p);
            } else {
                warn!(log, "Path not found: {}", path);
            }
        }

        // Firstly, load extensions
        trace!(log, "Loading extensions");

        for extension in &self.extensions {
            let mut found = false;
            for path in &search_paths {
                if let Some(version) = extension.version {
                    // If the extension specifies a version we search for name-version.psqlpack.
                    let mut path = path.to_path_buf();
                    path.push(format!("{}-{}.psqlpack", extension.name, version));
                    if path.exists() && path.is_file() {
                        match Package::from_packaged_file(&log, &path) {
                            Ok(package) => {
                                references.push(package);
                                found = true;
                                break;
                            }
                            Err(e) => {
                                error!(log, "Failed to load extension: {}", e);
                                break;
                            }
                        }
                    }
                } else {
                    // If no version is specified then we search for either the non-versioned extension
                    // or the highest versioned extension.
                    // Try to find the globs matching ext_name*.psqlpack
                    let mut search_path = path.to_path_buf();
                    search_path.push(format!("{}*.psqlpack", extension.name));
                    let search_path = search_path.to_str().unwrap();
                    let mut found_packages = Vec::new();
                    for glob_path in glob(search_path).unwrap() {
                        if let Ok(path) = glob_path {
                            if path.is_file() {
                                match Package::from_packaged_file(&log, &path) {
                                    Ok(package) => {
                                        trace!(log, "Found {} {}", package.meta.source, package.meta.version);
                                        found_packages.push(package);
                                    }
                                    Err(e) => {
                                        error!(log, "Failed to load extension: {}", e);
                                    }
                                }
                            }
                        } else {
                            error!(log, "Glob result had an error: {}", glob_path.err().unwrap().error());
                        }
                    }
                    if found_packages.is_empty() {
                        trace!(log, "No packages matched: {}", search_path);
                        break;
                    } else if found_packages.len() > 1 {
                        trace!(log, "Search for highest version");
                        found_packages.sort_by(|a, b| a.meta.version.cmp(&b.meta.version));
                        references.extend(found_packages.drain(..1));
                        break;
                    } else {
                        // Only one item in there so just drain and extend
                        references.extend(found_packages.drain(..));
                        found = true;
                        break;
                    }
                }
            }

            if !found {
                warn!(log, "Extension not found: {}", extension);
            }
        }

        // TODO: load passed references
        references
    }

    pub fn validate(&self, references: &[Package]) -> PsqlpackResult<()> {
        // 1. Validate schema existence
        let schemata = self.schemas.iter().map(|schema| &schema.name[..]).collect::<Vec<_>>();
        let names = self
            .tables
            .iter()
            .map(|t| &t.name)
            .chain(self.functions.iter().map(|f| &f.name))
            .collect::<Vec<_>>();
        let mut errors = names
            .iter()
            .filter(|o| {
                if let Some(ref s) = o.schema {
                    !schemata.contains(&&s[..])
                } else {
                    false
                }
            })
            .map(|o| ValidationKind::SchemaMissing {
                schema: o.schema.clone().unwrap(),
                object: o.name.to_owned(),
            })
            .collect::<Vec<_>>();

        // 2. Validate custom type are known
        let mut custom_types = self.types.iter().map(|ty| &ty.name).collect::<Vec<_>>();
        for reference in references {
            for ty in &reference.types {
                custom_types.push(&ty.name);
            }
        }
        errors.extend(self.tables.iter().flat_map(|t| {
            t.columns
                .iter()
                .filter_map(|c| match c.sql_type {
                    SqlType::Custom(ref name, ref _opts, _dim) => {
                        if !custom_types.contains(&&name) {
                            Some(ValidationKind::UnknownType {
                                ty: name.to_owned(),
                                table: t.name.to_string(),
                            })
                        } else {
                            None
                        }
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        }));

        // 3. Validate constraints map to known tables
        let foreign_keys = self
            .tables
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
                .filter(|&&(_, _, ref table, _)| !self.tables.iter().any(|t| t.name.eq(table)))
                .map(
                    |&(ref name, _, ref table, _)| ValidationKind::TableConstraintInvalidReferenceTable {
                        constraint: name.to_owned(),
                        table: table.to_string(),
                    },
                ),
        );
        // ii. Reference table exists, but the reference column doesn't.
        errors.extend(
            foreign_keys
                .iter()
                .filter(|&&(_, _, ref table, ref columns)| {
                    let table = self.tables.iter().find(|t| t.name.eq(table));
                    match table {
                        Some(t) => !columns.iter().all(|rc| t.columns.iter().any(|c| c.name.eq(rc))),
                        None => false,
                    }
                })
                .map(
                    |&(ref name, _, ref table, ref columns)| ValidationKind::TableConstraintInvalidReferenceColumns {
                        constraint: name.to_owned(),
                        table: table.to_string(),
                        columns: columns.clone(),
                    },
                ),
        );
        // iii. Source column doesn't exist
        errors.extend(
            foreign_keys
                .iter()
                .filter(|&&(ref constraint, ref columns, _, _)| {
                    let table = self
                        .tables
                        .iter()
                        .find(|t| t.constraints.iter().any(|c| c.name() == constraint));
                    match table {
                        Some(t) => !columns.iter().all(|rc| t.columns.iter().any(|c| c.name.eq(rc))),
                        None => false,
                    }
                })
                .map(
                    |&(ref name, ref columns, _, _)| ValidationKind::TableConstraintInvalidSourceColumns {
                        constraint: name.to_owned(),
                        columns: columns.clone(),
                    },
                ),
        );
        // iv. (Future) Source column match type is not compatible with reference column type

        // 4. Validate indexes map to known tables
        // i. reference table missing
        errors.extend(
            self.indexes
                .iter()
                .filter(|&index| !self.tables.iter().any(|t| t.name.eq(&index.table)))
                .map(|ref index| ValidationKind::IndexInvalidReferenceTable {
                    index: index.name.to_string(),
                    table: index.table.to_string(),
                }),
        );
        // ii. reference table exists but columns missing
        errors.extend(
            self.indexes
                .iter()
                .filter(|&index| {
                    let table = self.tables.iter().find(|t| t.name.eq(&index.table));
                    match table {
                        Some(t) => !index
                            .columns
                            .iter()
                            .all(|rc| t.columns.iter().any(|c| c.name.eq(&rc.name))),
                        None => false,
                    }
                })
                .map(|ref index| ValidationKind::IndexInvalidReferenceColumns {
                    index: index.name.to_string(),
                    table: index.table.to_string(),
                    columns: index.columns.iter().map(|c| c.name.to_string()).collect(),
                }),
        );

        // 5. Validate function languages. For now, custom languages aren't supported
        // until we can validate them.
        errors.extend(
            self.functions
                .iter()
                .filter(|&function| matches!(function.language, FunctionLanguage::Custom(_)))
                .map(|ref function| ValidationKind::UnsupportedFunctionLanguage {
                    language: function.language.clone(),
                    name: function.name.clone(),
                }),
        );

        // If there are no errors then we're "ok"
        if errors.is_empty() {
            Ok(())
        } else {
            bail!(ValidationError(errors))
        }
    }
}

impl Default for Package {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum ValidationKind {
    IndexInvalidReferenceTable {
        index: String,
        table: String,
    },
    IndexInvalidReferenceColumns {
        index: String,
        table: String,
        columns: Vec<String>,
    },
    TableConstraintInvalidReferenceTable {
        constraint: String,
        table: String,
    },
    TableConstraintInvalidReferenceColumns {
        constraint: String,
        table: String,
        columns: Vec<String>,
    },
    TableConstraintInvalidSourceColumns {
        constraint: String,
        columns: Vec<String>,
    },
    SchemaMissing {
        schema: String,
        object: String,
    },
    UnknownType {
        ty: ObjectName,
        table: String,
    },
    UnsupportedFunctionLanguage {
        language: FunctionLanguage,
        name: ObjectName,
    },
}

impl fmt::Display for ValidationKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ValidationKind::IndexInvalidReferenceTable { ref index, ref table } => {
                write!(f, "Index `{}` uses unknown reference table `{}`", index, table)
            }
            ValidationKind::IndexInvalidReferenceColumns {
                ref index,
                ref table,
                ref columns,
            } => write!(
                f,
                "Index `{}` uses unknown reference column(s) on table `{}` (`{}`)",
                index,
                table,
                columns.join("`, `")
            ),
            ValidationKind::TableConstraintInvalidReferenceTable {
                ref constraint,
                ref table,
            } => write!(
                f,
                "Foreign Key constraint `{}` uses unknown reference table `{}`",
                constraint, table
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
            ValidationKind::SchemaMissing { ref schema, ref object } => {
                write!(f, "Schema `{}` missing for object `{}`", schema, object)
            }
            ValidationKind::UnknownType { ref ty, ref table } => {
                write!(f, "Unknown type `{}` used on table `{}`", ty, table)
            }
            ValidationKind::UnsupportedFunctionLanguage { ref language, ref name } => write!(
                f,
                "Unsupported function language `{}` used on function `{}`",
                language, name,
            ),
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
            Node::Constraint(table, constraint) => {
                write!(f, "Constraint: {}.{}", table.name.to_string(), constraint.name())
            }
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
        let table_node = *parent.unwrap();
        let table = match table_node {
            Node::Table(table) => table,
            _ => panic!("Non table parent for column."),
        };
        match *self {
            TableConstraint::Primary {
                ref name, ref columns, ..
            } => {
                let log = log.new(o!("primary constraint" => name.to_owned()));
                // Primary relies on the columns existing (of course)
                trace!(log, "Adding");
                let constraint = graph.add_node(Node::Constraint(table, self));
                for column_name in columns {
                    trace!(log, "Adding edge to column"; "column" => &column_name);
                    let column = table.columns.iter().find(|x| &x.name == column_name).unwrap();
                    graph.add_edge(Node::Column(table, column), constraint, ());
                }
                graph.add_edge(table_node, constraint, ());
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
                    trace!(log, "Adding edge to refrenced column";
                                "table" => ref_table.to_string(),
                                "column" => &ref_column_name);

                    let ref_column = table_def.columns.iter().find(|x| &x.name == ref_column_name).unwrap();
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
                graph.add_edge(table_node, constraint, ());
                constraint
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::errors::PsqlpackError;
    use crate::errors::PsqlpackErrorKind::*;
    use crate::model::*;
    use crate::sql::parser::StatementListParser;
    use crate::sql::{ast, lexer};

    use slog::{Discard, Drain, Logger};
    use spectral::prelude::*;

    fn package_sql(sql: &str) -> Package {
        let tokens = match lexer::tokenize_stmt(sql) {
            Ok(t) => t,
            Err(e) => panic!("Syntax error: {}", e.line),
        };
        let mut package = Package::new();
        match StatementListParser::new().parse(tokens) {
            Ok(statement_list) => {
                for statement in statement_list {
                    match statement {
                        ast::Statement::Error(kind) => panic!("Unhandled error detected: {}", kind),
                        ast::Statement::Function(function_definition) => package.push_function(function_definition),
                        ast::Statement::Index(index_definition) => package.push_index(index_definition),
                        ast::Statement::Schema(schema_definition) => package.push_schema(schema_definition),
                        ast::Statement::Table(table_definition) => package.push_table(table_definition),
                        ast::Statement::Type(type_definition) => package.push_type(type_definition),
                    }
                }
            }
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
                }
                _ => panic!("Expected a table at index {}", $index),
            }
        };
    }

    macro_rules! assert_column {
        ($graph:ident,$index:expr,$table_name:expr,$column_name:expr) => {
            match $graph[$index] {
                Node::Column(table, column) => {
                    assert_that!(table.name.to_string()).is_equal_to($table_name.to_owned());
                    assert_that!(column.name.to_string()).is_equal_to($column_name.to_owned());
                }
                _ => panic!("Expected a column at index {}", $index),
            }
        };
    }

    macro_rules! assert_pk_constraint {
        ($graph:ident,$index:expr,$table_name:expr,$constraint_name:expr) => {
            match $graph[$index] {
                Node::Constraint(table, constraint) => {
                    assert_that!(table.name.to_string()).is_equal_to($table_name.to_owned());
                    match *constraint {
                        ast::TableConstraint::Primary { ref name, .. } => {
                            assert_that!(name.to_string()).is_equal_to($constraint_name.to_owned());
                        }
                        _ => panic!("Expected a primary key constraint at index {}", $index),
                    }
                }
                _ => panic!("Expected a constraint at index {}", $index),
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
                        }
                        _ => panic!("Expected a foreign key constraint at index {}", $index),
                    }
                }
                _ => panic!("Expected a constraint at index {}", $index),
            }
        };
    }

    #[test]
    fn it_sets_table_defaults() {
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
    fn it_sets_index_defaults() {
        let mut package = package_sql("CREATE INDEX idx_person_name ON person(name);");
        let project = Project::default();

        // Pre-condition checks
        {
            assert_that!(package.schemas).is_empty();
            assert_that!(package.indexes).has_length(1);
            let index = &package.indexes[0];
            assert_that!(index.table.schema).is_none();
            assert_that!(index.table.name).is_equal_to("person".to_owned());
            assert_that!(index.columns).has_length(1);
            let col = &index.columns[0];
            assert_that!(col.name).is_equal_to("name".to_owned());
            assert_that!(col.order).is_none();
            assert_that!(col.null_position).is_none();
        }

        // Set the defaults and assert again
        package.set_defaults(&project);
        assert_that!(package.schemas).has_length(1);
        assert_that!(package.indexes).has_length(1);
        let schema = &package.schemas[0];
        assert_that!(schema.name).is_equal_to("public".to_owned());
        let index = &package.indexes[0];
        assert_that!(index.table.schema)
            .is_some()
            .is_equal_to("public".to_owned());
        assert_that!(index.table.name).is_equal_to("person".to_owned());
        assert_that!(index.columns).has_length(1);
        let col = &index.columns[0];
        assert_that!(col.name).is_equal_to("name".to_owned());
        assert_that!(col.order)
            .is_some()
            .is_equal_to(ast::IndexOrder::Ascending);
        assert_that!(col.null_position)
            .is_some()
            .is_equal_to(ast::IndexPosition::Last);
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
    fn it_generates_a_complex_ordering_1() {
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
    fn it_generates_a_complex_ordering_2() {
        let package = package_sql(
            "CREATE TABLE public.allocation (
                id              serial                NOT NULL,
                CONSTRAINT pk_public_allocation PRIMARY KEY (id)
            );
            CREATE TABLE public.transaction (
                id                serial                NOT NULL,
                allocation_id     int                   NOT NULL,
                CONSTRAINT pk_public_transaction PRIMARY KEY (id),
                CONSTRAINT fk_public_transaction__allocation_id FOREIGN KEY (allocation_id)
                REFERENCES public.allocation (id) MATCH SIMPLE
                ON UPDATE NO ACTION ON DELETE NO ACTION
            );",
        );
        let logger = empty_logger();
        let graph = package.generate_dependency_graph(&logger);

        // Make sure we generated enough nodes (two tables + three columns + three constraints).
        assert_that!(graph).is_ok().has_length(8);
        let graph = graph.unwrap();
        assert_table!(graph, 0, "public.transaction");
        assert_column!(graph, 1, "public.transaction", "id");
        assert_column!(graph, 2, "public.transaction", "allocation_id");
        assert_pk_constraint!(graph, 3, "public.transaction", "pk_public_transaction");
        assert_table!(graph, 4, "public.allocation");
        assert_column!(graph, 5, "public.allocation", "id");
        assert_pk_constraint!(graph, 6, "public.allocation", "pk_public_allocation");
        // FK is last
        assert_fk_constraint!(graph, 7, "public.transaction", "fk_public_transaction__allocation_id");
    }

    #[test]
    fn it_validates_missing_schema_references() {
        let mut package = package_sql("CREATE TABLE my.items(id int);");
        let result = package.validate(&Vec::new());

        // `my` schema is missing
        assert_that!(result).is_err();
        let validation_errors = match result.err().unwrap() {
            PsqlpackError(ValidationError(errors), _) => errors,
            unexpected => panic!("Expected validation error however saw {:?}", unexpected),
        };
        assert_that!(validation_errors).has_length(1);
        match validation_errors[0] {
            ValidationKind::SchemaMissing { ref schema, ref object } => {
                assert_that!(*schema).is_equal_to("my".to_owned());
                assert_that!(*object).is_equal_to("items".to_owned());
            }
            ref unexpected => panic!("Unexpected validation type: {:?}", unexpected),
        }

        // Add the schema and try again
        package.schemas.push(ast::SchemaDefinition { name: "my".to_owned() });
        assert_that!(package.validate(&Vec::new())).is_ok();
    }

    #[test]
    fn it_validates_unknown_types() {
        let mut package = package_sql(
            "CREATE SCHEMA my;
             CREATE TABLE my.items(id mytype);",
        );
        let project = Project::default();
        package.set_defaults(&project);
        let result = package.validate(&Vec::new());

        // `mytype` is missing
        assert_that!(result).is_err();
        let validation_errors = match result.err().unwrap() {
            PsqlpackError(ValidationError(errors), _) => errors,
            unexpected => panic!("Expected validation error however saw {:?}", unexpected),
        };
        assert_that!(validation_errors).has_length(1);
        match validation_errors[0] {
            ValidationKind::UnknownType { ref ty, ref table } => {
                assert_that!(*ty).is_equal_to(ast::ObjectName {
                    schema: Some("public".to_string()),
                    name: "mytype".to_string(),
                });
                assert_that!(*table).is_equal_to("my.items".to_owned());
            }
            ref unexpected => panic!("Unexpected validation type: {:?}", unexpected),
        }

        // Add the type and try again
        package.types.push(ast::TypeDefinition {
            name: ast::ObjectName {
                schema: Some("public".to_string()),
                name: "mytype".to_string(),
            },
            kind: ast::TypeDefinitionKind::Enum(Vec::new()),
        });
        assert_that!(package.validate(&Vec::new())).is_ok();
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
        let result = package.validate(&Vec::new());

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
            columns: vec![ast::ColumnDefinition {
                name: "id".to_owned(),
                sql_type: ast::SqlType::Simple(ast::SimpleSqlType::Serial, None),
                constraints: Vec::new(),
            }],
            constraints: Vec::new(),
        });
        assert_that!(package.validate(&Vec::new())).is_ok();
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
        let result = package.validate(&Vec::new());

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
            let parent = package.tables.iter_mut().find(|t| t.name.name.eq("parent")).unwrap();
            parent.columns.push(ast::ColumnDefinition {
                name: "parent_id".to_owned(),
                sql_type: ast::SqlType::Simple(ast::SimpleSqlType::Integer, None),
                constraints: Vec::new(),
            });
        }
        assert_that!(package.validate(&Vec::new())).is_ok();
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
        let result = package.validate(&Vec::new());

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
            let child = package.tables.iter_mut().find(|t| t.name.name.eq("child")).unwrap();
            child.columns.push(ast::ColumnDefinition {
                name: "par_id".to_owned(),
                sql_type: ast::SqlType::Simple(ast::SimpleSqlType::Integer, None),
                constraints: Vec::new(),
            });
        }
        assert_that!(package.validate(&Vec::new())).is_ok();
    }

    #[test]
    fn it_validates_missing_reference_table_in_index() {
        let mut package = package_sql(
            "CREATE SCHEMA my;
             CREATE TABLE my.person(id int, name varchar(50));
             CREATE UNIQUE INDEX idx_company_name ON my.company (name);",
        );
        let result = package.validate(&Vec::new());

        // `my.company` does not exist
        assert_that!(result).is_err();
        let validation_errors = match result.err().unwrap() {
            PsqlpackError(ValidationError(errors), _) => errors,
            unexpected => panic!("Expected validation error however saw {:?}", unexpected),
        };
        assert_that!(validation_errors).has_length(1);
        match validation_errors[0] {
            ValidationKind::IndexInvalidReferenceTable { ref index, ref table } => {
                assert_that!(*index).is_equal_to("idx_company_name".to_owned());
                assert_that!(*table).is_equal_to("my.company".to_owned());
            }
            ref unexpected => panic!("Unexpected validation type: {:?}", unexpected),
        }

        // Add the table and try again
        package.tables.push(ast::TableDefinition {
            name: ast::ObjectName {
                schema: Some("my".to_owned()),
                name: "company".to_owned(),
            },
            columns: vec![
                ast::ColumnDefinition {
                    name: "id".to_owned(),
                    sql_type: ast::SqlType::Simple(ast::SimpleSqlType::Serial, None),
                    constraints: Vec::new(),
                },
                ast::ColumnDefinition {
                    name: "name".to_owned(),
                    sql_type: ast::SqlType::Simple(ast::SimpleSqlType::VariableLengthString(50), None),
                    constraints: Vec::new(),
                },
            ],
            constraints: Vec::new(),
        });
        assert_that!(package.validate(&Vec::new())).is_ok();
    }

    #[test]
    fn it_validates_missing_reference_column_in_index() {
        let mut package = package_sql(
            "CREATE SCHEMA my;
             CREATE TABLE my.person(id int, name varchar(50));
             CREATE UNIQUE INDEX idx_person_number ON my.person (number);",
        );
        let result = package.validate(&Vec::new());

        // Column `person.number` is invalid
        assert_that!(result).is_err();
        let validation_errors = match result.err().unwrap() {
            PsqlpackError(ValidationError(errors), _) => errors,
            unexpected => panic!("Expected validation error however saw {:?}", unexpected),
        };
        assert_that!(validation_errors).has_length(1);
        match validation_errors[0] {
            ValidationKind::IndexInvalidReferenceColumns {
                ref index,
                ref table,
                ref columns,
            } => {
                assert_that!(*index).is_equal_to("idx_person_number".to_owned());
                assert_that!(*table).is_equal_to("my.person".to_owned());
                assert_that!(*columns).has_length(1);
                assert_that!(columns[0]).is_equal_to("number".to_owned());
            }
            ref unexpected => panic!("Unexpected validation type: {:?}", unexpected),
        }

        // Add the column and try again
        {
            let person = package.tables.iter_mut().find(|t| t.name.name.eq("person")).unwrap();
            person.columns.push(ast::ColumnDefinition {
                name: "number".to_owned(),
                sql_type: ast::SqlType::Simple(ast::SimpleSqlType::Integer, None),
                constraints: Vec::new(),
            });
        }
        assert_that!(package.validate(&Vec::new())).is_ok();
    }
}
