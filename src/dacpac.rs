use ast::*;
use std::fmt::{self};
use lexer::{self};
use lalrpop_util::ParseError;
use postgres::{Connection, TlsMode};
use serde_json::{self};
use std::ascii::AsciiExt;
use std::io::Read;
use std::io::prelude::*;
use std::path::Path;
use std::fs::{self,File};
use std::result::Result as StdResult;
use sql::{self};
use walkdir::WalkDir;
use zip::{ZipArchive,ZipWriter};
use zip::write::FileOptions;

macro_rules! ztry {
    ($expr:expr) => {{ 
        match $expr {
            Ok(_) => {},
            Err(e) => return Err(vec!(DacpacError::GenerationError { 
                message: format!("Failed to write DACPAC: {}", e),
            })),
        }
    }};
}

macro_rules! dbtry {
    ($expr:expr) => { 
        match $expr {
            Ok(o) => o,
            Err(e) => return Err(vec!(DacpacError::DatabaseError { 
                message: format!("{}", e),
            })),
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

static Q_DATABASE_EXISTS : &'static str = "SELECT 1 from pg_database WHERE datname=$1;";
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
    pub fn package_project(source_project_file: String, output_file: String) -> StdResult<(), Vec<DacpacError>> {

        // Load the project file
        let project_path = Path::new(&source_project_file[..]);
        if !project_path.is_file() {
            return Err(vec!(DacpacError::IOError {
                file: format!("{}", project_path.display()),
                message: "Project file does not exist".to_owned(),
            }));
        }
        let mut project_source = String::new();
        if let Err(err) = File::open(&project_path).and_then(|mut f| f.read_to_string(&mut project_source)) {
            return Err(vec!(DacpacError::IOError {
                     file: format!("{}", project_path.display()),
                     message: format!("Failed to read project file: {}", err)
                 }));
        }

        // Load the project
        let project_config : ProjectConfig = match serde_json::from_str(&project_source) {
            Ok(c) => c,
            Err(e) => return Err(vec!(DacpacError::ProjectError { message: format!("{}", e) })),
        };
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
                errors.push(DacpacError::IOError { 
                    file: format!("{}", path.display()), 
                    message: format!("{}", err) 
                });
                continue;
            }

            let tokens = match lexer::tokenize(&contents[..]) {
                Ok(t) => t,
                Err(e) => {
                    errors.push(DacpacError::SyntaxError { 
                        file: format!("{}", path.display()), 
                        line: e.line.to_owned(), 
                        line_number: e.line_number, 
                        start_pos: e.start_pos, 
                        end_pos: e.end_pos 
                    });
                    continue;
                },
            };

            match sql::parse_statement_list(tokens) {
                Ok(statement_list) => { 
                    for statement in statement_list {
                        match statement {
                            Statement::Table(table_definition) => project.push_table(table_definition),
                            Statement::Schema(schema_definition) => project.push_schema(schema_definition),
                        }
                    }
                },
                Err(err) => { 
                    errors.push(DacpacError::ParseError { 
                        file: format!("{}", path.display()), 
                        errors: vec!(err), 
                    });
                    continue;
                }
            }            
        }

        // Early exit if errors
        if !errors.is_empty() {
            return Err(errors);
        }

        // First up validate the dacpac
        project.set_defaults(project_config);
        try!(project.validate());
        project.update_dependency_graph();

        // Now generate the dacpac
        let output_path = Path::new(&output_file[..]);
        if output_path.parent().is_some() {
            match fs::create_dir_all(format!("{}", output_path.parent().unwrap().display())) {
                Ok(_) => {},
                Err(e) => return Err(vec!(DacpacError::GenerationError { 
                    message: format!("Failed to create DACPAC directory: {}", e),
                })),
            }
        }

        let output_file = match File::create(&output_path) {
            Ok(f) => f,
            Err(e) => return Err(vec!(DacpacError::GenerationError { 
                message: format!("Failed to write DACPAC: {}", e),
            })),
        };
        let mut zip = ZipWriter::new(output_file);

        ztry!(zip.add_directory("tables/", FileOptions::default()));

        for table in project.tables {
            ztry!(zip.start_file(format!("tables/{}.json", table.name), FileOptions::default()));
            let json = match serde_json::to_string_pretty(&table) {
                Ok(j) => j,
                Err(e) => return Err(vec!(DacpacError::GenerationError {
                    message: format!("Failed to write DACPAC: {}", e),
                })),
            };
            ztry!(zip.write_all(json.as_bytes()));
        }
        ztry!(zip.finish());

        Ok(())
    }

    pub fn publish(source_dacpac_file: String, target_connection_string: String, publish_profile: String) -> StdResult<(), Vec<DacpacError>> {
        
        let project = try!(Dacpac::load_project(source_dacpac_file));
        let publish_profile = try!(Dacpac::load_publish_profile(publish_profile));
        let connection_string = try!(Dacpac::test_connection(target_connection_string));

        // Now we generate our instructions
        let changeset = project.generate_changeset(&connection_string, publish_profile)?;

        // These instructions turn into SQL statements that get executed
        let mut conn = dbtry!(Connection::connect(connection_string.uri(false), connection_string.tls_mode()));
        for change in &changeset {
            if let ChangeInstruction::UseDatabase(..) = *change {
                dbtry!(conn.finish());
                conn = dbtry!(Connection::connect(connection_string.uri(true), connection_string.tls_mode()));
                continue;
            }

            // Execute SQL directly
            println!("{}", change.to_progress_message());
            dbtry!(conn.execute(&change.to_sql()[..], &[]));
        }
        // Close the connection
        dbtry!(conn.finish());

        Ok(())
    }

    pub fn generate_sql(source_dacpac_file: String, target_connection_string: String, publish_profile: String, output_file: String) -> StdResult<(), Vec<DacpacError>> {

        let project = try!(Dacpac::load_project(source_dacpac_file));
        let publish_profile = try!(Dacpac::load_publish_profile(publish_profile));
        let connection_string = try!(Dacpac::test_connection(target_connection_string));

        // Now we generate our instructions
        let changeset = project.generate_changeset(&connection_string, publish_profile)?;

        // These instructions turn into a single SQL file
        let mut out = match File::create(&output_file[..]) {
            Ok(o) => o,
            Err(e) => return Err(vec!(DacpacError::GenerationError {
                message: format!("Failed to generate SQL file: {}", e),
            })),
        };

        for change in changeset {
            match out.write_all(change.to_sql().as_bytes()) {
                Ok(_) => {
                    // New line
                    match out.write(&[59u8, 10u8, 10u8]) {
                        Ok(_) => {},
                        Err(e) => return Err(vec!(DacpacError::GenerationError {
                            message: format!("Failed to generate SQL file: {}", e),
                        })),
                    }
                },
                Err(e) => return Err(vec!(DacpacError::GenerationError {
                    message: format!("Failed to generate SQL file: {}", e),
                })),
            }
        }

        Ok(())
    }

    pub fn generate_report(source_dacpac_file: String, target_connection_string: String, publish_profile: String, output_file: String) -> StdResult<(), Vec<DacpacError>> {

        let project = try!(Dacpac::load_project(source_dacpac_file));
        let publish_profile = try!(Dacpac::load_publish_profile(publish_profile));
        let connection_string = try!(Dacpac::test_connection(target_connection_string));

        // Now we generate our instructions
        let changeset = project.generate_changeset(&connection_string, publish_profile)?;

        // These instructions turn into a JSON report
        let json = match serde_json::to_string_pretty(&changeset) {
            Ok(j) => j,
            Err(e) => return Err(vec!(DacpacError::GenerationError {
                message: format!("Failed to generate report: {}", e),
            })),
        };

        let mut out = match File::create(&output_file[..]) {
            Ok(o) => o,
            Err(e) => return Err(vec!(DacpacError::GenerationError {
                message: format!("Failed to generate report: {}", e),
            })),
        };
        match out.write_all(json.as_bytes()) {
            Ok(_) => {},
            Err(e) => return Err(vec!(DacpacError::GenerationError {
                message: format!("Failed to generate report: {}", e),
            })),
        }

        Ok(())
    }

    fn load_project(source_dacpac_file: String) -> StdResult<Project, Vec<DacpacError>> {
        // Load the DACPAC
        let source_path = Path::new(&source_dacpac_file[..]);
        if !source_path.is_file() {
            return Err(vec!(DacpacError::IOError {
                file: format!("{}", source_path.display()),
                message: "DACPAC file does not exist".to_owned(),
            }));
        }
        let file = match fs::File::open(&source_path) {
            Ok(o) => o,
            Err(e) => return Err(vec!(DacpacError::IOError {
                file: format!("{}", source_path.display()),
                message: format!("Failed to open DACPAC file: {}", e),
            })),
        };
        let mut archive = match ZipArchive::new(file) {
            Ok(o) => o,
            Err(e) => return Err(vec!(DacpacError::IOError {
                file: format!("{}", source_path.display()),
                message: format!("Failed to open DACPAC file: {}", e),
            })),
        };

        let mut tables = Vec::new();
        let mut schemas = Vec::new();

        for i in 0..archive.len()
        {
            let mut file = archive.by_index(i).unwrap();
            if file.size() == 0 {
                continue;
            }
            if file.name().starts_with("tables/") {
                load_file!(TableDefinition, tables, file);
            } else if file.name().starts_with("schemas/") {
                load_file!(SchemaDefinition, schemas, file);
            }
        }

        Ok(Project {
            schemas: schemas,
            tables: tables,
        })
    }

    fn load_publish_profile(publish_profile: String) -> StdResult<PublishProfile, Vec<DacpacError>> {
        // Load the publish profile
        let path = Path::new(&publish_profile[..]);
        if !path.is_file() {
            return Err(vec!(DacpacError::IOError {
                file: format!("{}", path.display()),
                message: "Publish profile does not exist".to_owned(),
            }));
        }
        let mut publish_profile_raw = String::new();
        if let Err(err) = File::open(&path).and_then(|mut f| f.read_to_string(&mut publish_profile_raw)) {
            return Err(vec!(DacpacError::IOError {
                     file: format!("{}", path.display()),
                     message: format!("Failed to read publish profile: {}", err)
                 }));
        }

        // Deserialize
        let publish_profile : PublishProfile = match serde_json::from_str(&publish_profile_raw) {
            Ok(p) => p,
            Err(e) => return Err(vec!(DacpacError::FormatError {
                file: format!("{}", path.display()),
                message: format!("Publish profile was not well formed: {}", e),
            })),
        };
        Ok(publish_profile)
    }

    fn test_connection(target_connection_string: String) -> StdResult<ConnectionString, Vec<DacpacError>> {

        // Connection String
        let mut connection_string = ConnectionString::new();

        // First up, parse the connection string
        let sections: Vec<&str> = target_connection_string.split(';').collect();
        for section in sections {

            if section.trim().is_empty() {
                continue;
            }

            // Get the parts
            let parts: Vec<&str> = section.split('=').collect();
            if parts.len() != 2 {
                return Err(vec!(DacpacError::InvalidConnectionString {
                    message: "Connection string was not well formed".to_owned(),
                }));
            }

            match parts[0] {
                "host" => connection_string.set_host(parts[1]),
                "database" => connection_string.set_database(parts[1]),
                "userid" => connection_string.set_user(parts[1]),
                "password" => connection_string.set_password(parts[1]),
                "tlsmode" => connection_string.set_tls_mode(parts[1]),
                _ => {}
            }
        }

        // Make sure we have enough for a connection string
        try!(connection_string.validate());

        Ok(connection_string)
    }
}

