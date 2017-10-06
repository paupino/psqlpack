use std::fmt;
use std::io::Write;
use std::path::Path;
use std::fs::File;

use slog::Logger;
use serde_json;

use sql::ast::*;
use connection::Connection;
use model::{Node, Package, PublishProfile};
use errors::{PsqlpackResult, PsqlpackResultExt};
use errors::PsqlpackErrorKind::*;

static Q_DATABASE_EXISTS: &'static str = "SELECT 1 FROM pg_database WHERE datname=$1;";

enum DbObject<'a> {
    Extension(&'a ExtensionDefinition), // 2
    Function(&'a FunctionDefinition),   // 6 (ordered)
    Schema(&'a SchemaDefinition),       // 3
    Script(&'a ScriptDefinition),       // 1, 7
    Table(&'a TableDefinition),         // 5 (ordered)
    Column(&'a TableDefinition, &'a ColumnDefinition),
    Constraint(&'a TableDefinition, &'a TableConstraint),
    Type(&'a TypeDefinition), // 4
}

impl<'a> fmt::Display for DbObject<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DbObject::Extension(extension) => write!(f, "Extension: {}", extension.name), // 2
            DbObject::Function(function) => write!(f, "Function: {}", function.name),     // 6 (ordered)
            DbObject::Schema(schema) => write!(f, "Schema: {}", schema.name),             // 3
            DbObject::Script(script) => write!(f, "Script: {}", script.name),             // 1, 7
            DbObject::Table(table) => write!(f, "Table: {}", table.name),                 // 5 (ordered)
            DbObject::Column(table, column) => write!(f, "Table: {}, Column: {}", table.name, column.name),
            DbObject::Constraint(table, constraint) => write!(
                f,
                "Table: {}, Constraint: {}",
                table.name,
                constraint.name()
            ),
            DbObject::Type(tipe) => write!(f, "Type: {}", tipe.name), // 4
        }
    }
}

pub struct Delta<'package>(Vec<ChangeInstruction<'package>>);

impl<'package> Delta<'package> {
    pub fn generate(
        log: &Logger,
        package: &'package Package,
        connection: &Connection,
        publish_profile: PublishProfile,
    ) -> PsqlpackResult<Delta<'package>> {
        let log = log.new(o!("delta" => "generate"));

        // Start the changeset
        let mut changeset = Vec::new();

        // Create the build order - including all document types outside the topological sort.
        let mut build_order = Vec::new();

        // Pre deployment scripts
        for script in &package.scripts {
            if script.kind == ScriptKind::PreDeployment {
                build_order.push(DbObject::Script(script));
            }
        }

        // Extensions
        for extension in &package.extensions {
            build_order.push(DbObject::Extension(extension));
        }

        // Schemas
        for schema in &package.schemas {
            build_order.push(DbObject::Schema(schema));
        }

        // Types
        for t in &package.types {
            build_order.push(DbObject::Type(t));
        }

        // Now add everything else per the topological sort
        for item in package.generate_dependency_graph(&log)? {
            match item {
                Node::Function(_) => {
                    // for the moment, add these later.
                }
                Node::Table(table) => {
                    build_order.push(DbObject::Table(table));
                }
                Node::Column(table, column) => {
                    build_order.push(DbObject::Column(table, column));
                }
                Node::Constraint(table, constraint) => {
                    build_order.push(DbObject::Constraint(table, constraint));
                }
            }
        }

        for function in &package.functions {
            build_order.push(DbObject::Function(function));
        }

        // Add in post deployment scripts
        for script in &package.scripts {
            if script.kind == ScriptKind::PostDeployment {
                build_order.push(DbObject::Script(script));
            }
        }

        // First up, detect if there is no database (or it needs to be recreated)
        // If so, we assume everything is new
        trace!(log, "Connecting to host");
        let db_conn = dbtry!(connection.connect_host());
        trace!(
            log,
            "Checking for database `{}`",
            &connection.database()[..]
        );
        let db_result = dbtry!(db_conn.query(Q_DATABASE_EXISTS, &[&connection.database()]));
        let mut has_db = !db_result.is_empty();

        // If we always recreate then add a drop and set to false
        if has_db && publish_profile.always_recreate_database {
            changeset.push(ChangeInstruction::DropDatabase(
                connection.database().to_owned(),
            ));
            has_db = false;
        }

        // If we have the DB we generate an actual change set, else we generate new instructions
        if has_db {
            // We'll compare a delta against the existing state
            let existing_database = Package::from_connection(&connection)?;

            // Set the connection instruction
            changeset.push(ChangeInstruction::UseDatabase(
                connection.database().to_owned(),
            ));

            // Go through each item in order and figure out what to do with it
            for item in &build_order {
                match *item {
                    DbObject::Extension(extension) => {
                        changeset.push(ChangeInstruction::AssertExtension(extension));
                    }
                    DbObject::Function(function) => {
                        // Since we don't really need to worry about this in PG we just
                        // add it as is and rely on CREATE OR REPLACE. In the future, it'd
                        // be good to check the hash or something to only do this when required
                        changeset.push(ChangeInstruction::ModifyFunction(function));
                    }
                    DbObject::Schema(schema) => {
                        // Only add schema's, we do not drop them at this point
                        let schema_exists = existing_database
                            .schemas
                            .iter()
                            .any(|s| s.name == schema.name);
                        if !schema_exists {
                            changeset.push(ChangeInstruction::AddSchema(schema));
                        }
                    }
                    DbObject::Script(script) => {
                        changeset.push(ChangeInstruction::RunScript(script));
                    }
                    DbObject::Table(table) => {
                        let table_exists = existing_database
                            .tables
                            .iter()
                            .any(|t| t.name == table.name);
                        if table_exists {
                            // Check the columns

                            // Check the constraints
                        } else {
                            changeset.push(ChangeInstruction::AddTable(table));
                        }
                    }
                    DbObject::Type(ty) => {
                        let type_exists = existing_database.types.iter().any(|t| t.name == ty.name);
                        if type_exists {
                            // TODO: Need to figure out if it's changed and also perhaps how it's changed.
                            //       I don't think a blanket modify is enough.
                        } else {
                            changeset.push(ChangeInstruction::AddType(ty));
                        }
                    }
                    ref unhandled => warn!(log, "TODO - unhandled DBObject: {}", unhandled),
                }
            }
        } else {
            changeset.push(ChangeInstruction::CreateDatabase(
                connection.database().to_owned(),
            ));
            changeset.push(ChangeInstruction::UseDatabase(
                connection.database().to_owned(),
            ));

            // Since this is a new database add everything (in order)
            for item in &build_order {
                match *item {
                    DbObject::Extension(extension) => {
                        changeset.push(ChangeInstruction::AssertExtension(extension));
                    }
                    DbObject::Function(function) => {
                        changeset.push(ChangeInstruction::AddFunction(function));
                    }
                    DbObject::Schema(schema) => {
                        changeset.push(ChangeInstruction::AddSchema(schema));
                    }
                    DbObject::Script(script) => {
                        changeset.push(ChangeInstruction::RunScript(script));
                    }
                    DbObject::Table(table) => {
                        changeset.push(ChangeInstruction::AddTable(table));
                    }
                    DbObject::Column(table, column) => {
                        changeset.push(ChangeInstruction::AddColumn(table, column));
                    }
                    DbObject::Constraint(table, constraint) => {
                        changeset.push(ChangeInstruction::AddConstraint(table, constraint));
                    }
                    DbObject::Type(t) => {
                        changeset.push(ChangeInstruction::AddType(t));
                    }
                }
            }
        }
        Ok(Delta(changeset))
    }

    pub fn apply(&self, log: &Logger, connection: &Connection) -> PsqlpackResult<()> {
        let log = log.new(o!("delta" => "apply"));

        let changeset = &self.0;

        // These instructions turn into SQL statements that get executed
        let mut conn = connection.connect_host()?;

        for change in changeset.iter() {
            if let ChangeInstruction::UseDatabase(..) = *change {
                conn.finish().chain_err(|| DatabaseConnectionFinishError)?;
                conn = connection.connect_database()?;
                continue;
            }

            // Execute SQL directly
            trace!(log, "Executing: {}", change);
            let sql = change.to_sql(&log);
            conn.batch_execute(&sql)
                .chain_err(|| DatabaseExecuteError(sql))?;
        }

        // Close the connection
        conn.finish().chain_err(|| DatabaseConnectionFinishError)?;

        Ok(())
    }

    pub fn write_report(&self, destination: &Path) -> PsqlpackResult<()> {
        let changeset = &self.0;

        File::create(destination)
            .chain_err(|| GenerationError("Failed to generate report".to_owned()))
            .and_then(|writer| {
                serde_json::to_writer_pretty(writer, &changeset)
                    .chain_err(|| GenerationError("Failed to generate report".to_owned()))
            })?;

        Ok(())
    }

    pub fn write_sql(&self, log: &Logger, destination: &Path) -> PsqlpackResult<()> {
        let changeset = &self.0;

        // These instructions turn into a single SQL file
        let mut out = match File::create(destination) {
            Ok(o) => o,
            Err(e) => bail!(GenerationError(
                format!("Failed to generate SQL file: {}", e)
            )),
        };

        for change in changeset.iter() {
            let sql = change.to_sql(log);
            match out.write_all(sql.as_bytes()) {
                Ok(_) => {
                    // New line
                    match out.write(&[59u8, 10u8, 10u8]) {
                        Ok(_) => {}
                        Err(e) => bail!(GenerationError(
                            format!("Failed to generate SQL file: {}", e)
                        )),
                    }
                }
                Err(e) => bail!(GenerationError(
                    format!("Failed to generate SQL file: {}", e)
                )),
            }
        }

        Ok(())
    }
}

