use std::collections::HashSet;
use std::fmt;
use std::io::Write;
use std::path::Path;
use std::fs::File;

use slog::Logger;
use serde_json;

use sql::ast::*;
use connection::Connection;
use model::{Node, Package, PublishProfile, Toggle};
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
            DbObject::Column(table, column) => LinkedColumn { table: &table, column: &column }.generate(changeset, target, publish_profile, log),
            DbObject::Constraint(table, constraint) => LinkedTableConstraint { table: &table, constraint: &constraint }.generate(changeset, target, publish_profile, log),
            DbObject::Extension(extension) => extension.generate(changeset, target, publish_profile, log),
            DbObject::Function(function) => function.generate(changeset, target, publish_profile, log),
            DbObject::Schema(schema) => schema.generate(changeset, target, publish_profile, log),
            DbObject::Script(script) => script.generate(changeset, target, publish_profile, log),
            DbObject::Table(table) => table.generate(changeset, target, publish_profile, log),
            DbObject::Type(ty) => ty.generate(changeset, target, publish_profile, log),
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
        publish_profile: &PublishProfile,
        _log: &Logger,
    ) -> PsqlpackResult<()> {
        let table_result = target.tables.iter().find(|t| t.name == self.name);
        if let Some(target_table) = table_result {
            // We check for column removals here
            for tgt in target_table.columns.iter() {
                if !self.columns.iter().any(|src| tgt.name.eq(&src.name)) {
                    // Column in target but not in source
                    match publish_profile.generation_options.drop_columns {
                        Toggle::Allow => changeset.push(ChangeInstruction::RemoveColumn(self, tgt.name.to_owned())),
                        Toggle::Error => {
                            bail!(PublishUnsafeOperationError(format!(
                                "Unable to drop column as dropping columns is currently disabled: {}",
                                tgt.name
                            )));
                        }
                        _ => {}
                    }
                }
            }

            // We also check for table constraint removals here
            for tgt in target_table.constraints.iter() {
                if !self.constraints.iter().any(|src| tgt.name().eq(src.name())) {
                    let remove_ok = match tgt {
                        TableConstraint::Primary { .. } => {
                            match publish_profile.generation_options.drop_primary_key_constraints {
                                Toggle::Allow => true,
                                Toggle::Ignore => false,
                                Toggle::Error => bail!(PublishUnsafeOperationError(format!(
                                    "Unable to drop constraint as dropping PKs is currently disabled: {}",
                                    tgt.name()
                                )))
                            }
                        }
                        TableConstraint::Foreign { .. } => {
                            match publish_profile.generation_options.drop_foreign_key_constraints {
                                Toggle::Allow => true,
                                Toggle::Ignore => false,
                                Toggle::Error => bail!(PublishUnsafeOperationError(format!(
                                    "Unable to drop constraint as dropping FKs is currently disabled: {}",
                                    tgt.name()
                                )))
                            }
                        }
                    };
                    if remove_ok {
                        changeset.push(ChangeInstruction::RemoveConstraint(self, tgt.name().to_owned()));
                    }
                }
            }
        } else {
            changeset.push(ChangeInstruction::AddTable(self));
        }
        Ok(())
    }
}

struct LinkedColumn<'a> {
    table: &'a TableDefinition,
    column: &'a ColumnDefinition
}

impl<'a> Diffable<'a, Package> for LinkedColumn<'a> {
    fn generate(
        &self,
        changeset: &mut Vec<ChangeInstruction<'a>>,
        target: &Package,
        _publish_profile: &PublishProfile,
        _log: &Logger,
    ) -> PsqlpackResult<()> {
        // We only generate items here if the table doesn't exist (for the time being)
        // We should consider if we want to just generate empty tables and then be consistent adding
        let table_result = target.tables.iter().find(|t| t.name == self.table.name);
        if let Some(target_table) = table_result {

            // Check if the column exists on the target
            let target_column = target_table.columns.iter().find(|tgt| tgt.name.eq(&self.column.name));
            if let Some(target_column) = target_column {
                // Check the type
                if !self.column.sql_type.eq(&target_column.sql_type) {
                    changeset.push(ChangeInstruction::ModifyColumnType(self.table, &self.column));
                }

                // Check column constraints
                let src_set: HashSet<_> = self.column.constraints.iter().cloned().collect();
                let target_set: HashSet<_> = target_column.constraints.iter().cloned().collect();

                // target_set - src_set (e.g. adding new constraints)
                for x in target_set.difference(&src_set) {
                    match *x {
                        ColumnConstraint::Null | ColumnConstraint::NotNull => changeset.push(ChangeInstruction::ModifyColumnNull(self.table, &self.column)),
                        ColumnConstraint::Default(_) => changeset.push(ChangeInstruction::ModifyColumnDefault(self.table, &self.column)),
                        ColumnConstraint::Unique => changeset.push(ChangeInstruction::ModifyColumnUniqueConstraint(self.table, &self.column)),
                        ColumnConstraint::PrimaryKey => changeset.push(ChangeInstruction::ModifyColumnPrimaryKeyConstraint(self.table, &self.column)),
                    }
                }

                // TODO: src_sec - target_set (e.g. what column constraints have been removed)

            } else {
                // Doesn't exist, add it
                changeset.push(ChangeInstruction::AddColumn(self.table, &self.column));
            }
        }
        Ok(())
    }
}