struct ConnectionString {
    database : Option<String>,
    host : Option<String>,
    user : Option<String>,
    password : Option<String>,
    tls_mode : bool,
}

macro_rules! assert_existance {
    ($s:ident, $field:ident, $errors:ident) => {{ 
        if $s.$field.is_none() {
            let text = stringify!($field);
            $errors.push(DacpacError::InvalidConnectionString { message: format!("No {} defined", text) }); 
        }
    }};
}

impl ConnectionString {
    fn new() -> Self {
        ConnectionString {
            database: None,
            host: None,
            user: None,
            password: None,
            tls_mode: false
        }
    }

    fn set_database(&mut self, value: &str) {
        self.database = Some(value.to_owned());
    }

    fn set_host(&mut self, value: &str) {
        self.host = Some(value.to_owned());
    }

    fn set_user(&mut self, value: &str) {
        self.user = Some(value.to_owned());
    }

    fn set_password(&mut self, value: &str) {
        self.password = Some(value.to_owned());
    }

    fn set_tls_mode(&mut self, value: &str) {
        self.tls_mode = value.eq_ignore_ascii_case("true");
    }

    fn validate(&self) -> StdResult<(), Vec<DacpacError>> {
        let mut errors = Vec::new();
        assert_existance!(self, database, errors);
        assert_existance!(self, host, errors);
        assert_existance!(self, user, errors);
        if self.tls_mode {
            errors.push(DacpacError::InvalidConnectionString { message: "TLS not supported".to_owned() }); 
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn uri(&self, with_database: bool) -> String {
        // Assumes validate has been called
        if self.password.is_none() {
            if with_database {
                format!("postgres://{}@{}/{}", self.user.clone().unwrap(), self.host.clone().unwrap(), self.database.clone().unwrap())                
            } else {
                format!("postgres://{}@{}", self.user.clone().unwrap(), self.host.clone().unwrap())
            }
        } else {
            if with_database {
                format!("postgres://{}:{}@{}/{}", self.user.clone().unwrap(), self.password.clone().unwrap(), self.host.clone().unwrap(), self.database.clone().unwrap())
            } else {
                format!("postgres://{}:{}@{}", self.user.clone().unwrap(), self.password.clone().unwrap(), self.host.clone().unwrap())
            }
        }
    }

    fn tls_mode(&self) -> TlsMode {
        TlsMode::None
    }
}

#[derive(Deserialize)]
struct ProjectConfig {
    version: String,
    default_schema: String,
    predeploy_scripts: Vec<String>,
    postdeploy_scripts: Vec<String>,
}

struct Project {
    tables: Vec<TableDefinition>,
    schemas: Vec<SchemaDefinition>,
}

impl Project {

    fn new() -> Self {
        Project {
            schemas: Vec::new(),
            tables: Vec::new(),
        }
    }

    fn push_table(&mut self, table: TableDefinition) {
        self.tables.push(table);
    }

    fn push_schema(&mut self, schema: SchemaDefinition) {
        self.schemas.push(schema);
    }

    fn set_defaults(&mut self, config: ProjectConfig) { 

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

    fn update_dependency_graph(&mut self) {

    }

    fn validate(&self) -> Result<(), Vec<DacpacError>> {

        // TODO: Validate references etc
        Ok(())
    }

    fn generate_changeset(&self, connection_string: &ConnectionString, publish_profile: PublishProfile) -> StdResult<Vec<ChangeInstruction>, Vec<DacpacError>> {

        // Start the changeset
        let mut changeset = Vec::new();

        // First up, detect if there is no database (or it needs to be recreated)
        // If so, we assume everything is new
        let db_conn = dbtry!(Connection::connect(connection_string.uri(false), connection_string.tls_mode()));
        let db_result = dbtry!(db_conn.query(Q_DATABASE_EXISTS, &[ &connection_string.database.clone().unwrap() ]));
        let mut has_db = !db_result.is_empty();

        // If we always recreate then add a drop and set to false
        if has_db && publish_profile.always_recreate_database {
            changeset.push(ChangeInstruction::DropDatabase(connection_string.database.clone().unwrap()));
            has_db = false;
        }

        // If we have the DB we generate an actual change set, else we generate new instructions
        if has_db {

            // Set the connection instruction
            changeset.push(ChangeInstruction::UseDatabase(connection_string.database.clone().unwrap()));

            // Connect to the database
            let conn = dbtry!(Connection::connect(connection_string.uri(true), connection_string.tls_mode()));

            // Go through each table
            for table in &self.tables {
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
            } 
        } else {
            changeset.push(ChangeInstruction::CreateDatabase(connection_string.database.clone().unwrap()));
            changeset.push(ChangeInstruction::UseDatabase(connection_string.database.clone().unwrap()));
            for table in &self.tables {
                changeset.push(ChangeInstruction::AddTable(table));
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

#[derive(Serialize)]
enum ChangeInstruction<'input> {

    // Databases
    DropDatabase(String),
    CreateDatabase(String),
    UseDatabase(String),

    // Tables
    AddTable(&'input TableDefinition),
    RemoveTable(String),

    // Columns
    AddColumn(&'input ColumnDefinition),
    ModifyColumn(&'input ColumnDefinition),
    RemoveColumn(String),
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
                format!("/c {}", db)
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

impl fmt::Display for TableName {
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
            SqlType::FixedLengthString(size) => write!(f, "char({})", size),
            SqlType::VariableLengthString(size) => write!(f, "varchar({})", size),
            SqlType::Text => write!(f, "text"),
            
            SqlType::FixedLengthBitString(size) => write!(f, "bit({})", size),
            SqlType::VariableLengthBitString(size) => write!(f, "varbit({})", size),

            SqlType::SmallInteger => write!(f, "smallint"),
            SqlType::Integer => write!(f, "int"),
            SqlType::BigInteger => write!(f, "bigint"),

            SqlType::SmallSerial => write!(f, "smallserial"),
            SqlType::Serial => write!(f, "serial"),
            SqlType::BigSerial => write!(f, "bigserial"),

            SqlType::Numeric(m, d) => write!(f, "numeric({},{})", m, d),
            SqlType::Double => write!(f, "double precision"),
            SqlType::Single => write!(f, "real"),
            SqlType::Money => write!(f, "money"),

            SqlType::Boolean => write!(f, "bool"),

            SqlType::Date => write!(f, "date"),
            SqlType::DateTime => write!(f, "timestamp without time zone"),
            SqlType::DateTimeWithTimeZone => write!(f, "timestamp with time zone"),
            SqlType::Time => write!(f, "time"),
            SqlType::TimeWithTimeZone => write!(f, "time with time zone"),

            SqlType::Uuid => write!(f, "uuid"),

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

pub enum DacpacError {
    IOError { file: String, message: String },
    SyntaxError { file: String, line: String, line_number: i32, start_pos: i32, end_pos: i32 },
    ParseError { file: String, errors: Vec<ParseError<(), lexer::Token, ()>> },
    GenerationError { message: String },
    FormatError { file: String, message: String },
    InvalidConnectionString { message: String },
    DatabaseError { message: String },
    ProjectError { message: String },
}

impl DacpacError {
    pub fn print(&self) {
        match *self {
            DacpacError::IOError { ref file, ref message } => {
                println!("IO Error when reading {}", file);
                println!("  {}", message);
                println!();
            },
            DacpacError::FormatError { ref file, ref message } => {
                println!("Formatting Error when reading {}", file);
                println!("  {}", message);
                println!();
            },
            DacpacError::InvalidConnectionString { ref message } => {
                println!("Invalid connection string");
                println!("  {}", message);
                println!();
            },
            DacpacError::SyntaxError { ref file, ref line, line_number, start_pos, end_pos } => {
                println!("Syntax error in {} on line {}", file, line_number);
                println!("  {}", line);
                print!("  ");
                for _ in 0..start_pos {
                    print!(" ");
                }
                for _ in start_pos..end_pos {
                    print!("^");
                }
                println!();
            },
            DacpacError::ParseError { ref file, ref errors } => {
                println!("Error in {}", file);
                for e in errors.iter() {
                    match *e {
                        ParseError::InvalidToken { .. } => { 
                            println!("  Invalid token");
                        },
                        ParseError::UnrecognizedToken { ref token, ref expected } => {
                            if let Some(ref x) = *token {
                                println!("  Unexpected {:?}.", x.1);
                            } else {
                                println!("  Unexpected end of file");
                            }
                            print!("  Expected one of: ");
                            let mut first = true;
                            for expect in expected {
                                if first {
                                    first = false;
                                } else {
                                    print!(", ");
                                }
                                print!("{}", expect);
                            }
                            println!();
                        },
                        ParseError::ExtraToken { ref token } => {
                            println!("  Extra token detectd: {:?}", token);
                        },
                        ParseError::User { ref error } => {
                            println!("  {:?}", error);
                        },
                    }
                }
                println!();                            
            },
            DacpacError::GenerationError { ref message } => {
                println!("Error generating DACPAC");
                println!("  {}", message);
                println!();
            },
            DacpacError::DatabaseError { ref message } => {
                println!("Database error:");
                println!("  {}", message);
                println!();
            },
            DacpacError::ProjectError { ref message } => {
                println!("Project format error:");
                println!("  {}", message);
                println!();
            },
        }        
    }
}