#[allow(dead_code)]
#[derive(Serialize)]
pub enum ChangeInstruction<'input> {
    // Databases
    DropDatabase(String),
    CreateDatabase(String),
    UseDatabase(String),

    // Extensions
    AssertExtension(&'input ExtensionDefinition),

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
    AddColumn(&'input TableDefinition, &'input ColumnDefinition),
    ModifyColumn(&'input ColumnDefinition),
    RemoveColumn(String),

    // Constraints
    AddConstraint(&'input TableDefinition, &'input TableConstraint),

    // Functions
    AddFunction(&'input FunctionDefinition),
    ModifyFunction(&'input FunctionDefinition), // This is identical to add however it's for future possible support
    DropFunction(String),
}

impl<'input> fmt::Display for ChangeInstruction<'input> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::ChangeInstruction::*;

        match *self {
            // Databases
            DropDatabase(ref database) => write!(f, "Drop database: {}", database),
            CreateDatabase(ref database) => write!(f, "Create database: {}", database),
            UseDatabase(ref database) => write!(f, "Use database: {}", database),

            // Extensions
            AssertExtension(extension) => write!(f, "Assert extension: {}", extension.name),

            // Schema
            AddSchema(schema) => write!(f, "Add schema: {}", schema.name),
            //RemoveSchema(String),

            // Scripts
            RunScript(script) => write!(f, "Run script: {}", script.name),

            // Types
            AddType(tipe) => write!(f, "Add type: {}", tipe.name),
            RemoveType(ref type_name) => write!(f, "Remove type: {}", type_name),

            // Tables
            AddTable(table) => write!(f, "Add table: {}", table.name),
            RemoveTable(ref table_name) => write!(f, "Remove table: {}", table_name),

            // Columns
            AddColumn(table, column) => write!(f, "Add column: {} to table: {}", column.name, table.name),
            ModifyColumn(column) => write!(f, "Modify column: {}", column.name),
            RemoveColumn(ref column_name) => write!(f, "Remove column: {}", column_name),

            // Constraints
            AddConstraint(table, constraint) => write!(
                f,
                "Add constraint: {} to table: {}",
                constraint.name(),
                table.name
            ),

            // Functions
            AddFunction(function) => write!(f, "Add function: {}", function.name),
            // Modify is identical to add however it's for future possible support
            ModifyFunction(function) => write!(f, "Modify function: {}", function.name),
            DropFunction(ref function_name) => write!(f, "Drop function: {}", function_name),
        }
    }
}

impl<'input> ChangeInstruction<'input> {
    fn to_sql(&self, log: &Logger) -> String {
        match *self {
            // Database level
            ChangeInstruction::CreateDatabase(ref db) => format!("CREATE DATABASE {}", db),
            ChangeInstruction::DropDatabase(ref db) => format!("DROP DATABASE {}", db),
            ChangeInstruction::UseDatabase(ref db) => format!("-- Using database `{}`", db),

            // Extension level
            ChangeInstruction::AssertExtension(ext) => format!("-- Assert extension exists {}", ext.name),

            // Schema level
            ChangeInstruction::AddSchema(schema) => if schema.name == "public" {
                format!("CREATE SCHEMA IF NOT EXISTS {}", schema.name)
            } else {
                format!("CREATE SCHEMA {}", schema.name)
            },

            // Type level
            ChangeInstruction::AddType(t) => {
                let mut def = String::new();
                def.push_str(&format!("CREATE TYPE {} AS ", t.name)[..]);
                match t.kind {
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
            }

            // Function level
            ChangeInstruction::AddFunction(function) | ChangeInstruction::ModifyFunction(function) => {
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
                    }
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
                    FunctionLanguage::SQL => func.push_str("SQL"),
                }
                func
            }

            // Table level
            ChangeInstruction::AddTable(def) => format!("CREATE TABLE {} ()", def.name),

            // Column level
            ChangeInstruction::AddColumn(table, column) => {
                let mut instr = String::new();
                instr.push_str(&format!("ALTER TABLE {}\n", table.name));
                instr.push_str(&format!("ADD COLUMN {} {}", column.name, column.sql_type));
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
                instr
            }

            ChangeInstruction::AddConstraint(table, constraint) => {
                let mut instr = String::new();
                instr.push_str(&format!("ALTER TABLE {}\nADD ", table.name));
                match *constraint {
                    TableConstraint::Primary {
                        ref name,
                        ref columns,
                        ref parameters,
                    } => {
                        instr.push_str(&format!(
                            "CONSTRAINT {} PRIMARY KEY ({})",
                            name,
                            columns.join(", ")
                        ));

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
                    }
                    TableConstraint::Foreign {
                        ref name,
                        ref columns,
                        ref ref_table,
                        ref ref_columns,
                        ref match_type,
                        ref events,
                    } => {
                        instr.push_str(&format!("CONSTRAINT {} FOREIGN KEY ({})", name, columns.join(", "))[..]);
                        instr.push_str(&format!(" REFERENCES {} ({})", ref_table, ref_columns.join(", "))[..]);
                        if let Some(ref m) = *match_type {
                            instr.push_str(&format!(" {}", m));
                        }
                        if let Some(ref events) = *events {
                            for e in events {
                                match *e {
                                    ForeignConstraintEvent::Delete(ref action) => {
                                        instr.push_str(&format!(" ON DELETE {}", action))
                                    }
                                    ForeignConstraintEvent::Update(ref action) => {
                                        instr.push_str(&format!(" ON UPDATE {}", action))
                                    }
                                }
                            }
                        }
                    }
                }
                instr
            }

            // Raw scripts
            ChangeInstruction::RunScript(script) => {
                let mut instr = String::new();
                instr.push_str(&format!("-- Script: {}\n", script.name)[..]);
                instr.push_str(&script.contents[..]);
                instr.push('\n');
                instr
            }

            ref unimplemented => {
                warn!(log, "TODO - not creating SQL for {}", unimplemented);
                "".to_owned()
            }
        }
    }
}