struct LinkedTableConstraint<'a> {
    table: &'a TableDefinition,
    constraint: &'a TableConstraint
}

impl<'a> Diffable<'a, Package> for LinkedTableConstraint<'a> {
    fn generate(
        &self,
        changeset: &mut Vec<ChangeInstruction<'a>>,
        target: &Package,
        publish_profile: &PublishProfile,
        _log: &Logger,
    ) -> PsqlpackResult<()> {
        fn vec_different<T: PartialEq>(src: &Vec<T>, tgt: &Vec<T>) -> bool {
            if src.len() != tgt.len() {
                return true;
            }
            for i in 0..src.len() {
                if src[i].ne(&tgt[i]) {
                    return true;
                }
            }
            false
        }
        fn optional_vec_different<T: PartialEq>(src: &Option<Vec<T>>, tgt: &Option<Vec<T>>) -> bool {
            if src.is_some() && tgt.is_some() {
                vec_different(src.as_ref().unwrap(), tgt.as_ref().unwrap())
            } else {
                src.is_none() ^ tgt.is_none()
            }
        }

        // If the table doesn't exist in the target we assume it will, so we just add
        let table_result = target.tables.iter().find(|t| t.name == self.table.name);
        if let Some(target_table) = table_result {
            // Check if the constraint exists on the target - this is a basic comparison of name
            let target_constraint = target_table.constraints.iter().find(|tgt| tgt.name().eq(self.constraint.name()));
            if let Some(target_constraint) = target_constraint {
                // Exists on target - compare to see if it's equal
                // TODO: After we have a min defined Postgres version we may be able to use ALTER in some cases as supported
                let has_changed = match *self.constraint {
                        TableConstraint::Primary { name: _, ref columns, ref parameters } => {
                            let src_columns = columns;
                            let src_parameters = parameters;
                            match target_constraint {
                                TableConstraint::Primary { name: _, ref columns, ref parameters } => {
                                    vec_different(src_columns, columns) ||
                                        optional_vec_different(src_parameters, parameters)
                                },
                                TableConstraint::Foreign { .. } => true,
                            }
                        }
                        TableConstraint::Foreign {
                            name: _,
                            ref columns,
                            ref ref_table,
                            ref ref_columns,
                            ref match_type,
                            ref events
                        } => {
                            let src_columns = columns;
                            let src_ref_table = ref_table;
                            let src_ref_columns = ref_columns;
                            let src_match_type = match_type;
                            let src_events = events;
                            match target_constraint {
                                TableConstraint::Primary { .. } => true,
                                TableConstraint::Foreign {
                                    name: _,
                                    ref columns,
                                    ref ref_table,
                                    ref ref_columns,
                                    ref match_type,
                                    ref events
                                } => {
                                    let match_type_different = if src_match_type.is_some() && match_type.is_some() {
                                        src_match_type.as_ref().unwrap().ne(match_type.as_ref().unwrap())
                                    } else {
                                        src_match_type.is_none() ^ match_type.is_none()
                                    };
                                    match_type_different ||
                                        vec_different(src_columns, columns) ||
                                        src_ref_table.ne(ref_table) ||
                                        vec_different(src_ref_columns, ref_columns) ||
                                        optional_vec_different(src_events, events)
                                }
                            }
                        }
                    };
                if has_changed {
                    let remove_ok = match self.constraint {
                        TableConstraint::Primary { .. } => {
                            match publish_profile.generation_options.drop_primary_key_constraints {
                                Toggle::Allow => true,
                                Toggle::Ignore => false,
                                Toggle::Error => bail!(PublishUnsafeOperationError(format!(
                                    "Unable to modify constraint as dropping PKs is currently disabled: {}",
                                    self.constraint.name()
                                )))
                            }
                        }
                        TableConstraint::Foreign { .. } => {
                            match publish_profile.generation_options.drop_foreign_key_constraints {
                                Toggle::Allow => true,
                                Toggle::Ignore => false,
                                Toggle::Error => bail!(PublishUnsafeOperationError(format!(
                                    "Unable to modify constraint as dropping FKs is currently disabled: {}",
                                    self.constraint.name()
                                )))
                            }
                        }
                    };
                    if remove_ok {
                        changeset.push(ChangeInstruction::RemoveConstraint(self.table, self.constraint.name().to_owned()));
                        changeset.push(ChangeInstruction::AddConstraint(self.table, self.constraint));
                    }
                }
            } else {
                // Doesn't exist, add it
                changeset.push(ChangeInstruction::AddConstraint(self.table, &self.constraint));
            }

        } else {
            changeset.push(ChangeInstruction::AddConstraint(self.table, &self.constraint));
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
                            match publish_profile.generation_options.drop_enum_values {
                                Toggle::Allow => {
                                    changeset.extend(
                                        to_delete
                                            .drain(..)
                                            .map(|d| ChangeInstruction::ModifyType(self, d)),
                                    );
                                }
                                Toggle::Error => {
                                    bail!(PublishUnsafeOperationError(format!(
                                        "Unable to remove enum value(s) as unsafe operations are disabled: {:?}",
                                        to_delete
                                    )));
                                }
                                _ => {}
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

#[derive(Debug)]
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

        // If we always recreate then add a drop and set to false
        let mut target = target;
        if target.is_some() && publish_profile.generation_options.always_recreate_database {
            changeset.push(ChangeInstruction::DropDatabase(
                target_database_name.to_owned(),
            ));
            target = None;
        }

        // For an empty database use an empty package, but also push a CREATE DB instruction
        let target_package = match target {
            Some(target_package) => target_package,
            None => {
                changeset.push(ChangeInstruction::CreateDatabase(
                    target_database_name.to_owned(),
                ));
                Package::new()
            }
        };

        // Set the connection instruction
        changeset.push(ChangeInstruction::UseDatabase(
            target_database_name.to_owned(),
        ));

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

        // Drop tables first - first figure out if there are any to drop
        for table in &target_package.tables {
            if !package.tables.iter().any(|t| t.name.eq(&table.name)) {
                match publish_profile.generation_options.drop_tables {
                    Toggle::Allow => changeset.push(ChangeInstruction::RemoveTable(table.name.to_string())),
                    Toggle::Error => bail!(
                                        PublishUnsafeOperationError(
                                            format!("Attempted to remove table {} however dropping tables is currently disabled", table.name)
                                        )
                                    ),
                    _ => {}
                }
            }
        }

        // Drop functions next - first figure out if there are any to drop
        for function in &target_package.functions {
            if !package.functions.iter().any(|t| t.name.eq(&function.name)) {
                match publish_profile.generation_options.drop_functions {
                    Toggle::Allow => changeset.push(ChangeInstruction::DropFunction(function.name.to_string())),
                    Toggle::Error => bail!(
                                        PublishUnsafeOperationError(
                                            format!("Attempted to remove function {} however dropping functions is currently disabled", function.name)
                                        )
                                    ),
                    _ => {}
                }
            }
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

        // Go through each item in order and figure out what to do with it
        for item in &build_order {
            item.generate(&mut changeset, &target_package, &publish_profile, &log)?;
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
    ModifyColumnType(&'input TableDefinition, &'input ColumnDefinition),
    ModifyColumnNull(&'input TableDefinition, &'input ColumnDefinition),
    ModifyColumnDefault(&'input TableDefinition, &'input ColumnDefinition),
    ModifyColumnUniqueConstraint(&'input TableDefinition, &'input ColumnDefinition),
    ModifyColumnPrimaryKeyConstraint(&'input TableDefinition, &'input ColumnDefinition),
    RemoveColumn(&'input TableDefinition, String),

    // Constraints
    AddConstraint(&'input TableDefinition, &'input TableConstraint),
    RemoveConstraint(&'input TableDefinition, String),

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
                "Modify type by {}: {}",
                match *action {
                    TypeModificationAction::AddEnumValueBefore { .. } => "inserting an enum value",
                    TypeModificationAction::AddEnumValueAfter { .. } => "inserting an enum value",
                    TypeModificationAction::RemoveEnumValue { .. } => "removing enum value",
                },
                ty.name
            ),
            RemoveType(ref type_name) => write!(f, "Remove type: {}", type_name),

            // Tables
            AddTable(table) => write!(f, "Add table: {}", table.name),
            RemoveTable(ref table_name) => write!(f, "Remove table: {}", table_name),

            // Columns
            AddColumn(table, column) => write!(f, "Add column: {} to table: {}", column.name, table.name),
            ModifyColumnType(table, column) => write!(f, "Modify type for column: {} on table: {}", column.name, table.name),
            ModifyColumnNull(table, column) => write!(f, "Modify null for column: {} on table: {}", column.name, table.name),
            ModifyColumnDefault(table, column) => write!(f, "Modify default for column: {} on table: {}", column.name, table.name),
            ModifyColumnUniqueConstraint(table, column) => write!(f, "Modify unique constraint for column: {} on table: {}", column.name, table.name),
            ModifyColumnPrimaryKeyConstraint(table, column) => write!(f, "Modify primary key constraint for column: {} on table: {}", column.name, table.name),
            RemoveColumn(table, ref column_name) => write!(f, "Remove column: {} on table: {}", column_name, table.name),

            // Constraints
            AddConstraint(table, constraint) => write!(
                f,
                "Add constraint: {} to table: {}",
                constraint.name(),
                table.name
            ),
            RemoveConstraint(table, ref name) => write!(
                f,
                "Remove constraint: {} to table: {}",
                name,
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
            ChangeInstruction::DropDatabase(ref db) => {
                let mut drop = String::new();
                drop.push_str("SELECT pg_terminate_backend(pg_stat_activity.pid) ");
                drop.push_str("FROM pg_stat_activity ");
                drop.push_str(&format!("WHERE pg_stat_activity.datname = '{}';", db));
                drop.push_str(&format!("DROP DATABASE {}", db));
                drop
            },
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
            ChangeInstruction::AddTable(def) => {
                let mut instr = String::new();
                instr.push_str(&format!("CREATE TABLE {} (", def.name));
                for (position, column) in def.columns.iter().enumerate() {
                    if position > 0 {
                        instr.push_str(",");
                    }
                    instr.push_str("\n\t");
                    instr.push_str(&format!("{} {}", column.name, column.sql_type));
                    for constraint in column.constraints.iter() {
                        match *constraint {
                            ColumnConstraint::Default(ref any_type) => instr.push_str(&format!(" DEFAULT {}", any_type)),
                            ColumnConstraint::NotNull => instr.push_str(" NOT NULL"),
                            ColumnConstraint::Null => instr.push_str(" NULL"),
                            ColumnConstraint::Unique => instr.push_str(" UNIQUE"),
                            ColumnConstraint::PrimaryKey => instr.push_str(" PRIMARY KEY"),
                        }
                    }
                }
                // Table constraints are added later
                /*
                for constraint in def.constraints.iter() {
                    instr.push_str(",\n\t");
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
                }
                */
                instr.push_str("\n)");
                instr
            }

            // Column level
            ChangeInstruction::AddColumn(table, column) => {
                let mut instr = String::new();
                instr.push_str(&format!("ALTER TABLE {} ADD COLUMN {} {}", table.name, column.name, column.sql_type));
                for constraint in column.constraints.iter() {
                    match *constraint {
                        ColumnConstraint::Default(ref any_type) => instr.push_str(&format!(" DEFAULT {}", any_type)),
                        ColumnConstraint::NotNull => instr.push_str(" NOT NULL"),
                        ColumnConstraint::Null => instr.push_str(" NULL"),
                        ColumnConstraint::Unique => instr.push_str(" UNIQUE"),
                        ColumnConstraint::PrimaryKey => instr.push_str(" PRIMARY KEY"),
                    }
                }
                instr
            }
            ChangeInstruction::ModifyColumnType(table, column) => {
                format!("ALTER TABLE {} ALTER COLUMN {} TYPE {}", table.name, column.name, column.sql_type)
            }
            ChangeInstruction::ModifyColumnNull(table, column) => {
                for constraint in column.constraints.iter() {
                    match *constraint {
                        ColumnConstraint::NotNull => {
                            return format!("ALTER TABLE {} ALTER COLUMN {} SET NOT NULL", table.name, column.name);
                        }
                        ColumnConstraint::Null => {
                            return format!("ALTER TABLE {} ALTER COLUMN {} DROP NOT NULL", table.name, column.name);
                        }
                        _ => {}
                    }
                }
                error!(log, "Expected to modify column null constraint for {}.{}", table.name, column.name);
                "".to_owned()
            }
            ChangeInstruction::ModifyColumnDefault(table, column) => {
                for constraint in column.constraints.iter() {
                    match *constraint {
                        ColumnConstraint::Default(ref any_type) => {
                            return format!(";\nALTER TABLE {} ALTER COLUMN {} SET DEFAULT {}", table.name, column.name, any_type);
                        }
                        _ => {}
                    }
                }
                error!(log, "Expected to modify column default constraint for {}.{}", table.name, column.name);
                "".to_owned()
            }
            ChangeInstruction::ModifyColumnUniqueConstraint(table, column) => {
                for constraint in column.constraints.iter() {
                    match *constraint {
                        ColumnConstraint::Unique => {
                            // TODO: These have to be table level constraints. Ignore??
                            warn!(log, "Ignoring UNIQUE column constraint for {}.{}", table.name, column.name);
                        }
                        _ => {}
                    }
                }
                "".to_owned()
            }
            ChangeInstruction::ModifyColumnPrimaryKeyConstraint(table, column) => {
                for constraint in column.constraints.iter() {
                    match *constraint {
                        ColumnConstraint::PrimaryKey => {
                            // TODO: These have to be table level constraints. Ignore??
                            warn!(log, "Ignoring PRIMARY KEY column constraint for {}.{}", table.name, column.name);
                        }
                        _ => {}
                    }
                }
                "".to_owned()
            }
            ChangeInstruction::RemoveColumn(table, ref column_name) => {
                format!("ALTER TABLE {} DROP COLUMN {}", table.name, column_name)
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

            ChangeInstruction::RemoveConstraint(table, ref name) => {
                format!("ALTER TABLE {}\nDROP CONSTRAINT {}", table.name, name)
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
        let existing_database = Package::new();
        let publish_profile = PublishProfile::default();

        let mut changeset = Vec::new();
        let result = (&source_type).generate(&mut changeset, &existing_database, &publish_profile, &log);
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
        let mut existing_database = Package::new();
        existing_database.types.push(base_type());
        let publish_profile = PublishProfile::default();

        let mut changeset = Vec::new();
        let result = (&source_type).generate(&mut changeset, &existing_database, &publish_profile, &log);
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
        let mut existing_database = Package::new();
        existing_database.types.push(base_type());
        let publish_profile = PublishProfile::default();

        let mut changeset = Vec::new();
        let result = (&source_type).generate(&mut changeset, &existing_database, &publish_profile, &log);
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
        let mut existing_database = Package::new();
        existing_database.types.push(base_type());
        let publish_profile = PublishProfile::default();

        let mut changeset = Vec::new();
        let result = (&source_type).generate(&mut changeset, &existing_database, &publish_profile, &log);
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
        let mut existing_database = Package::new();
        existing_database.types.push(base_type());
        let publish_profile = PublishProfile::default();

        let mut changeset = Vec::new();
        let result = (&source_type).generate(&mut changeset, &existing_database, &publish_profile, &log);
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
        let mut existing_database = Package::new();
        existing_database.types.push(base_type());
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.drop_enum_values = Toggle::Allow;

        let mut changeset = Vec::new();
        let result = (&source_type).generate(&mut changeset, &existing_database, &publish_profile, &log);
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
        let mut existing_database = Package::new();
        existing_database.types.push(base_type());
        let publish_profile = PublishProfile::default();

        let mut changeset = Vec::new();
        let result = (&source_type).generate(&mut changeset, &existing_database, &publish_profile, &log);
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
        let mut existing_database = Package::new();
        existing_database.types.push(base_type());
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.drop_enum_values = Toggle::Allow;

        let mut changeset = Vec::new();
        let result = (&source_type).generate(&mut changeset, &existing_database, &publish_profile, &log);
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

    fn base_table() -> ast::TableDefinition {
        ast::TableDefinition {
            name: ObjectName {
                schema: Some("my".to_owned()),
                name: "contacts".to_owned(),
            },
            columns: vec![
                ColumnDefinition {
                    name: "id".to_owned(),
                    sql_type: SqlType::Simple(SimpleSqlType::Serial),
                    constraints: vec![
                        ColumnConstraint::NotNull,
                        ColumnConstraint::PrimaryKey,
                    ],
                },
                ColumnDefinition {
                    name: "company_id".to_owned(),
                    sql_type: SqlType::Simple(SimpleSqlType::BigInteger),
                    constraints: vec![
                        ColumnConstraint::NotNull,
                    ],
                },
                ColumnDefinition {
                    name: "first_name".to_owned(),
                    sql_type: SqlType::Simple(SimpleSqlType::VariableLengthString(100)),
                    constraints: vec![
                        ColumnConstraint::NotNull,
                    ],
                },
            ],
            constraints: Vec::new(),
        }
    }

    #[test]
    fn it_can_add_new_table() {
        let log = empty_logger();
        let source_table = base_table();

        // Create an empty database
        let existing_database = Package::new();
        let publish_profile = PublishProfile::default();

        let mut changeset = Vec::new();
        let result = (&source_table).generate(&mut changeset, &existing_database, &publish_profile, &log);
        assert_that!(result).is_ok();

        // We should have a single instruction to create a new table
        assert_that!(changeset).has_length(1);
        match changeset[0] {
            ChangeInstruction::AddTable(ref table) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                assert_that!(table.columns).has_length(3);
                assert_that!(table.columns[0].name).is_equal_to("id".to_owned());
                assert_that!(table.columns[1].name).is_equal_to("company_id".to_owned());
                assert_that!(table.columns[2].name).is_equal_to("first_name".to_owned());
                assert_that!(table.constraints).is_empty();
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(changeset[0].to_sql(&log))
            .is_equal_to("CREATE TABLE my.contacts (\n\
                \tid serial NOT NULL PRIMARY KEY,\n\
                \tcompany_id bigint NOT NULL,\n\
                \tfirst_name varchar(100) NOT NULL\n\
                )".to_owned());
    }

    #[test]
    fn it_can_add_column_to_existing_table() {
        let log = empty_logger();
        let mut source_table = base_table();
        source_table.columns.push(ColumnDefinition {
                name: "last_name".to_owned(),
                sql_type: SqlType::Simple(SimpleSqlType::VariableLengthString(100)),
                constraints: vec![
                    ColumnConstraint::NotNull,
                ],
            });

        // Create a database with the base table already defined.
        let mut existing_database = Package::new();
        existing_database.tables.push(base_table());
        let publish_profile = PublishProfile::default();

        let mut changeset = Vec::new();
        // First, check that source table changes do nothing
        let _ = (&source_table).generate(&mut changeset, &existing_database, &publish_profile, &log);
        assert_that!(changeset).is_empty();

        // Now we check with a linked column
        let result = LinkedColumn { table: &source_table, column: &source_table.columns.last().unwrap()}
                        .generate(&mut changeset, &existing_database, &publish_profile, &log);
        assert_that!(result).is_ok();

        // We should have a single instruction to create a new table
        assert_that!(changeset).has_length(1);
        match changeset[0] {
            ChangeInstruction::AddColumn(ref table, ref column) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                assert_that!(column.name).is_equal_to("last_name".to_owned());
                assert_that!(column.sql_type).is_equal_to(SqlType::Simple(SimpleSqlType::VariableLengthString(100)));
                assert_that!(column.constraints).has_length(1);
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(changeset[0].to_sql(&log))
            .is_equal_to("ALTER TABLE my.contacts ADD COLUMN last_name varchar(100) NOT NULL".to_owned());
    }

    #[test]
    fn it_can_widen_column_on_existing_table() {
        let log = empty_logger();
        let mut source_table = base_table();
        source_table.columns.push(
            ColumnDefinition {
                name: "last_name".to_owned(),
                sql_type: SqlType::Simple(SimpleSqlType::VariableLengthString(200)),
                constraints: vec![
                    ColumnConstraint::NotNull,
                ],
            });

        // Create a database with the base table already defined.
        let mut existing_database = Package::new();
        let mut existing_table = base_table();
        existing_table.columns.push(
            ColumnDefinition {
                name: "last_name".to_owned(),
                sql_type: SqlType::Simple(SimpleSqlType::VariableLengthString(100)),
                constraints: vec![
                    ColumnConstraint::NotNull,
                ],
            });

        existing_database.tables.push(existing_table);
        let publish_profile = PublishProfile::default();

        let mut changeset = Vec::new();
        // First, check that source table changes do nothing
        let _ = (&source_table).generate(&mut changeset, &existing_database, &publish_profile, &log);
        assert_that!(changeset).is_empty();

        // Now we check with a linked column
        let result = LinkedColumn { table: &source_table, column: &source_table.columns.last().unwrap()}
                        .generate(&mut changeset, &existing_database, &publish_profile, &log);
        assert_that!(result).is_ok();

        // We should have a single instruction to create a new table
        assert_that!(changeset).has_length(1);
        match changeset[0] {
            ChangeInstruction::ModifyColumnType(ref table, ref column) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                assert_that!(column.name).is_equal_to("last_name".to_owned());
                assert_that!(column.sql_type).is_equal_to(SqlType::Simple(SimpleSqlType::VariableLengthString(200)));
                assert_that!(column.constraints).has_length(1);
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(changeset[0].to_sql(&log))
            .is_equal_to("ALTER TABLE my.contacts ALTER COLUMN last_name TYPE varchar(200)".to_owned());
    }

    #[test]
    fn it_can_drop_column_on_existing_table() {
        let log = empty_logger();
        let source_table = base_table();

        // Create a database with the base table already defined.
        let mut existing_database = Package::new();
        let mut existing_table = base_table();
        existing_table.columns.push(
            ColumnDefinition {
                name: "last_name".to_owned(),
                sql_type: SqlType::Simple(SimpleSqlType::VariableLengthString(100)),
                constraints: vec![
                    ColumnConstraint::NotNull,
                ],
            });

        existing_database.tables.push(existing_table);
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.drop_columns = Toggle::Allow;

        let mut changeset = Vec::new();
        let result = (&source_table).generate(&mut changeset, &existing_database, &publish_profile, &log);
        assert_that!(result).is_ok();

        // We should have a single instruction to create a new table
        assert_that!(changeset).has_length(1);
        match changeset[0] {
            ChangeInstruction::RemoveColumn(ref table, ref column_name) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                assert_that!(*column_name).is_equal_to("last_name".to_owned());
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(changeset[0].to_sql(&log))
            .is_equal_to("ALTER TABLE my.contacts DROP COLUMN last_name".to_owned());
    }

    #[test]
    fn it_can_add_a_new_primary_key() {
        let log = empty_logger();
        let mut source_table = base_table();
        source_table.constraints.push(TableConstraint::Primary {
                name: "pk_my_contacts_id".to_owned(),
                columns: vec!["id".into()],
                parameters: Some(vec![IndexParameter::FillFactor(80)]),
            });

        // Create a database with the base table already defined.
        let mut existing_database = Package::new();
        existing_database.tables.push(base_table());
        let publish_profile = PublishProfile::default();

        let mut changeset = Vec::new();
        // First, check that source table changes do nothing
        let _ = (&source_table).generate(&mut changeset, &existing_database, &publish_profile, &log);
        assert_that!(changeset).is_empty();

        // Now we check with a linked table constraint
        let result = LinkedTableConstraint { table: &source_table, constraint: &source_table.constraints.first().unwrap()}
                        .generate(&mut changeset, &existing_database, &publish_profile, &log);
        assert_that!(result).is_ok();

        // We should have a single instruction to add a constraint
        assert_that!(changeset).has_length(1);
        match changeset[0] {
            ChangeInstruction::AddConstraint(ref table, ref constraint) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                match *constraint {
                    TableConstraint::Primary { name, columns, parameters } => {
                        assert_that!(*name).is_equal_to("pk_my_contacts_id".to_owned());
                        assert_that!(*columns).has_length(1);
                        assert_that!(columns.iter()).contains("id".to_owned());
                        assert_that!(*parameters).is_some().has_length(1);
                    }
                    unxpected => panic!("Unexpected constraint: {:?}", unxpected),
                }
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(changeset[0].to_sql(&log))
            .is_equal_to("ALTER TABLE my.contacts\nADD CONSTRAINT pk_my_contacts_id PRIMARY KEY (id) WITH (FILLFACTOR=80)".to_owned());
    }

    #[test]
    fn it_can_remove_an_existing_primary_key() {
        let log = empty_logger();
        let source_table = base_table();

        // Create a database with the base table already defined.
        let mut existing_database = Package::new();
        let mut existing_table = base_table();
        existing_table.constraints.push(TableConstraint::Primary {
            name: "pk_my_contacts_id".to_owned(),
            columns: vec!["id".into()],
            parameters: Some(vec![IndexParameter::FillFactor(80)]),
        });
        existing_database.tables.push(existing_table);
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.drop_primary_key_constraints = Toggle::Allow;

        let mut changeset = Vec::new();
        // This changeset gets generated at the table level
        let result = (&source_table).generate(&mut changeset, &existing_database, &publish_profile, &log);
        assert_that!(result).is_ok();

        // We should have a single instruction to remove the constraint
        assert_that!(changeset).has_length(1);
        match changeset[0] {
            ChangeInstruction::RemoveConstraint(ref table, ref name) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                assert_that!(*name).is_equal_to("pk_my_contacts_id".to_owned());
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(changeset[0].to_sql(&log))
            .is_equal_to("ALTER TABLE my.contacts\nDROP CONSTRAINT pk_my_contacts_id".to_owned());
    }

    #[test]
    fn it_can_modify_an_existing_primary_key() {
        let log = empty_logger();
        let mut source_table = base_table();
        source_table.constraints.push(TableConstraint::Primary {
                name: "pk_my_contacts_id".to_owned(),
                columns: vec!["id".into()],
                parameters: Some(vec![IndexParameter::FillFactor(80)]),
            });

        // Create a database with the base table already defined.
        let mut existing_database = Package::new();
        let mut existing_table = base_table();
        existing_table.constraints.push(TableConstraint::Primary {
                name: "pk_my_contacts_id".to_owned(),
                columns: vec!["id".into(), "company_id".into()],
                parameters: Some(vec![IndexParameter::FillFactor(80)]),
            });
        existing_database.tables.push(existing_table);
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.drop_primary_key_constraints = Toggle::Allow;

        let mut changeset = Vec::new();
        // First, check that source table changes do nothing
        let _ = (&source_table).generate(&mut changeset, &existing_database, &publish_profile, &log);
        assert_that!(changeset).is_empty();

        // Now we check with a linked table constraint
        let result = LinkedTableConstraint { table: &source_table, constraint: &source_table.constraints.first().unwrap()}
                        .generate(&mut changeset, &existing_database, &publish_profile, &log);
        assert_that!(result).is_ok();

        // Primary keys cannot be altered, so we drop/create
        assert_that!(changeset).has_length(2);
        match changeset[0] {
            ChangeInstruction::RemoveConstraint(ref table, ref name) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                assert_that!(*name).is_equal_to("pk_my_contacts_id".to_owned());
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }
        match changeset[1] {
            ChangeInstruction::AddConstraint(ref table, ref constraint) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                match *constraint {
                    TableConstraint::Primary { name, columns, parameters } => {
                        assert_that!(*name).is_equal_to("pk_my_contacts_id".to_owned());
                        assert_that!(*columns).has_length(1);
                        assert_that!(columns.iter()).contains("id".to_owned());
                        assert_that!(*parameters).is_some().has_length(1);
                    }
                    unxpected => panic!("Unexpected constraint: {:?}", unxpected),
                }
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(changeset[0].to_sql(&log))
            .is_equal_to("ALTER TABLE my.contacts\nDROP CONSTRAINT pk_my_contacts_id".to_owned());
        assert_that!(changeset[1].to_sql(&log))
            .is_equal_to("ALTER TABLE my.contacts\nADD CONSTRAINT pk_my_contacts_id PRIMARY KEY (id) WITH (FILLFACTOR=80)".to_owned());
    }

    #[test]
    fn it_can_add_a_new_foreign_key() {
        let log = empty_logger();
        let mut source_table = base_table();
        source_table.constraints.push(TableConstraint::Foreign {
                name: "fk_my_contacts_my_companies".to_owned(),
                columns: vec!["company_id".into()],
                ref_table: ObjectName { schema: Some("my".into()), name: "companies".into() },
                ref_columns: vec!["id".into()],
                match_type: Some(ForeignConstraintMatchType::Simple),
                events: Some(vec![
                    ForeignConstraintEvent::Update(ForeignConstraintAction::Cascade),
                    ForeignConstraintEvent::Delete(ForeignConstraintAction::NoAction),
                ]),
            });

        // Create a database with the base table already defined.
        let mut existing_database = Package::new();
        existing_database.tables.push(base_table());
        let publish_profile = PublishProfile::default();

        let mut changeset = Vec::new();
        // First, check that source table changes do nothing
        let _ = (&source_table).generate(&mut changeset, &existing_database, &publish_profile, &log);
        assert_that!(changeset).is_empty();

        // Now we check with a linked table constraint
        let result = LinkedTableConstraint { table: &source_table, constraint: &source_table.constraints.first().unwrap()}
                        .generate(&mut changeset, &existing_database, &publish_profile, &log);
        assert_that!(result).is_ok();

        // We should have a single instruction to create a new constraint
        assert_that!(changeset).has_length(1);
        match changeset[0] {
            ChangeInstruction::AddConstraint(ref table, ref constraint) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                match *constraint {
                    TableConstraint::Foreign { name, columns, ref_table, ref_columns, match_type, events } => {
                        assert_that!(*name).is_equal_to("fk_my_contacts_my_companies".to_owned());
                        assert_that!(*columns).has_length(1);
                        assert_that!(columns.iter()).contains("company_id".to_owned());
                        assert_that!(ref_table.to_string()).is_equal_to("my.companies".to_owned());
                        assert_that!(*ref_columns).has_length(1);
                        assert_that!(ref_columns.iter()).contains("id".to_owned());
                        assert_that!(*match_type).is_some().is_equal_to(ForeignConstraintMatchType::Simple);
                        assert_that!(*events).is_some().has_length(2); // We test this further below
                    }
                    unxpected => panic!("Unexpected constraint: {:?}", unxpected),
                }
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(changeset[0].to_sql(&log))
            .is_equal_to("ALTER TABLE my.contacts\n\
                          ADD CONSTRAINT fk_my_contacts_my_companies FOREIGN KEY (company_id) \
                          REFERENCES my.companies (id) MATCH SIMPLE ON UPDATE CASCADE ON DELETE NO ACTION".to_owned());
    }

    #[test]
    fn it_can_remove_an_existing_foreign_key() {
        let log = empty_logger();
        let source_table = base_table();

        // Create a database with the base table already defined.
        let mut existing_database = Package::new();
        let mut existing_table = base_table();
        existing_table.constraints.push(TableConstraint::Foreign {
                name: "fk_my_contacts_my_companies".to_owned(),
                columns: vec!["company_id".into()],
                ref_table: ObjectName { schema: Some("my".into()), name: "companies".into() },
                ref_columns: vec!["id".into()],
                match_type: Some(ForeignConstraintMatchType::Simple),
                events: Some(vec![
                    ForeignConstraintEvent::Update(ForeignConstraintAction::Cascade),
                    ForeignConstraintEvent::Delete(ForeignConstraintAction::NoAction),
                ]),
            });
        existing_database.tables.push(existing_table);
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.drop_foreign_key_constraints = Toggle::Allow;

        let mut changeset = Vec::new();
        // This changeset gets generated at the table level
        let result = (&source_table).generate(&mut changeset, &existing_database, &publish_profile, &log);
        assert_that!(result).is_ok();

        // We should have a single instruction to remove a constraint
        assert_that!(changeset).has_length(1);
        match changeset[0] {
            ChangeInstruction::RemoveConstraint(ref table, ref name) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                assert_that!(*name).is_equal_to("fk_my_contacts_my_companies".to_owned());
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(changeset[0].to_sql(&log))
            .is_equal_to("ALTER TABLE my.contacts\nDROP CONSTRAINT fk_my_contacts_my_companies".to_owned());
    }

    #[test]
    fn it_can_modify_an_existing_foreign_key() {
        let log = empty_logger();
        let mut source_table = base_table();
        source_table.constraints.push(TableConstraint::Foreign {
                name: "fk_my_contacts_my_companies".to_owned(),
                columns: vec!["company_id".into()],
                ref_table: ObjectName { schema: Some("my".into()), name: "companies".into() },
                ref_columns: vec!["id".into()],
                match_type: Some(ForeignConstraintMatchType::Simple),
                events: Some(vec![
                    ForeignConstraintEvent::Update(ForeignConstraintAction::NoAction),
                    ForeignConstraintEvent::Delete(ForeignConstraintAction::NoAction),
                ]),
            });

        // Create a database with the base table already defined.
        let mut existing_database = Package::new();
        let mut existing_table = base_table();
        existing_table.constraints.push(TableConstraint::Foreign {
                name: "fk_my_contacts_my_companies".to_owned(),
                columns: vec!["company_id".into()],
                ref_table: ObjectName { schema: Some("my".into()), name: "companies".into() },
                ref_columns: vec!["id".into()],
                match_type: Some(ForeignConstraintMatchType::Simple),
                events: Some(vec![
                    ForeignConstraintEvent::Update(ForeignConstraintAction::Cascade),
                    ForeignConstraintEvent::Delete(ForeignConstraintAction::NoAction),
                ]),
            });
        existing_database.tables.push(existing_table);
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.drop_foreign_key_constraints = Toggle::Allow;

        let mut changeset = Vec::new();
        // First, check that source table changes do nothing
        let _ = (&source_table).generate(&mut changeset, &existing_database, &publish_profile, &log);
        assert_that!(changeset).is_empty();

        // Now we check with a linked table constraint
        let result = LinkedTableConstraint { table: &source_table, constraint: &source_table.constraints.first().unwrap()}
                        .generate(&mut changeset, &existing_database, &publish_profile, &log);
        assert_that!(result).is_ok();

        // Primary keys cannot be altered, so we drop/create
        assert_that!(changeset).has_length(2);
        match changeset[0] {
            ChangeInstruction::RemoveConstraint(ref table, ref name) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                assert_that!(*name).is_equal_to("fk_my_contacts_my_companies".to_owned());
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }
        match changeset[1] {
            ChangeInstruction::AddConstraint(ref table, ref constraint) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                match *constraint {
                    TableConstraint::Foreign { name, columns, ref_table, ref_columns, match_type, events } => {
                        assert_that!(*name).is_equal_to("fk_my_contacts_my_companies".to_owned());
                        assert_that!(*columns).has_length(1);
                        assert_that!(columns.iter()).contains("company_id".to_owned());
                        assert_that!(ref_table.to_string()).is_equal_to("my.companies".to_owned());
                        assert_that!(*ref_columns).has_length(1);
                        assert_that!(ref_columns.iter()).contains("id".to_owned());
                        assert_that!(*match_type).is_some().is_equal_to(ForeignConstraintMatchType::Simple);
                        assert_that!(*events).is_some().has_length(2); // We test this further below
                    }
                    unxpected => panic!("Unexpected constraint: {:?}", unxpected),
                }
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(changeset[0].to_sql(&log))
            .is_equal_to("ALTER TABLE my.contacts\nDROP CONSTRAINT fk_my_contacts_my_companies".to_owned());
        assert_that!(changeset[1].to_sql(&log))
            .is_equal_to("ALTER TABLE my.contacts\n\
                          ADD CONSTRAINT fk_my_contacts_my_companies FOREIGN KEY (company_id) \
                          REFERENCES my.companies (id) MATCH SIMPLE ON UPDATE NO ACTION ON DELETE NO ACTION".to_owned());
    }
}
