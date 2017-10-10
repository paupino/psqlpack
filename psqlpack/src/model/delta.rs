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

trait Diffable<'a, T> {
    fn generate(
        &self,
        changeset: &mut Vec<ChangeInstruction<'a>>,
        target: &T,
        publish_profile: &PublishProfile,
        log: &Logger,
    ) -> PsqlpackResult<()>;
}

impl<'a> Diffable<'a, Package> for DbObject<'a> {
    fn generate(
        &self,
        changeset: &mut Vec<ChangeInstruction<'a>>,
        target: &Package,
        publish_profile: &PublishProfile,
        log: &Logger,
    ) -> PsqlpackResult<()> {
        match *self {
            DbObject::Extension(extension) => extension.generate(changeset, target, publish_profile, log),
            DbObject::Function(function) => function.generate(changeset, target, publish_profile, log),
            DbObject::Schema(schema) => schema.generate(changeset, target, publish_profile, log),
            DbObject::Script(script) => script.generate(changeset, target, publish_profile, log),
            DbObject::Table(table) => table.generate(changeset, target, publish_profile, log),
            DbObject::Type(ty) => ty.generate(changeset, target, publish_profile, log),
            ref unhandled => {
                warn!(log, "TODO - unhandled DBObject: {}", unhandled);
                Ok(())
            }
        }
    }
}

impl<'a> Diffable<'a, Package> for &'a ExtensionDefinition {
    fn generate(
        &self,
        changeset: &mut Vec<ChangeInstruction<'a>>,
        _: &Package,
        _: &PublishProfile,
        _: &Logger,
    ) -> PsqlpackResult<()> {
        changeset.push(ChangeInstruction::AssertExtension(self));
        Ok(())
    }
}

impl<'a> Diffable<'a, Package> for &'a FunctionDefinition {
    fn generate(
        &self,
        changeset: &mut Vec<ChangeInstruction<'a>>,
        _: &Package,
        _: &PublishProfile,
        _: &Logger,
    ) -> PsqlpackResult<()> {
        // Since we don't really need to worry about this in PG we just
        // add it as is and rely on CREATE OR REPLACE. In the future, it'd
        // be good to check the hash or something to only do this when required
        changeset.push(ChangeInstruction::ModifyFunction(self));
        Ok(())
    }
}

impl<'a> Diffable<'a, Package> for &'a SchemaDefinition {
    fn generate(
        &self,
        changeset: &mut Vec<ChangeInstruction<'a>>,
        target: &Package,
        _: &PublishProfile,
        _: &Logger,
    ) -> PsqlpackResult<()> {
        // Only add schema's, we do not drop them at this point
        let schema_exists = target.schemas.iter().any(|s| s.name == self.name);
        if !schema_exists {
            changeset.push(ChangeInstruction::AddSchema(self));
        }
        Ok(())
    }
}

impl<'a> Diffable<'a, Package> for &'a ScriptDefinition {
    fn generate(
        &self,
        changeset: &mut Vec<ChangeInstruction<'a>>,
        _: &Package,
        _: &PublishProfile,
        _: &Logger,
    ) -> PsqlpackResult<()> {
        changeset.push(ChangeInstruction::RunScript(self));
        Ok(())
    }
}

impl<'a> Diffable<'a, Package> for &'a TableDefinition {
    fn generate(
        &self,
        changeset: &mut Vec<ChangeInstruction<'a>>,
        target: &Package,
        _: &PublishProfile,
        _: &Logger,
    ) -> PsqlpackResult<()> {
        let table_exists = target.tables.iter().any(|t| t.name == self.name);
        if table_exists {
            // Check the columns

            // Check the constraints

        } else {
            changeset.push(ChangeInstruction::AddTable(self));
        }
        Ok(())
    }
}

impl<'a> Diffable<'a, Package> for &'a TypeDefinition {
    fn generate(
        &self,
        changeset: &mut Vec<ChangeInstruction<'a>>,
        target: &Package,
        publish_profile: &PublishProfile,
        log: &Logger,
    ) -> PsqlpackResult<()> {
        let ty = target.types.iter().find(|t| t.name == self.name);
        if let Some(ty) = ty {
            self.generate(changeset, ty, publish_profile, log)
        } else {
            changeset.push(ChangeInstruction::AddType(self));
            Ok(())
        }
    }
}

