use std::io::Write;
use std::path::Path;
use std::fs::File;

use serde_json;

use sql::ast::*;
use connection::Connection;
use graph::Node;
use model::{Package, PublishProfile};
use errors::{PsqlpackResult, PsqlpackResultExt};
use errors::PsqlpackErrorKind::*;

macro_rules! dbtry {
    ($expr:expr) => {
        match $expr {
            Ok(o) => o,
            Err(e) => bail!(DatabaseError(format!("{}", e))),
        }
    };
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

enum DbObject<'a> {
    Extension(&'a ExtensionDefinition), // 2
    Function(&'a FunctionDefinition), // 6 (ordered)
    Schema(&'a SchemaDefinition), // 3
    Script(&'a ScriptDefinition), // 1, 7
    Table(&'a TableDefinition), // 5 (ordered)
    Type(&'a TypeDefinition), // 4
}

pub struct Delta<'package>(Vec<ChangeInstruction<'package>>);

impl<'package> Delta<'package> {
    pub fn generate(package: &'package Package, connection: &Connection, publish_profile: PublishProfile) -> PsqlpackResult<Delta<'package>> {
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
        if let Some(ref ordered_items) = package.order {
            for item in ordered_items {
                // Not the most efficient algorithm, perhaps something to cleanup
                match *item {
                    Node::Column(_) | Node::Constraint(_) => {
                        /* Necessary for ordering however unused here for now */
                    },
                    Node::Function(ref name) => {
                        if let Some(function) = package.functions.iter().find(|x| x.name.to_string() == *name) {
                            build_order.push(DbObject::Function(function));
                        } else {
                            // Warning?
                        }
                    },
                    Node::Table(ref name) => {
                        if let Some(table) = package.tables.iter().find(|x| x.name.to_string() == *name) {
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
        for script in &package.scripts {
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
                    DbObject::Extension(extension) => {
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
                    DbObject::Function(function) => {
                        // Since we don't really need to worry about this in PG we just
                        // add it as is and rely on CREATE OR REPLACE. In the future, it'd
                        // be good to check the hash or something to only do this when required
                        changeset.push(ChangeInstruction::ModifyFunction(function));
                    },
                    DbObject::Schema(schema) => {
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
                    DbObject::Script(script) => {
                        changeset.push(ChangeInstruction::RunScript(script));
                    },
                    DbObject::Table(table) => {
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
                    DbObject::Type(t) => {
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
                    DbObject::Extension(extension) => {
                        changeset.push(ChangeInstruction::AddExtension(extension));
                    },
                    DbObject::Function(function) => {
                        changeset.push(ChangeInstruction::AddFunction(function));
                    },
                    DbObject::Schema(schema) => {
                        changeset.push(ChangeInstruction::AddSchema(schema));
                    },
                    DbObject::Script(script) => {
                        changeset.push(ChangeInstruction::RunScript(script));
                    },
                    DbObject::Table(table) => {
                        changeset.push(ChangeInstruction::AddTable(table));
                    },
                    DbObject::Type(t) => {
                        changeset.push(ChangeInstruction::AddType(t));
                    }
                }
            }
        }
        Ok(Delta(changeset))
    }

    pub fn apply(&self, connection: &Connection) -> PsqlpackResult<()> {
        let changeset = &self.0;

        // These instructions turn into SQL statements that get executed
        let mut conn = dbtry!(connection.connect_host());

        for change in changeset.iter() {
            if let ChangeInstruction::UseDatabase(..) = *change {
                dbtry!(conn.finish());
                conn = dbtry!(connection.connect_database());
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

    pub fn write_report(&self, destination: &Path) -> PsqlpackResult<()> {
        let changeset = &self.0;

        File::create(destination)
            .chain_err(|| GenerationError("Failed to generate report".to_owned()))
            .and_then(|writer| serde_json::to_writer_pretty(writer, &changeset).chain_err(|| GenerationError("Failed to generate report".to_owned())))?;

        Ok(())
    }

    pub fn write_sql(&self, destination: &Path) -> PsqlpackResult<()> {
        let changeset = &self.0;

        // These instructions turn into a single SQL file
        let mut out = match File::create(destination) {
            Ok(o) => o,
            Err(e) => bail!(GenerationError(format!("Failed to generate SQL file: {}", e)))
        };

        for change in changeset.iter() {
            match out.write_all(change.to_sql().as_bytes()) {
                Ok(_) => {
                    // New line
                    match out.write(&[59u8, 10u8, 10u8]) {
                        Ok(_) => {},
                        Err(e) => bail!(GenerationError(format!("Failed to generate SQL file: {}", e)))
                    }
                },
                Err(e) => bail!(GenerationError(format!("Failed to generate SQL file: {}", e)))
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
            ChangeInstruction::AddExtension(ext) => {
                format!("CREATE EXTENSION {}", ext.name)
            },

            // Schema level
            ChangeInstruction::AddSchema(schema) => {
                format!("CREATE SCHEMA {}", schema.name)
            },

            // Type level
            ChangeInstruction::AddType(t) => {
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

    pub fn to_progress_message(&self) -> String {
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