impl<'a> Diffable<'a, TypeDefinition> for &'a TypeDefinition {
    fn generate(
        &self,
        changeset: &mut Vec<ChangeInstruction<'a>>,
        target: &TypeDefinition,
        publish_profile: &PublishProfile,
        _: &Logger,
    ) -> PsqlpackResult<()> {
        if self.name.ne(&target.name) {
            bail!(PublishInvalidOperationError(format!(
                "Types not diffable: {} != {}",
                self.name,
                target.name
            )))
        }
        // We can only diff types of the same kind. Only one type right now, but future proofing.
        match self.kind {
            TypeDefinitionKind::Enum(ref source_values) => {
                match target.kind {
                    TypeDefinitionKind::Enum(ref target_values) => {
                        // Detect if anything needs to be deleted in the target
                        let mut to_delete = target_values
                            .iter()
                            .filter(|v| !source_values.contains(v))
                            .map(|v| {
                                TypeModificationAction::RemoveEnumValue {
                                    value: v.to_owned(),
                                }
                            })
                            .collect::<Vec<_>>();
                        if !to_delete.is_empty() {
                            if publish_profile.generation_options.allow_unsafe_operations {
                                changeset.extend(
                                    to_delete
                                        .drain(..)
                                        .map(|d| ChangeInstruction::ModifyType(self, d)),
                                );
                            } else {
                                bail!(PublishUnsafeOperationError(format!(
                                    "Unable to remove enum value(s) as unsafe operations are disabled: {:?}",
                                    to_delete
                                )))
                            }
                        }

                        // Our working group after items being deleted
                        let mut working = target_values
                            .iter()
                            .filter(|v| source_values.contains(v))
                            .collect::<Vec<_>>();

                        // Detect what needs adding
                        let mut index = 0;
                        for value in source_values {
                            if !working.contains(&value) {
                                if index == 0 {
                                    changeset.push(ChangeInstruction::ModifyType(
                                        self,
                                        TypeModificationAction::AddEnumValueBefore {
                                            value: value.to_owned(),
                                            before: working[0].to_owned(),
                                        },
                                    ));
                                    working.insert(0, value);
                                } else {
                                    changeset.push(ChangeInstruction::ModifyType(
                                        self,
                                        TypeModificationAction::AddEnumValueAfter {
                                            value: value.to_owned(),
                                            after: working[index - 1].to_owned(),
                                        },
                                    ));
                                    working.insert(index, value);
                                }
                            }
                            index += 1;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}


pub struct Delta<'package>(Vec<ChangeInstruction<'package>>);

impl<'package> Delta<'package> {
    pub fn generate(
        log: &Logger,
        package: &'package Package,
        target: Option<Package>,
        target_database_name: String,
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

        // If we always recreate then add a drop and set to false
        let mut target = target;
        if target.is_some() && publish_profile.generation_options.always_recreate_database {
            changeset.push(ChangeInstruction::DropDatabase(
                target_database_name.to_owned(),
            ));
            target = None;
        }

        // If we have the DB we generate an actual change set, else we generate new instructions
        match target {
            Some(target_package) => {
                // Set the connection instruction
                changeset.push(ChangeInstruction::UseDatabase(
                    target_database_name.to_owned(),
                ));

                // Go through each item in order and figure out what to do with it
                for item in &build_order {
                    item.generate(&mut changeset, &target_package, &publish_profile, &log)?;
                }
            }
            None => {
                changeset.push(ChangeInstruction::CreateDatabase(
                    target_database_name.to_owned(),
                ));
                changeset.push(ChangeInstruction::UseDatabase(
                    target_database_name.to_owned(),
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
#[derive(Debug, Serialize)]
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
    ModifyType(&'input TypeDefinition, TypeModificationAction),
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

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub enum TypeModificationAction {
    AddEnumValueBefore { value: String, before: String },
    AddEnumValueAfter { value: String, after: String },
    RemoveEnumValue { value: String },
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
            AddType(ty) => write!(f, "Add type: {}", ty.name),
            ModifyType(ty, ref action) => write!(
                f,
                "Modify type: {} {}",
                ty.name,
                match *action {
                    TypeModificationAction::AddEnumValueBefore { .. } => "(add enum value before)",
                    TypeModificationAction::AddEnumValueAfter { .. } => "(add enum value after)",
                    TypeModificationAction::RemoveEnumValue { .. } => "(remove enum value)",
                }
            ),
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
            ChangeInstruction::AddType(ty) => {
                let mut def = String::new();
                def.push_str(&format!("CREATE TYPE {} AS ", ty.name)[..]);
                match ty.kind {
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
            ChangeInstruction::ModifyType(ty, ref action) => match *action {
                TypeModificationAction::AddEnumValueBefore {
                    ref value,
                    ref before,
                } => format!(
                    "ALTER TYPE {} ADD VALUE '{}' BEFORE '{}'",
                    ty.name,
                    value,
                    before
                ),
                TypeModificationAction::AddEnumValueAfter {
                    ref value,
                    ref after,
                } => format!(
                    "ALTER TYPE {} ADD VALUE '{}' AFTER '{}'",
                    ty.name,
                    value,
                    after
                ),
                TypeModificationAction::RemoveEnumValue { ref value } => format!(
                    "DELETE FROM pg_enum \
                     WHERE enumlabel = '{}' AND \
                     enumtypid = (SELECT oid FROM pg_type WHERE typname = '{}')",
                    value,
                    ty.name
                ),
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

#[cfg(test)]
mod tests {
    use super::*;

    use errors::PsqlpackError;
    use errors::PsqlpackErrorKind::*;
    use model::*;
    use sql::ast;

    use slog::{Discard, Drain, Logger};
    use spectral::prelude::*;

    fn empty_logger() -> Logger {
        Logger::root(Discard.fuse(), o!())
    }

    fn base_type() -> ast::TypeDefinition {
        ast::TypeDefinition {
            name: "colors".into(),
            kind: ast::TypeDefinitionKind::Enum(vec!["red".into(), "green".into(), "blue".into()]),
        }
    }

    #[test]
    fn it_can_add_enum_type() {
        let log = empty_logger();
        let source_type = base_type();

        // Create an empty package (i.e. so it needs to create the type)
        let package = Package::new();
        let publish_profile = PublishProfile::new();

        let mut changeset = Vec::new();
        let result = (&source_type).generate(&mut changeset, &package, &publish_profile, &log);
        assert_that!(result).is_ok();

        // We should have a single instruction to add
        assert_that!(changeset).has_length(1);
        match changeset[0] {
            ChangeInstruction::AddType(ref ty) => {
                assert_that!(ty.name).is_equal_to("colors".to_owned());
                let values = match ty.kind {
                    TypeDefinitionKind::Enum(ref values) => values.clone(),
                };
                assert_that!(values).has_length(3);
                assert_that!(values[0]).is_equal_to("red".to_owned());
                assert_that!(values[1]).is_equal_to("green".to_owned());
                assert_that!(values[2]).is_equal_to("blue".to_owned());
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(changeset[0].to_sql(&log))
            .is_equal_to("CREATE TYPE colors AS ENUM (\n  'red',\n  'green',\n  'blue'\n)".to_owned());
    }

    #[test]
    fn it_ignores_enum_type_if_not_modified() {
        let log = empty_logger();
        let source_type = base_type();

        // Create a package with the type already defined (same as base type)
        let mut package = Package::new();
        package.types.push(base_type());
        let publish_profile = PublishProfile::new();

        let mut changeset = Vec::new();
        let result = (&source_type).generate(&mut changeset, &package, &publish_profile, &log);
        assert_that!(result).is_ok();

        // We should have a single instruction to add
        assert_that!(changeset).is_empty();
    }

    #[test]
    fn it_can_modify_enum_type_by_adding_a_value_to_the_end() {
        let log = empty_logger();
        let source_type = ast::TypeDefinition {
            name: "colors".to_owned(),
            kind: ast::TypeDefinitionKind::Enum(vec![
                "red".to_owned(),
                "green".to_owned(),
                "blue".to_owned(),
                "black".to_owned(),
            ]),
        };

        // Create a package with the type already defined
        let mut package = Package::new();
        package.types.push(base_type());
        let publish_profile = PublishProfile::new();

        let mut changeset = Vec::new();
        let result = (&source_type).generate(&mut changeset, &package, &publish_profile, &log);
        assert_that!(result).is_ok();

        // We should have a single instruction to modify the enum with an additional value
        assert_that!(changeset).has_length(1);
        match changeset[0] {
            ChangeInstruction::ModifyType(ty, ref action) => {
                assert_that!(ty.name).is_equal_to("colors".to_owned());

                // Also, match the action
                match *action {
                    TypeModificationAction::AddEnumValueAfter {
                        ref value,
                        ref after,
                    } => {
                        assert_that!(*value).is_equal_to("black".to_owned());
                        assert_that!(*after).is_equal_to("blue".to_owned());
                    }
                    ref unexpected => panic!("Unexpected enum modification action: {:?}", unexpected),
                }
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(changeset[0].to_sql(&log)).is_equal_to("ALTER TYPE colors ADD VALUE 'black' AFTER 'blue'".to_owned());
    }

    #[test]
    fn it_can_modify_enum_type_by_adding_a_value_to_the_start() {
        let log = empty_logger();
        let source_type = ast::TypeDefinition {
            name: "colors".to_owned(),
            kind: ast::TypeDefinitionKind::Enum(vec![
                "black".to_owned(),
                "red".to_owned(),
                "green".to_owned(),
                "blue".to_owned(),
            ]),
        };

        // Create a package with the type already defined
        let mut package = Package::new();
        package.types.push(base_type());
        let publish_profile = PublishProfile::new();

        let mut changeset = Vec::new();
        let result = (&source_type).generate(&mut changeset, &package, &publish_profile, &log);
        assert_that!(result).is_ok();

        // We should have a single instruction to modify the enum with an additional value
        assert_that!(changeset).has_length(1);
        match changeset[0] {
            ChangeInstruction::ModifyType(ty, ref action) => {
                assert_that!(ty.name).is_equal_to("colors".to_owned());

                // Also, match the action
                match *action {
                    TypeModificationAction::AddEnumValueBefore {
                        ref value,
                        ref before,
                    } => {
                        assert_that!(*value).is_equal_to("black".to_owned());
                        assert_that!(*before).is_equal_to("red".to_owned());
                    }
                    ref unexpected => panic!("Unexpected enum modification action: {:?}", unexpected),
                }
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(changeset[0].to_sql(&log)).is_equal_to("ALTER TYPE colors ADD VALUE 'black' BEFORE 'red'".to_owned());
    }

    #[test]
    fn it_can_modify_enum_type_by_adding_a_value_to_the_middle() {
        let log = empty_logger();
        let source_type = ast::TypeDefinition {
            name: "colors".to_owned(),
            kind: ast::TypeDefinitionKind::Enum(vec![
                "red".to_owned(),
                "green".to_owned(),
                "black".to_owned(),
                "blue".to_owned(),
            ]),
        };

        // Create a package with the type already defined
        let mut package = Package::new();
        package.types.push(base_type());
        let publish_profile = PublishProfile::new();

        let mut changeset = Vec::new();
        let result = (&source_type).generate(&mut changeset, &package, &publish_profile, &log);
        assert_that!(result).is_ok();

        // We should have a single instruction to modify the enum with an additional value
        assert_that!(changeset).has_length(1);
        match changeset[0] {
            ChangeInstruction::ModifyType(ty, ref action) => {
                assert_that!(ty.name).is_equal_to("colors".to_owned());

                // Also, match the action
                match *action {
                    TypeModificationAction::AddEnumValueAfter {
                        ref value,
                        ref after,
                    } => {
                        assert_that!(*value).is_equal_to("black".to_owned());
                        assert_that!(*after).is_equal_to("green".to_owned());
                    }
                    ref unexpected => panic!("Unexpected enum modification action: {:?}", unexpected),
                }
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(changeset[0].to_sql(&log)).is_equal_to("ALTER TYPE colors ADD VALUE 'black' AFTER 'green'".to_owned());
    }

    #[test]
    fn it_can_modify_enum_type_by_modifying_values_and_unsafe_declared() {
        let log = empty_logger();
        let source_type = ast::TypeDefinition {
            name: "colors".to_owned(),
            kind: ast::TypeDefinitionKind::Enum(vec![
                "black".to_owned(),
                "green".to_owned(),
                "blue".to_owned(),
            ]),
        };

        // Create a package with the type already defined
        let mut package = Package::new();
        package.types.push(base_type());
        let publish_profile = PublishProfile {
            version: "1.0".to_owned(),
            generation_options: GenerationOptions {
                always_recreate_database: false,
                allow_unsafe_operations: true,
            },
        };

        let mut changeset = Vec::new();
        let result = (&source_type).generate(&mut changeset, &package, &publish_profile, &log);
        assert_that!(result).is_ok();

        // We should have a single instruction to modify the enum with an additional value
        assert_that!(changeset).has_length(2);

        // Removals first
        match changeset[0] {
            ChangeInstruction::ModifyType(ty, ref action) => {
                assert_that!(ty.name).is_equal_to("colors".to_owned());

                // Also, match the action
                match *action {
                    TypeModificationAction::RemoveEnumValue { ref value } => {
                        assert_that!(*value).is_equal_to("red".to_owned());
                    }
                    ref unexpected => panic!("Unexpected enum modification action: {:?}", unexpected),
                }
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }
        // Check the SQL generation
        assert_that!(changeset[0].to_sql(&log)).is_equal_to(
            "DELETE FROM pg_enum \
             WHERE enumlabel = 'red' AND \
             enumtypid = (SELECT oid FROM pg_type WHERE typname = 'colors')"
                .to_owned(),
        );

        // Additions second
        match changeset[1] {
            ChangeInstruction::ModifyType(ty, ref action) => {
                assert_that!(ty.name).is_equal_to("colors".to_owned());

                // Also, match the action
                match *action {
                    TypeModificationAction::AddEnumValueBefore {
                        ref value,
                        ref before,
                    } => {
                        assert_that!(*value).is_equal_to("black".to_owned());
                        assert_that!(*before).is_equal_to("green".to_owned());
                    }
                    ref unexpected => panic!("Unexpected enum modification action: {:?}", unexpected),
                }
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }
        // Check the SQL generation
        assert_that!(changeset[1].to_sql(&log)).is_equal_to("ALTER TYPE colors ADD VALUE 'black' BEFORE 'green'".to_owned());
    }

    #[test]
    fn it_rejects_modifying_enum_type_when_modifying_values_by_default() {
        let log = empty_logger();
        let source_type = ast::TypeDefinition {
            name: "colors".to_owned(),
            kind: ast::TypeDefinitionKind::Enum(vec![
                "black".to_owned(),
                "green".to_owned(),
                "blue".to_owned(),
            ]),
        };

        // Create a package with the type already defined
        let mut package = Package::new();
        package.types.push(base_type());
        let publish_profile = PublishProfile::new();

        let mut changeset = Vec::new();
        let result = (&source_type).generate(&mut changeset, &package, &publish_profile, &log);
        assert_that!(result).is_err();
        match result.err().unwrap() {
            PsqlpackError(PublishUnsafeOperationError(_), _) => {}
            unexpected => panic!(
                "Expected unsafe operation error however saw {:?}",
                unexpected
            ),
        };
    }

    #[test]
    fn it_can_modify_enum_type_by_removing_values_and_unsafe_declared() {
        let log = empty_logger();
        let source_type = ast::TypeDefinition {
            name: "colors".to_owned(),
            kind: ast::TypeDefinitionKind::Enum(vec!["green".to_owned(), "blue".to_owned()]),
        };

        // Create a package with the type already defined
        let mut package = Package::new();
        package.types.push(base_type());
        let publish_profile = PublishProfile {
            version: "1.0".to_owned(),
            generation_options: GenerationOptions {
                always_recreate_database: false,
                allow_unsafe_operations: true,
            },
        };

        let mut changeset = Vec::new();
        let result = (&source_type).generate(&mut changeset, &package, &publish_profile, &log);
        assert_that!(result).is_ok();

        // We should have a single instruction to modify the enum with an additional value
        assert_that!(changeset).has_length(1);

        // Removals first
        match changeset[0] {
            ChangeInstruction::ModifyType(ty, ref action) => {
                assert_that!(ty.name).is_equal_to("colors".to_owned());

                // Also, match the action
                match *action {
                    TypeModificationAction::RemoveEnumValue { ref value } => {
                        assert_that!(*value).is_equal_to("red".to_owned());
                    }
                    ref unexpected => panic!("Unexpected enum modification action: {:?}", unexpected),
                }
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }
        // Check the SQL generation
        assert_that!(changeset[0].to_sql(&log)).is_equal_to(
            "DELETE FROM pg_enum \
             WHERE enumlabel = 'red' AND \
             enumtypid = (SELECT oid FROM pg_type WHERE typname = 'colors')"
                .to_owned(),
        );
    }
    /*
    //TODO: Implement this when we have provision for dropping objects
    #[test]
    fn it_can_drop_enum_type() {
        panic!("Not implemented");
    }
*/
}
