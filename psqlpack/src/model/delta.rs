use std::collections::HashSet;
use std::fmt;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use serde_json;
use slog::Logger;

use crate::connection::Connection;
use crate::errors::PsqlpackErrorKind::*;
use crate::errors::{PsqlpackResult, PsqlpackResultExt};
use crate::model::delta::DbObject::Script;
use crate::model::{Capabilities, Dependency, Node, Package, PublishProfile, Toggle};
use crate::sql::ast::*;
use crate::Semver;

enum DbObject<'a> {
    Column(&'a TableDefinition, &'a ColumnDefinition),
    Constraint(&'a TableDefinition, &'a TableConstraint),
    ExtensionRequest(&'a Dependency), // 2
    Function(&'a FunctionDefinition), // 6 (ordered)
    Index(&'a IndexDefinition),       // 7
    Schema(&'a SchemaDefinition),     // 3
    Script(&'a ScriptDefinition),     // 1, 8
    Table(&'a TableDefinition),       // 5 (ordered)
    Type(&'a TypeDefinition),         // 4
}

impl<'a> fmt::Display for DbObject<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DbObject::Column(table, column) => write!(f, "Table: {}, Column: {}", table.name, column.name),
            DbObject::Constraint(table, constraint) => {
                write!(f, "Table: {}, Constraint: {}", table.name, constraint.name())
            }
            DbObject::ExtensionRequest(extension) => write!(f, "ExtensionRequest: {}", extension.name),
            DbObject::Function(function) => write!(f, "Function: {}", function.name),
            DbObject::Index(index) => write!(f, "Index: {}", index.name),
            DbObject::Schema(schema) => write!(f, "Schema: {}", schema.name),
            DbObject::Script(script) => write!(f, "Script: {}", script.name),
            DbObject::Table(table) => write!(f, "Table: {}", table.name),
            DbObject::Type(tipe) => write!(f, "Type: {}", tipe.name),
        }
    }
}

trait Diffable<'a, T> {
    fn generate(
        &self,
        change_set: &mut Vec<ChangeInstruction<'a>>,
        target: &T,
        target_capabilities: &Capabilities,
        publish_profile: &PublishProfile,
        log: &Logger,
    ) -> PsqlpackResult<()>;
}

impl<'a> Diffable<'a, Package> for DbObject<'a> {
    fn generate(
        &self,
        change_set: &mut Vec<ChangeInstruction<'a>>,
        target: &Package,
        target_capabilities: &Capabilities,
        publish_profile: &PublishProfile,
        log: &Logger,
    ) -> PsqlpackResult<()> {
        match *self {
            DbObject::Column(table, column) => LinkedColumn {
                table: &table,
                column: &column,
            }
            .generate(change_set, target, target_capabilities, publish_profile, log),
            DbObject::Constraint(table, constraint) => LinkedTableConstraint {
                table: &table,
                constraint: &constraint,
            }
            .generate(change_set, target, target_capabilities, publish_profile, log),
            DbObject::ExtensionRequest(dependency) => ExtensionRequest {
                name: &dependency.name,
                version: &dependency.version,
            }
            .generate(change_set, target, target_capabilities, publish_profile, log),
            DbObject::Function(function) => {
                function.generate(change_set, target, target_capabilities, publish_profile, log)
            }
            DbObject::Index(index) => index.generate(change_set, target, target_capabilities, publish_profile, log),
            DbObject::Schema(schema) => schema.generate(change_set, target, target_capabilities, publish_profile, log),
            DbObject::Script(script) => script.generate(change_set, target, target_capabilities, publish_profile, log),
            DbObject::Table(table) => table.generate(change_set, target, target_capabilities, publish_profile, log),
            DbObject::Type(ty) => ty.generate(change_set, target, target_capabilities, publish_profile, log),
        }
    }
}

struct ExtensionRequest<'a> {
    name: &'a String,
    version: &'a Option<Semver>,
}

impl<'a> Diffable<'a, Package> for ExtensionRequest<'a> {
    fn generate(
        &self,
        change_set: &mut Vec<ChangeInstruction<'a>>,
        _target: &Package,
        target_capabilities: &Capabilities,
        profile: &PublishProfile,
        _log: &Logger,
    ) -> PsqlpackResult<()> {
        let available = target_capabilities.available_extensions(self.name, None);

        // First of all, check to see what is installed
        let installed = available.iter().filter(|e| e.installed).count();

        // Nothing is installed
        if installed == 0 {
            // See if something is available to install first.
            let available_version = if let Some(ref version) = self.version {
                available.iter().any(|e| e.version.eq(version))
            } else {
                !available.is_empty()
            };
            if available_version {
                change_set.push(ChangeInstruction::CreateExtension(self.name.to_string(), *self.version));
            } else {
                if let Some(ref version) = self.version {
                    bail!(PublishError(format!(
                        "ExtensionRequest {} version {} not available to install",
                        self.name, version
                    )))
                }
                bail!(PublishError(format!(
                    "ExtensionRequest {} not available to install",
                    self.name
                )))
            }
        } else {
            // Something is installed - verify if we need to upgrade.
            if let Some(ref version) = self.version {
                // A SPECIFIC version is specified so check to see if it is available
                let version_available = available.iter().filter(|e| e.version.eq(version)).nth(0);
                if let Some(v) = version_available {
                    // It's available so if it is not installed then upgrade
                    if !v.installed {
                        match profile.generation_options.upgrade_extensions {
                            Toggle::Allow => {
                                change_set.push(ChangeInstruction::UpgradeExtension(
                                    self.name.to_string(),
                                    *self.version,
                                ));
                            }
                            Toggle::Error => {
                                bail!(PublishUnsafeOperationError(format!(
                                    "ExtensionRequest {} version {} is available to upgrade",
                                    v.name, v.version,
                                )));
                            }
                            Toggle::Ignore => {}
                        }
                    }
                } else {
                    // It's not installed and not available... error!
                    bail!(PublishError(format!(
                        "Expecting extension {} version {} to be installed",
                        self.name, version
                    )));
                }
            } else {
                // No version specified - are we on the latest?
                if !available[0].installed {
                    match profile.generation_options.upgrade_extensions {
                        Toggle::Allow => {
                            change_set.push(ChangeInstruction::UpgradeExtension(
                                self.name.to_string(),
                                *self.version,
                            ));
                        }
                        Toggle::Error => {
                            bail!(PublishUnsafeOperationError(format!(
                                "ExtensionRequest {} version {} is available to upgrade",
                                available[0].name, available[0].version,
                            )));
                        }
                        Toggle::Ignore => {}
                    }
                }
            }
        }
        Ok(())
    }
}

impl<'a> Diffable<'a, Package> for &'a FunctionDefinition {
    fn generate(
        &self,
        change_set: &mut Vec<ChangeInstruction<'a>>,
        _target: &Package,
        _target_capabilities: &Capabilities,
        _publish_profile: &PublishProfile,
        _log: &Logger,
    ) -> PsqlpackResult<()> {
        // Since we don't really need to worry about this in PG we just
        // add it as is and rely on CREATE OR REPLACE. In the future, it'd
        // be good to check the hash or something to only do this when required
        change_set.push(ChangeInstruction::ModifyFunction(self));
        Ok(())
    }
}

impl<'a> Diffable<'a, Package> for &'a SchemaDefinition {
    fn generate(
        &self,
        change_set: &mut Vec<ChangeInstruction<'a>>,
        target: &Package,
        _target_capabilities: &Capabilities,
        _publish_profile: &PublishProfile,
        _log: &Logger,
    ) -> PsqlpackResult<()> {
        // Only add schema's, we do not drop them at this point
        let schema_exists = target.schemas.iter().any(|s| s.name == self.name);
        if !schema_exists {
            change_set.push(ChangeInstruction::AddSchema(self));
        }
        Ok(())
    }
}

impl<'a> Diffable<'a, Package> for &'a ScriptDefinition {
    fn generate(
        &self,
        change_set: &mut Vec<ChangeInstruction<'a>>,
        _target: &Package,
        _target_capabilities: &Capabilities,
        _publish_profile: &PublishProfile,
        _log: &Logger,
    ) -> PsqlpackResult<()> {
        change_set.push(ChangeInstruction::RunScript(self));
        Ok(())
    }
}

impl<'a> Diffable<'a, Package> for &'a TableDefinition {
    fn generate(
        &self,
        change_set: &mut Vec<ChangeInstruction<'a>>,
        target: &Package,
        _target_capabilities: &Capabilities,
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
                        Toggle::Allow => change_set.push(ChangeInstruction::DropColumn(self, tgt.name.to_owned())),
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
                                ))),
                            }
                        }
                        TableConstraint::Foreign { .. } => {
                            match publish_profile.generation_options.drop_foreign_key_constraints {
                                Toggle::Allow => true,
                                Toggle::Ignore => false,
                                Toggle::Error => bail!(PublishUnsafeOperationError(format!(
                                    "Unable to drop constraint as dropping FKs is currently disabled: {}",
                                    tgt.name()
                                ))),
                            }
                        }
                    };
                    if remove_ok {
                        change_set.push(ChangeInstruction::DropConstraint(self, tgt.name().to_owned()));
                    }
                }
            }
        } else {
            change_set.push(ChangeInstruction::AddTable(self));
        }
        Ok(())
    }
}

struct LinkedColumn<'a> {
    table: &'a TableDefinition,
    column: &'a ColumnDefinition,
}

impl<'a> Diffable<'a, Package> for LinkedColumn<'a> {
    fn generate(
        &self,
        change_set: &mut Vec<ChangeInstruction<'a>>,
        target: &Package,
        _target_capabilities: &Capabilities,
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
                    change_set.push(ChangeInstruction::ModifyColumnType(self.table, &self.column));
                }

                // Check column constraints
                let src_set: HashSet<_> = self.column.constraints.iter().cloned().collect();
                let target_set: HashSet<_> = target_column.constraints.iter().cloned().collect();

                // target_set - src_set (e.g. adding new constraints)
                for x in target_set.difference(&src_set) {
                    match *x {
                        ColumnConstraint::Null => {
                            // This is a strange one - first check if the source specifies not null.
                            // If it doesn't then it's likely implicitly implied to be null.
                            // Also, we only check not null as if null is specified then we've got nothing to change!
                            if self.column.constraints.iter().any(|c| ColumnConstraint::NotNull.eq(c)) {
                                change_set.push(ChangeInstruction::ModifyColumnNull(self.table, &self.column));
                            }
                        }
                        ColumnConstraint::NotNull => {
                            change_set.push(ChangeInstruction::ModifyColumnNull(self.table, &self.column))
                        }
                        ColumnConstraint::Default(_) => {
                            change_set.push(ChangeInstruction::ModifyColumnDefault(self.table, &self.column))
                        }
                        ColumnConstraint::Unique => change_set.push(ChangeInstruction::ModifyColumnUniqueConstraint(
                            self.table,
                            &self.column,
                        )),
                        ColumnConstraint::PrimaryKey => change_set.push(
                            ChangeInstruction::ModifyColumnPrimaryKeyConstraint(self.table, &self.column),
                        ),
                    }
                }

            // TODO: src_sec - target_set (e.g. what column constraints have been removed)
            } else {
                // Doesn't exist, add it
                change_set.push(ChangeInstruction::AddColumn(self.table, &self.column));
            }
        }
        Ok(())
    }
}

struct LinkedTableConstraint<'a> {
    table: &'a TableDefinition,
    constraint: &'a TableConstraint,
}

impl<'a> Diffable<'a, Package> for LinkedTableConstraint<'a> {
    fn generate(
        &self,
        change_set: &mut Vec<ChangeInstruction<'a>>,
        target: &Package,
        _target_capabilities: &Capabilities,
        publish_profile: &PublishProfile,
        _log: &Logger,
    ) -> PsqlpackResult<()> {
        fn vec_different<T: PartialEq>(src: &[T], tgt: &[T]) -> bool {
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
            let target_constraint = target_table
                .constraints
                .iter()
                .find(|tgt| tgt.name().eq(self.constraint.name()));
            if let Some(target_constraint) = target_constraint {
                // Exists on target - compare to see if it's equal
                // TODO: After we have a min defined Postgres version we may be able to use ALTER in some cases as supported
                let has_changed = match *self.constraint {
                    TableConstraint::Primary {
                        ref columns,
                        ref parameters,
                        ..
                    } => {
                        let src_columns = columns;
                        let src_parameters = parameters;
                        match target_constraint {
                            TableConstraint::Primary {
                                ref columns,
                                ref parameters,
                                ..
                            } => {
                                vec_different(src_columns, columns)
                                    || optional_vec_different(src_parameters, parameters)
                            }
                            TableConstraint::Foreign { .. } => true,
                        }
                    }
                    TableConstraint::Foreign {
                        ref columns,
                        ref ref_table,
                        ref ref_columns,
                        ref match_type,
                        ref events,
                        ..
                    } => {
                        let src_columns = columns;
                        let src_ref_table = ref_table;
                        let src_ref_columns = ref_columns;
                        let src_match_type = match_type;
                        let src_events = events;
                        match target_constraint {
                            TableConstraint::Primary { .. } => true,
                            TableConstraint::Foreign {
                                ref columns,
                                ref ref_table,
                                ref ref_columns,
                                ref match_type,
                                ref events,
                                ..
                            } => {
                                let match_type_different = if src_match_type.is_some() && match_type.is_some() {
                                    src_match_type.as_ref().unwrap().ne(match_type.as_ref().unwrap())
                                } else {
                                    src_match_type.is_none() ^ match_type.is_none()
                                };
                                match_type_different
                                    || vec_different(src_columns, columns)
                                    || src_ref_table.ne(ref_table)
                                    || vec_different(src_ref_columns, ref_columns)
                                    || optional_vec_different(src_events, events)
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
                                ))),
                            }
                        }
                        TableConstraint::Foreign { .. } => {
                            match publish_profile.generation_options.drop_foreign_key_constraints {
                                Toggle::Allow => true,
                                Toggle::Ignore => false,
                                Toggle::Error => bail!(PublishUnsafeOperationError(format!(
                                    "Unable to modify constraint as dropping FKs is currently disabled: {}",
                                    self.constraint.name()
                                ))),
                            }
                        }
                    };
                    if remove_ok {
                        change_set.push(ChangeInstruction::DropConstraint(
                            self.table,
                            self.constraint.name().to_owned(),
                        ));
                        change_set.push(ChangeInstruction::AddConstraint(self.table, self.constraint));
                    }
                }
            } else {
                // Doesn't exist, add it
                change_set.push(ChangeInstruction::AddConstraint(self.table, &self.constraint));
            }
        } else {
            change_set.push(ChangeInstruction::AddConstraint(self.table, &self.constraint));
        }
        Ok(())
    }
}

impl<'a> Diffable<'a, Package> for &'a IndexDefinition {
    fn generate(
        &self,
        change_set: &mut Vec<ChangeInstruction<'a>>,
        target: &Package,
        _target_capabilities: &Capabilities,
        publish_profile: &PublishProfile,
        _log: &Logger,
    ) -> PsqlpackResult<()> {
        // Indexes are unique across schema (implied by table)
        let index = target.indexes.iter().find(|idx| idx.is_same_index(self));
        let concurrently = publish_profile.generation_options.force_concurrent_indexes;
        if let Some(index) = index {
            // We should be able to just use an eq for this since column ordering is significant
            if index.ne(self) {
                change_set.push(ChangeInstruction::DropIndex(self.fully_qualified_name(), concurrently));
                change_set.push(ChangeInstruction::AddIndex(self, concurrently));
            }
        } else {
            change_set.push(ChangeInstruction::AddIndex(self, concurrently));
        }
        Ok(())
    }
}

impl<'a> Diffable<'a, Package> for &'a TypeDefinition {
    fn generate(
        &self,
        change_set: &mut Vec<ChangeInstruction<'a>>,
        target: &Package,
        _target_capabilities: &Capabilities,
        publish_profile: &PublishProfile,
        log: &Logger,
    ) -> PsqlpackResult<()> {
        let ty = target.types.iter().find(|t| t.name == self.name);
        if let Some(ty) = ty {
            self.generate(change_set, ty, _target_capabilities, publish_profile, log)
        } else {
            change_set.push(ChangeInstruction::AddType(self));
            Ok(())
        }
    }
}

impl<'a> Diffable<'a, TypeDefinition> for &'a TypeDefinition {
    fn generate(
        &self,
        change_set: &mut Vec<ChangeInstruction<'a>>,
        target: &TypeDefinition,
        _target_capabilities: &Capabilities,
        publish_profile: &PublishProfile,
        _log: &Logger,
    ) -> PsqlpackResult<()> {
        if self.name.ne(&target.name) {
            bail!(PublishInvalidOperationError(format!(
                "Types not diffable: {} != {}",
                self.name, target.name
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
                            .map(|v| TypeModificationAction::RemoveEnumValue { value: v.to_owned() })
                            .collect::<Vec<_>>();
                        if !to_delete.is_empty() {
                            match publish_profile.generation_options.drop_enum_values {
                                Toggle::Allow => {
                                    change_set
                                        .extend(to_delete.drain(..).map(|d| ChangeInstruction::ModifyType(self, d)));
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
                        for (index, value) in source_values.iter().enumerate() {
                            if !working.contains(&value) {
                                if index == 0 {
                                    change_set.push(ChangeInstruction::ModifyType(
                                        self,
                                        TypeModificationAction::AddEnumValueBefore {
                                            value: value.to_owned(),
                                            before: working[0].to_owned(),
                                        },
                                    ));
                                    working.insert(0, value);
                                } else {
                                    change_set.push(ChangeInstruction::ModifyType(
                                        self,
                                        TypeModificationAction::AddEnumValueAfter {
                                            value: value.to_owned(),
                                            after: working[index - 1].to_owned(),
                                        },
                                    ));
                                    working.insert(index, value);
                                }
                            }
                        }
                    }
                    ref unknown_target_kind => panic!("Unknown target kind: {}", unknown_target_kind), // TODO
                }
            }
            ref unknown_source_kind => panic!("Unknown source kind: {}", unknown_source_kind), // TODO
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
        target_database_name: &str,
        target_capabilities: &Capabilities,
        publish_profile: &PublishProfile,
    ) -> PsqlpackResult<Delta<'package>> {
        let log = log.new(o!("delta" => "generate"));

        // Start the change_set
        let mut change_set = Vec::new();

        // If we always recreate then add a drop and set to false
        let mut target = target;
        if target.is_some() && publish_profile.generation_options.always_recreate_database {
            change_set.push(ChangeInstruction::KillConnections(target_database_name.to_owned()));
            change_set.push(ChangeInstruction::DropDatabase(target_database_name.to_owned()));
            target = None;
        }

        // For an empty database use an empty package, but also push a CREATE DB instruction
        let target_package = match target {
            Some(target_package) => target_package,
            None => {
                change_set.push(ChangeInstruction::CreateDatabase(target_database_name.to_owned()));
                Package::new()
            }
        };

        // Set the connection instruction
        change_set.push(ChangeInstruction::UseDatabase(target_database_name.to_owned()));

        // Create the build order - including all document types outside the topological sort.
        let mut build_order = Vec::new();

        // Pre deployment scripts
        let mut scripts = package
            .scripts
            .iter()
            .filter(|s| s.kind == ScriptKind::PreDeployment)
            .collect::<Vec<_>>();
        scripts.sort_by_key(|s| s.order);
        for script in scripts {
            build_order.push(DbObject::Script(script));
        }

        // Extensions
        for extension in &package.extensions {
            build_order.push(DbObject::ExtensionRequest(extension));
        }

        // Schemas
        for schema in &package.schemas {
            build_order.push(DbObject::Schema(schema));
        }

        // Types
        for t in &package.types {
            build_order.push(DbObject::Type(t));
        }

        // Drop indexes first
        for index in &target_package.indexes {
            if !package.indexes.iter().any(|idx| idx.is_same_index(&index)) {
                match publish_profile.generation_options.drop_indexes {
                    Toggle::Allow => change_set.push(ChangeInstruction::DropIndex(
                        index.fully_qualified_name(),
                        publish_profile.generation_options.force_concurrent_indexes,
                    )),
                    Toggle::Error => bail!(PublishUnsafeOperationError(format!(
                        "Attempted to drop index {} however dropping indexes is currently disabled",
                        index.name
                    ))),
                    _ => {}
                }
            }
        }

        // Drop functions next - first figure out if there are any to drop
        for function in &target_package.functions {
            if !package.functions.iter().any(|t| t.name.eq(&function.name)) {
                match publish_profile.generation_options.drop_functions {
                    Toggle::Allow => change_set.push(ChangeInstruction::DropFunction(function.name.to_string())),
                    Toggle::Error => bail!(PublishUnsafeOperationError(format!(
                        "Attempted to drop function {} however dropping functions is currently disabled",
                        function.name
                    ))),
                    _ => {}
                }
            }
        }

        // Drop tables next - first figure out if there are any to drop
        for table in &target_package.tables {
            if !package.tables.iter().any(|t| t.name.eq(&table.name)) {
                match publish_profile.generation_options.drop_tables {
                    Toggle::Allow => change_set.push(ChangeInstruction::DropTable(table.name.to_string())),
                    Toggle::Error => bail!(PublishUnsafeOperationError(format!(
                        "Attempted to drop table {} however dropping tables is currently disabled",
                        table.name
                    ))),
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

        // Indexes come into play now (all objects and constraints are created)
        for index in &package.indexes {
            build_order.push(DbObject::Index(index));
        }

        // Add in post deployment scripts
        let mut scripts = package
            .scripts
            .iter()
            .filter(|s| s.kind == ScriptKind::PostDeployment)
            .collect::<Vec<_>>();
        scripts.sort_by_key(|s| s.order);
        for script in scripts {
            build_order.push(DbObject::Script(script));
        }

        // Go through each item in order and figure out what to do with it
        for item in &build_order {
            item.generate(
                &mut change_set,
                &target_package,
                &target_capabilities,
                &publish_profile,
                &log,
            )?;
        }

        Ok(Delta(change_set))
    }

    pub fn apply(&self, log: &Logger, connection: &Connection) -> PsqlpackResult<()> {
        let log = log.new(o!("delta" => "apply"));

        let change_set = &self.0;

        // These instructions turn into SQL statements that get executed
        let mut conn = connection.connect_host()?;

        for change in change_set.iter() {
            if let ChangeInstruction::UseDatabase(..) = *change {
                conn = connection.connect_database()?;
                continue;
            }

            // Execute SQL directly
            trace!(log, "Executing: {}", change);
            let sql = change.to_sql(&log);
            conn.batch_execute(&sql).chain_err(|| DatabaseExecuteError(sql))?;
        }

        Ok(())
    }

    pub fn write_report(&self, destination: &Path) -> PsqlpackResult<()> {
        let change_set = &self.0;

        File::create(destination)
            .chain_err(|| GenerationError("Failed to generate report".to_owned()))
            .and_then(|writer| {
                serde_json::to_writer_pretty(writer, &change_set)
                    .chain_err(|| GenerationError("Failed to generate report".to_owned()))
            })?;

        Ok(())
    }

    pub fn write_sql(&self, log: &Logger, destination: &Path) -> PsqlpackResult<()> {
        let change_set = &self.0;

        // These instructions turn into a single SQL file
        let mut out = match File::create(destination) {
            Ok(o) => o,
            Err(e) => bail!(GenerationError(format!("Failed to generate SQL file: {}", e))),
        };

        for change in change_set.iter() {
            let sql = change.to_sql(log);
            match out.write_all(sql.as_bytes()) {
                Ok(_) => {
                    // New line
                    match out.write(&[59u8, 10u8, 10u8]) {
                        Ok(_) => {}
                        Err(e) => bail!(GenerationError(format!("Failed to generate SQL file: {}", e))),
                    }
                }
                Err(e) => bail!(GenerationError(format!("Failed to generate SQL file: {}", e))),
            }
        }

        Ok(())
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub enum ChangeInstruction<'input> {
    // Databases
    KillConnections(String),
    DropDatabase(String),
    CreateDatabase(String),
    UseDatabase(String),

    // Extensions - no delete for now
    CreateExtension(String, Option<Semver>),
    UpgradeExtension(String, Option<Semver>),

    // Schema
    AddSchema(&'input SchemaDefinition),
    //DropSchema(String),

    // Scripts
    RunScript(&'input ScriptDefinition),

    // Types
    AddType(&'input TypeDefinition),
    ModifyType(&'input TypeDefinition, TypeModificationAction),
    DropType(String),

    // Tables
    AddTable(&'input TableDefinition),
    DropTable(String),

    // Columns
    AddColumn(&'input TableDefinition, &'input ColumnDefinition),
    ModifyColumnType(&'input TableDefinition, &'input ColumnDefinition),
    ModifyColumnNull(&'input TableDefinition, &'input ColumnDefinition),
    ModifyColumnDefault(&'input TableDefinition, &'input ColumnDefinition),
    ModifyColumnUniqueConstraint(&'input TableDefinition, &'input ColumnDefinition),
    ModifyColumnPrimaryKeyConstraint(&'input TableDefinition, &'input ColumnDefinition),
    DropColumn(&'input TableDefinition, String),

    // Constraints
    AddConstraint(&'input TableDefinition, &'input TableConstraint),
    DropConstraint(&'input TableDefinition, String),

    // Index
    AddIndex(&'input IndexDefinition, bool),
    DropIndex(String, bool),

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
            KillConnections(ref database) => write!(f, "Kill connections: {}", database),
            DropDatabase(ref database) => write!(f, "Drop database: {}", database),
            CreateDatabase(ref database) => write!(f, "Create database: {}", database),
            UseDatabase(ref database) => write!(f, "Use database: {}", database),

            // Extensions
            CreateExtension(ref name, ref version) => {
                if let Some(ref version) = version {
                    write!(f, "Create extension: {} version {}", name, version)
                } else {
                    write!(f, "Create extension: {}", name)
                }
            }
            UpgradeExtension(ref name, ref version) => {
                if let Some(ref version) = version {
                    write!(f, "Upgrade extension: {} version {}", name, version)
                } else {
                    write!(f, "Upgrade extension: {}", name)
                }
            }

            // Schema
            AddSchema(schema) => write!(f, "Add schema: {}", schema.name),
            //DropSchema(String),

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
            DropType(ref type_name) => write!(f, "Drop type: {}", type_name),

            // Tables
            AddTable(table) => write!(f, "Add table: {}", table.name),
            DropTable(ref table_name) => write!(f, "Drop table: {}", table_name),

            // Columns
            AddColumn(table, column) => write!(f, "Add column: {} to table: {}", column.name, table.name),
            ModifyColumnType(table, column) => {
                write!(f, "Modify type for column: {} on table: {}", column.name, table.name)
            }
            ModifyColumnNull(table, column) => {
                write!(f, "Modify null for column: {} on table: {}", column.name, table.name)
            }
            ModifyColumnDefault(table, column) => {
                write!(f, "Modify default for column: {} on table: {}", column.name, table.name)
            }
            ModifyColumnUniqueConstraint(table, column) => write!(
                f,
                "Modify unique constraint for column: {} on table: {}",
                column.name, table.name
            ),
            ModifyColumnPrimaryKeyConstraint(table, column) => write!(
                f,
                "Modify primary key constraint for column: {} on table: {}",
                column.name, table.name
            ),
            DropColumn(table, ref column_name) => write!(f, "Drop column: {} on table: {}", column_name, table.name),

            // Constraints
            AddConstraint(table, constraint) => {
                write!(f, "Add constraint: {} to table: {}", constraint.name(), table.name)
            }
            DropConstraint(table, ref name) => write!(f, "Drop constraint: {} to table: {}", name, table.name),

            // Indexes
            AddIndex(index, concurrently) => write!(
                f,
                "Add index{}: {}",
                if concurrently { " concurrently" } else { "" },
                index.fully_qualified_name()
            ),
            DropIndex(ref index_name, concurrently) => write!(
                f,
                "Drop index{}: {}",
                if concurrently { " concurrently" } else { "" },
                index_name
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
            ChangeInstruction::KillConnections(ref db) => {
                let mut drop = String::new();
                drop.push_str("SELECT pg_terminate_backend(pg_stat_activity.pid) ");
                drop.push_str("FROM pg_stat_activity ");
                drop.push_str(&format!("WHERE pg_stat_activity.datname = '{}';", db));
                drop
            }
            ChangeInstruction::CreateDatabase(ref db) => format!("CREATE DATABASE {}", db),
            ChangeInstruction::DropDatabase(ref db) => format!("DROP DATABASE {}", db),
            ChangeInstruction::UseDatabase(ref db) => format!("-- Using database `{}`", db),

            // ExtensionRequest level
            ChangeInstruction::CreateExtension(ref name, ref version) => {
                if let Some(ref version) = version {
                    format!(
                        "CREATE EXTENSION IF NOT EXISTS \"{}\" WITH VERSION \"{}\"",
                        name, version
                    )
                } else {
                    format!("CREATE EXTENSION IF NOT EXISTS \"{}\"", name)
                }
            }
            ChangeInstruction::UpgradeExtension(ref name, ref version) => {
                if let Some(ref version) = version {
                    format!("ALTER EXTENSION \"{}\" UPDATE TO \"{}\"", name, version)
                } else {
                    format!("ALTER EXTENSION \"{}\" UPDATE", name)
                }
            }

            // Schema level
            ChangeInstruction::AddSchema(schema) => {
                if schema.name == "public" {
                    format!("CREATE SCHEMA IF NOT EXISTS {}", schema.name)
                } else {
                    format!("CREATE SCHEMA {}", schema.name)
                }
            }

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
                    ref unknown => panic!("Unknown kind: {}", unknown), // TODO
                }
                def
            }
            ChangeInstruction::ModifyType(ty, ref action) => match *action {
                TypeModificationAction::AddEnumValueBefore { ref value, ref before } => {
                    format!("ALTER TYPE {} ADD VALUE '{}' BEFORE '{}'", ty.name, value, before)
                }
                TypeModificationAction::AddEnumValueAfter { ref value, ref after } => {
                    format!("ALTER TYPE {} ADD VALUE '{}' AFTER '{}'", ty.name, value, after)
                }
                TypeModificationAction::RemoveEnumValue { ref value } => format!(
                    "DELETE FROM pg_enum \
                     WHERE enumlabel='{}' AND \
                     enumtypid=(SELECT oid FROM pg_type WHERE {})",
                    value,
                    if let Some(ref schema) = ty.name.schema {
                        format!("nspname='{}' AND typname='{}'", schema, ty.name.name)
                    } else {
                        format!("typname='{}'", ty.name.name)
                    },
                ),
            },
            ChangeInstruction::DropType(ref type_name) => format!("DROP TYPE IF EXISTS {}", type_name),

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

                    func.push_str(&arg.to_string());
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
                    FunctionReturnType::SetOf(ref sql_type) => {
                        func.push_str(&format!("SETOF {}", sql_type)[..]);
                    }
                    FunctionReturnType::SqlType(ref sql_type) => {
                        func.push_str(&format!("{} ", sql_type)[..]);
                    }
                }
                func.push_str("AS $$");
                func.push_str(&function.body[..]);
                func.push_str("$$\n");
                func.push_str("LANGUAGE ");
                func.push_str(&function.language.to_string());
                func
            }
            ChangeInstruction::DropFunction(ref function_name) => format!("DROP FUNCTION IF EXISTS {}", function_name),

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
                            ColumnConstraint::Default(ref any_type) => {
                                instr.push_str(&format!(" DEFAULT {}", any_type))
                            }
                            ColumnConstraint::NotNull => instr.push_str(" NOT NULL"),
                            ColumnConstraint::Null => instr.push_str(" NULL"),
                            ColumnConstraint::Unique => instr.push_str(" UNIQUE"),
                            ColumnConstraint::PrimaryKey => instr.push_str(" PRIMARY KEY"),
                        }
                    }
                }
                // Table constraints are added later
                instr.push_str("\n)");
                instr
            }
            ChangeInstruction::DropTable(ref table_name) => format!("DROP TABLE IF EXISTS {}", table_name),

            // Column level
            ChangeInstruction::AddColumn(table, column) => {
                let mut instr = String::new();
                instr.push_str(&format!(
                    "ALTER TABLE {} ADD COLUMN {} {}",
                    table.name, column.name, column.sql_type
                ));
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
            ChangeInstruction::ModifyColumnType(table, column) => format!(
                "ALTER TABLE {} ALTER COLUMN {} TYPE {}",
                table.name, column.name, column.sql_type
            ),
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
                error!(
                    log,
                    "Expected to modify column null constraint for {}.{}", table.name, column.name
                );
                "".to_owned()
            }
            ChangeInstruction::ModifyColumnDefault(table, column) => {
                for constraint in column.constraints.iter() {
                    if let ColumnConstraint::Default(ref any_type) = *constraint {
                        return format!(
                            ";\nALTER TABLE {} ALTER COLUMN {} SET DEFAULT {}",
                            table.name, column.name, any_type
                        );
                    }
                }
                error!(
                    log,
                    "Expected to modify column default constraint for {}.{}", table.name, column.name
                );
                "".to_owned()
            }
            ChangeInstruction::ModifyColumnUniqueConstraint(table, column) => {
                for constraint in column.constraints.iter() {
                    if let ColumnConstraint::Unique = *constraint {
                        // TODO: These have to be table level constraints. Ignore??
                        warn!(
                            log,
                            "Ignoring UNIQUE column constraint for {}.{}", table.name, column.name
                        );
                    }
                }
                "".to_owned()
            }
            ChangeInstruction::ModifyColumnPrimaryKeyConstraint(table, column) => {
                for constraint in column.constraints.iter() {
                    if let ColumnConstraint::PrimaryKey = *constraint {
                        // TODO: These have to be table level constraints. Ignore??
                        warn!(
                            log,
                            "Ignoring PRIMARY KEY column constraint for {}.{}", table.name, column.name
                        );
                    }
                }
                "".to_owned()
            }
            ChangeInstruction::DropColumn(table, ref column_name) => {
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
                        instr.push_str(&format!("CONSTRAINT {} PRIMARY KEY ({})", name, columns.join(", ")));

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

            ChangeInstruction::DropConstraint(table, ref name) => {
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

            // Indexes
            ChangeInstruction::AddIndex(index, concurrently) => {
                let mut instr = String::new();
                instr.push_str("CREATE ");
                if index.unique {
                    instr.push_str("UNIQUE ");
                }
                instr.push_str("INDEX ");
                if concurrently {
                    instr.push_str("CONCURRENTLY ");
                }
                instr.push_str(&format!("{} ON {}", index.name, index.table));
                if let Some(ref method) = index.index_type {
                    instr.push_str(" USING ");
                    instr.push_str(match method {
                        IndexType::BTree => "btree",
                        IndexType::Gin => "gin",
                        IndexType::Gist => "gist",
                        IndexType::Hash => "hash",
                    });
                }
                instr.push_str(" (");
                for (position, col) in index.columns.iter().enumerate() {
                    if position > 0 {
                        instr.push_str(", ");
                    }
                    instr.push_str(&col.name);
                    if let Some(ref order) = col.order {
                        instr.push_str(match order {
                            IndexOrder::Ascending => " ASC",
                            IndexOrder::Descending => " DESC",
                        });
                    }
                    if let Some(ref pos) = col.null_position {
                        instr.push_str(match pos {
                            IndexPosition::First => " NULLS FIRST",
                            IndexPosition::Last => " NULLS LAST",
                        });
                    }
                }
                instr.push_str(")");
                if let Some(ref storage_parameters) = index.storage_parameters {
                    instr.push_str(" WITH (");
                    for (position, value) in storage_parameters.iter().enumerate() {
                        if position > 0 {
                            instr.push_str(", ");
                        }
                        match *value {
                            IndexParameter::FillFactor(i) => instr.push_str(&format!("FILLFACTOR={}", i)),
                        }
                    }
                    instr.push_str(")");
                }
                instr
            }
            ChangeInstruction::DropIndex(ref index_name, concurrently) => {
                let mut instr = String::new();
                instr.push_str("DROP INDEX ");
                if concurrently {
                    instr.push_str("CONCURRENTLY ");
                }
                instr.push_str(&format!("IF EXISTS {}", index_name));
                instr
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::errors::PsqlpackError;
    use crate::model::*;
    use crate::sql::ast;
    use crate::Semver;

    use slog::{Discard, Drain, Logger};
    use spectral::prelude::*;

    fn empty_logger() -> Logger {
        Logger::root(Discard.fuse(), o!())
    }

    fn base_type() -> ast::TypeDefinition {
        ast::TypeDefinition {
            name: ast::ObjectName {
                schema: Some("public".to_string()),
                name: "colors".to_string(),
            },
            kind: ast::TypeDefinitionKind::Enum(vec!["red".into(), "green".into(), "blue".into()]),
        }
    }

    #[test]
    fn it_can_add_enum_type() {
        let log = empty_logger();
        let source_type = base_type();

        // Create an empty package (i.e. so it needs to create the type)
        let existing_database = Package::new();
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let publish_profile = PublishProfile::default();

        let mut change_set = Vec::new();
        let result = (&source_type).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have a single instruction to add
        assert_that!(change_set).has_length(1);
        match change_set[0] {
            ChangeInstruction::AddType(ref ty) => {
                assert_that!(ty.name).is_equal_to(ast::ObjectName {
                    schema: Some("public".to_string()),
                    name: "colors".to_string(),
                });
                let values = match ty.kind {
                    TypeDefinitionKind::Enum(ref values) => values.clone(),
                    ref unknown => panic!("Unknown kind: {}", unknown), // TODO
                };
                assert_that!(values).has_length(3);
                assert_that!(values[0]).is_equal_to("red".to_owned());
                assert_that!(values[1]).is_equal_to("green".to_owned());
                assert_that!(values[2]).is_equal_to("blue".to_owned());
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(change_set[0].to_sql(&log))
            .is_equal_to("CREATE TYPE public.colors AS ENUM (\n  'red',\n  'green',\n  'blue'\n)".to_owned());
    }

    #[test]
    fn it_ignores_enum_type_if_not_modified() {
        let log = empty_logger();
        let source_type = base_type();

        // Create a package with the type already defined (same as base type)
        let mut existing_database = Package::new();
        existing_database.types.push(base_type());
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let publish_profile = PublishProfile::default();

        let mut change_set = Vec::new();
        let result = (&source_type).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have a single instruction to add
        assert_that!(change_set).is_empty();
    }

    #[test]
    fn it_can_modify_enum_type_by_adding_a_value_to_the_end() {
        let log = empty_logger();
        let source_type = ast::TypeDefinition {
            name: ast::ObjectName {
                schema: Some("public".to_string()),
                name: "colors".to_string(),
            },
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
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let publish_profile = PublishProfile::default();

        let mut change_set = Vec::new();
        let result = (&source_type).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have a single instruction to modify the enum with an additional value
        assert_that!(change_set).has_length(1);
        match change_set[0] {
            ChangeInstruction::ModifyType(ty, ref action) => {
                assert_that!(ty.name).is_equal_to(ast::ObjectName {
                    schema: Some("public".to_string()),
                    name: "colors".to_string(),
                });

                // Also, match the action
                match *action {
                    TypeModificationAction::AddEnumValueAfter { ref value, ref after } => {
                        assert_that!(*value).is_equal_to("black".to_owned());
                        assert_that!(*after).is_equal_to("blue".to_owned());
                    }
                    ref unexpected => panic!("Unexpected enum modification action: {:?}", unexpected),
                }
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(change_set[0].to_sql(&log))
            .is_equal_to("ALTER TYPE public.colors ADD VALUE 'black' AFTER 'blue'".to_owned());
    }

    #[test]
    fn it_can_modify_enum_type_by_adding_a_value_to_the_start() {
        let log = empty_logger();
        let source_type = ast::TypeDefinition {
            name: ast::ObjectName {
                schema: Some("public".to_string()),
                name: "colors".to_string(),
            },
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
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let publish_profile = PublishProfile::default();

        let mut change_set = Vec::new();
        let result = (&source_type).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have a single instruction to modify the enum with an additional value
        assert_that!(change_set).has_length(1);
        match change_set[0] {
            ChangeInstruction::ModifyType(ty, ref action) => {
                assert_that!(ty.name).is_equal_to(ast::ObjectName {
                    schema: Some("public".to_string()),
                    name: "colors".to_string(),
                });

                // Also, match the action
                match *action {
                    TypeModificationAction::AddEnumValueBefore { ref value, ref before } => {
                        assert_that!(*value).is_equal_to("black".to_owned());
                        assert_that!(*before).is_equal_to("red".to_owned());
                    }
                    ref unexpected => panic!("Unexpected enum modification action: {:?}", unexpected),
                }
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(change_set[0].to_sql(&log))
            .is_equal_to("ALTER TYPE public.colors ADD VALUE 'black' BEFORE 'red'".to_owned());
    }

    #[test]
    fn it_can_modify_enum_type_by_adding_a_value_to_the_middle() {
        let log = empty_logger();
        let source_type = ast::TypeDefinition {
            name: ast::ObjectName {
                schema: Some("public".to_string()),
                name: "colors".to_string(),
            },
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
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let publish_profile = PublishProfile::default();

        let mut change_set = Vec::new();
        let result = (&source_type).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have a single instruction to modify the enum with an additional value
        assert_that!(change_set).has_length(1);
        match change_set[0] {
            ChangeInstruction::ModifyType(ty, ref action) => {
                assert_that!(ty.name).is_equal_to(ast::ObjectName {
                    schema: Some("public".to_string()),
                    name: "colors".to_string(),
                });

                // Also, match the action
                match *action {
                    TypeModificationAction::AddEnumValueAfter { ref value, ref after } => {
                        assert_that!(*value).is_equal_to("black".to_owned());
                        assert_that!(*after).is_equal_to("green".to_owned());
                    }
                    ref unexpected => panic!("Unexpected enum modification action: {:?}", unexpected),
                }
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(change_set[0].to_sql(&log))
            .is_equal_to("ALTER TYPE public.colors ADD VALUE 'black' AFTER 'green'".to_owned());
    }

    #[test]
    fn it_can_modify_enum_type_by_modifying_values_and_unsafe_declared() {
        let log = empty_logger();
        let source_type = ast::TypeDefinition {
            name: ast::ObjectName {
                schema: Some("public".to_string()),
                name: "colors".to_string(),
            },
            kind: ast::TypeDefinitionKind::Enum(vec!["black".to_owned(), "green".to_owned(), "blue".to_owned()]),
        };

        // Create a package with the type already defined
        let mut existing_database = Package::new();
        existing_database.types.push(base_type());
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.drop_enum_values = Toggle::Allow;

        let mut change_set = Vec::new();
        let result = (&source_type).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have a single instruction to modify the enum with an additional value
        assert_that!(change_set).has_length(2);

        // Removals first
        match change_set[0] {
            ChangeInstruction::ModifyType(ty, ref action) => {
                assert_that!(ty.name).is_equal_to(ast::ObjectName {
                    schema: Some("public".to_string()),
                    name: "colors".to_string(),
                });

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
        assert_that!(change_set[0].to_sql(&log)).is_equal_to(
            "DELETE FROM pg_enum \
             WHERE enumlabel='red' AND \
             enumtypid=(SELECT oid FROM pg_type WHERE nspname='public' AND typname='colors')"
                .to_owned(),
        );

        // Additions second
        match change_set[1] {
            ChangeInstruction::ModifyType(ty, ref action) => {
                assert_that!(ty.name).is_equal_to(ast::ObjectName {
                    schema: Some("public".to_string()),
                    name: "colors".to_string(),
                });

                // Also, match the action
                match *action {
                    TypeModificationAction::AddEnumValueBefore { ref value, ref before } => {
                        assert_that!(*value).is_equal_to("black".to_owned());
                        assert_that!(*before).is_equal_to("green".to_owned());
                    }
                    ref unexpected => panic!("Unexpected enum modification action: {:?}", unexpected),
                }
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }
        // Check the SQL generation
        assert_that!(change_set[1].to_sql(&log))
            .is_equal_to("ALTER TYPE public.colors ADD VALUE 'black' BEFORE 'green'".to_owned());
    }

    #[test]
    fn it_rejects_modifying_enum_type_when_modifying_values_by_default() {
        let log = empty_logger();
        let source_type = ast::TypeDefinition {
            name: ast::ObjectName {
                schema: Some("public".to_string()),
                name: "colors".to_string(),
            },
            kind: ast::TypeDefinitionKind::Enum(vec!["black".to_owned(), "green".to_owned(), "blue".to_owned()]),
        };

        // Create a package with the type already defined
        let mut existing_database = Package::new();
        existing_database.types.push(base_type());
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let publish_profile = PublishProfile::default();

        let mut change_set = Vec::new();
        let result = (&source_type).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_err();
        match result.err().unwrap() {
            PsqlpackError(PublishUnsafeOperationError(_), _) => {}
            unexpected => panic!("Expected unsafe operation error however saw {:?}", unexpected),
        };
    }

    #[test]
    fn it_can_modify_enum_type_by_removing_values_and_unsafe_declared() {
        let log = empty_logger();
        let source_type = ast::TypeDefinition {
            name: ast::ObjectName {
                schema: Some("public".to_string()),
                name: "colors".to_string(),
            },
            kind: ast::TypeDefinitionKind::Enum(vec!["green".to_owned(), "blue".to_owned()]),
        };

        // Create a package with the type already defined
        let mut existing_database = Package::new();
        existing_database.types.push(base_type());
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.drop_enum_values = Toggle::Allow;

        let mut change_set = Vec::new();
        let result = (&source_type).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have a single instruction to modify the enum with an additional value
        assert_that!(change_set).has_length(1);

        // Removals first
        match change_set[0] {
            ChangeInstruction::ModifyType(ty, ref action) => {
                assert_that!(ty.name).is_equal_to(ast::ObjectName {
                    schema: Some("public".to_string()),
                    name: "colors".to_string(),
                });

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
        assert_that!(change_set[0].to_sql(&log)).is_equal_to(
            "DELETE FROM pg_enum \
             WHERE enumlabel='red' AND \
             enumtypid=(SELECT oid FROM pg_type WHERE nspname='public' AND typname='colors')"
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
                    sql_type: SqlType::Simple(SimpleSqlType::Serial, None),
                    constraints: vec![ColumnConstraint::NotNull, ColumnConstraint::PrimaryKey],
                },
                ColumnDefinition {
                    name: "company_id".to_owned(),
                    sql_type: SqlType::Simple(SimpleSqlType::BigInteger, None),
                    constraints: vec![ColumnConstraint::NotNull],
                },
                ColumnDefinition {
                    name: "first_name".to_owned(),
                    sql_type: SqlType::Simple(SimpleSqlType::VariableLengthString(100), None),
                    constraints: vec![ColumnConstraint::NotNull],
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
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let publish_profile = PublishProfile::default();

        let mut change_set = Vec::new();
        let result = (&source_table).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have a single instruction to create a new table
        assert_that!(change_set).has_length(1);
        match change_set[0] {
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
        assert_that!(change_set[0].to_sql(&log)).is_equal_to(
            "CREATE TABLE my.contacts (\n\
             \tid serial NOT NULL PRIMARY KEY,\n\
             \tcompany_id bigint NOT NULL,\n\
             \tfirst_name varchar(100) NOT NULL\n\
             )"
            .to_owned(),
        );
    }

    #[test]
    fn it_can_add_column_to_existing_table() {
        let log = empty_logger();
        let mut source_table = base_table();
        source_table.columns.push(ColumnDefinition {
            name: "last_name".to_owned(),
            sql_type: SqlType::Simple(SimpleSqlType::VariableLengthString(100), None),
            constraints: vec![ColumnConstraint::NotNull],
        });

        // Create a database with the base table already defined.
        let mut existing_database = Package::new();
        existing_database.tables.push(base_table());
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let publish_profile = PublishProfile::default();

        let mut change_set = Vec::new();
        // First, check that source table changes do nothing
        let _ = (&source_table).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(change_set).is_empty();

        // Now we check with a linked column
        let result = LinkedColumn {
            table: &source_table,
            column: &source_table.columns.last().unwrap(),
        }
        .generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have a single instruction to create a new table
        assert_that!(change_set).has_length(1);
        match change_set[0] {
            ChangeInstruction::AddColumn(ref table, ref column) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                assert_that!(column.name).is_equal_to("last_name".to_owned());
                assert_that!(column.sql_type)
                    .is_equal_to(SqlType::Simple(SimpleSqlType::VariableLengthString(100), None));
                assert_that!(column.constraints).has_length(1);
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(change_set[0].to_sql(&log))
            .is_equal_to("ALTER TABLE my.contacts ADD COLUMN last_name varchar(100) NOT NULL".to_owned());
    }

    #[test]
    fn it_can_widen_column_on_existing_table() {
        let log = empty_logger();
        let mut source_table = base_table();
        source_table.columns.push(ColumnDefinition {
            name: "last_name".to_owned(),
            sql_type: SqlType::Simple(SimpleSqlType::VariableLengthString(200), None),
            constraints: vec![ColumnConstraint::NotNull],
        });

        // Create a database with the base table already defined.
        let mut existing_database = Package::new();
        let mut existing_table = base_table();
        existing_table.columns.push(ColumnDefinition {
            name: "last_name".to_owned(),
            sql_type: SqlType::Simple(SimpleSqlType::VariableLengthString(100), None),
            constraints: vec![ColumnConstraint::NotNull],
        });

        existing_database.tables.push(existing_table);
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let publish_profile = PublishProfile::default();

        let mut change_set = Vec::new();
        // First, check that source table changes do nothing
        let _ = (&source_table).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(change_set).is_empty();

        // Now we check with a linked column
        let result = LinkedColumn {
            table: &source_table,
            column: &source_table.columns.last().unwrap(),
        }
        .generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have a single instruction to create a new table
        assert_that!(change_set).has_length(1);
        match change_set[0] {
            ChangeInstruction::ModifyColumnType(ref table, ref column) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                assert_that!(column.name).is_equal_to("last_name".to_owned());
                assert_that!(column.sql_type)
                    .is_equal_to(SqlType::Simple(SimpleSqlType::VariableLengthString(200), None));
                assert_that!(column.constraints).has_length(1);
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(change_set[0].to_sql(&log))
            .is_equal_to("ALTER TABLE my.contacts ALTER COLUMN last_name TYPE varchar(200)".to_owned());
    }

    #[test]
    fn it_can_drop_column_on_existing_table() {
        let log = empty_logger();
        let source_table = base_table();

        // Create a database with the base table already defined.
        let mut existing_database = Package::new();
        let mut existing_table = base_table();
        existing_table.columns.push(ColumnDefinition {
            name: "last_name".to_owned(),
            sql_type: SqlType::Simple(SimpleSqlType::VariableLengthString(100), None),
            constraints: vec![ColumnConstraint::NotNull],
        });

        existing_database.tables.push(existing_table);
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.drop_columns = Toggle::Allow;

        let mut change_set = Vec::new();
        let result = (&source_table).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have a single instruction to create a new table
        assert_that!(change_set).has_length(1);
        match change_set[0] {
            ChangeInstruction::DropColumn(ref table, ref column_name) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                assert_that!(*column_name).is_equal_to("last_name".to_owned());
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(change_set[0].to_sql(&log))
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
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let publish_profile = PublishProfile::default();

        let mut change_set = Vec::new();
        // First, check that source table changes do nothing
        let _ = (&source_table).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(change_set).is_empty();

        // Now we check with a linked table constraint
        let result = LinkedTableConstraint {
            table: &source_table,
            constraint: &source_table.constraints.first().unwrap(),
        }
        .generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have a single instruction to add a constraint
        assert_that!(change_set).has_length(1);
        match change_set[0] {
            ChangeInstruction::AddConstraint(ref table, ref constraint) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                match *constraint {
                    TableConstraint::Primary {
                        name,
                        columns,
                        parameters,
                    } => {
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
        assert_that!(change_set[0].to_sql(&log)).is_equal_to(
            "ALTER TABLE my.contacts\nADD CONSTRAINT pk_my_contacts_id PRIMARY KEY (id) WITH (FILLFACTOR=80)"
                .to_owned(),
        );
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
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.drop_primary_key_constraints = Toggle::Allow;

        let mut change_set = Vec::new();
        // This change_set gets generated at the table level
        let result = (&source_table).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have a single instruction to remove the constraint
        assert_that!(change_set).has_length(1);
        match change_set[0] {
            ChangeInstruction::DropConstraint(ref table, ref name) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                assert_that!(*name).is_equal_to("pk_my_contacts_id".to_owned());
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(change_set[0].to_sql(&log))
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
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.drop_primary_key_constraints = Toggle::Allow;

        let mut change_set = Vec::new();
        // First, check that source table changes do nothing
        let _ = (&source_table).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(change_set).is_empty();

        // Now we check with a linked table constraint
        let result = LinkedTableConstraint {
            table: &source_table,
            constraint: &source_table.constraints.first().unwrap(),
        }
        .generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // Primary keys cannot be altered, so we drop/create
        assert_that!(change_set).has_length(2);
        match change_set[0] {
            ChangeInstruction::DropConstraint(ref table, ref name) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                assert_that!(*name).is_equal_to("pk_my_contacts_id".to_owned());
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }
        match change_set[1] {
            ChangeInstruction::AddConstraint(ref table, ref constraint) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                match *constraint {
                    TableConstraint::Primary {
                        name,
                        columns,
                        parameters,
                    } => {
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
        assert_that!(change_set[0].to_sql(&log))
            .is_equal_to("ALTER TABLE my.contacts\nDROP CONSTRAINT pk_my_contacts_id".to_owned());
        assert_that!(change_set[1].to_sql(&log)).is_equal_to(
            "ALTER TABLE my.contacts\nADD CONSTRAINT pk_my_contacts_id PRIMARY KEY (id) WITH (FILLFACTOR=80)"
                .to_owned(),
        );
    }

    #[test]
    fn it_can_add_a_new_foreign_key() {
        let log = empty_logger();
        let mut source_table = base_table();
        source_table.constraints.push(TableConstraint::Foreign {
            name: "fk_my_contacts_my_companies".to_owned(),
            columns: vec!["company_id".into()],
            ref_table: ObjectName {
                schema: Some("my".into()),
                name: "companies".into(),
            },
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
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let publish_profile = PublishProfile::default();

        let mut change_set = Vec::new();
        // First, check that source table changes do nothing
        let _ = (&source_table).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(change_set).is_empty();

        // Now we check with a linked table constraint
        let result = LinkedTableConstraint {
            table: &source_table,
            constraint: &source_table.constraints.first().unwrap(),
        }
        .generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have a single instruction to create a new constraint
        assert_that!(change_set).has_length(1);
        match change_set[0] {
            ChangeInstruction::AddConstraint(ref table, ref constraint) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                match *constraint {
                    TableConstraint::Foreign {
                        name,
                        columns,
                        ref_table,
                        ref_columns,
                        match_type,
                        events,
                    } => {
                        assert_that!(*name).is_equal_to("fk_my_contacts_my_companies".to_owned());
                        assert_that!(*columns).has_length(1);
                        assert_that!(columns.iter()).contains("company_id".to_owned());
                        assert_that!(ref_table.to_string()).is_equal_to("my.companies".to_owned());
                        assert_that!(*ref_columns).has_length(1);
                        assert_that!(ref_columns.iter()).contains("id".to_owned());
                        assert_that!(*match_type)
                            .is_some()
                            .is_equal_to(ForeignConstraintMatchType::Simple);
                        assert_that!(*events).is_some().has_length(2); // We test this further below
                    }
                    unxpected => panic!("Unexpected constraint: {:?}", unxpected),
                }
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(change_set[0].to_sql(&log)).is_equal_to(
            "ALTER TABLE my.contacts\n\
             ADD CONSTRAINT fk_my_contacts_my_companies FOREIGN KEY (company_id) \
             REFERENCES my.companies (id) MATCH SIMPLE ON UPDATE CASCADE ON DELETE NO ACTION"
                .to_owned(),
        );
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
            ref_table: ObjectName {
                schema: Some("my".into()),
                name: "companies".into(),
            },
            ref_columns: vec!["id".into()],
            match_type: Some(ForeignConstraintMatchType::Simple),
            events: Some(vec![
                ForeignConstraintEvent::Update(ForeignConstraintAction::Cascade),
                ForeignConstraintEvent::Delete(ForeignConstraintAction::NoAction),
            ]),
        });
        existing_database.tables.push(existing_table);
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.drop_foreign_key_constraints = Toggle::Allow;

        let mut change_set = Vec::new();
        // This change_set gets generated at the table level
        let result = (&source_table).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have a single instruction to remove a constraint
        assert_that!(change_set).has_length(1);
        match change_set[0] {
            ChangeInstruction::DropConstraint(ref table, ref name) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                assert_that!(*name).is_equal_to("fk_my_contacts_my_companies".to_owned());
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(change_set[0].to_sql(&log))
            .is_equal_to("ALTER TABLE my.contacts\nDROP CONSTRAINT fk_my_contacts_my_companies".to_owned());
    }

    #[test]
    fn it_can_modify_an_existing_foreign_key() {
        let log = empty_logger();
        let mut source_table = base_table();
        source_table.constraints.push(TableConstraint::Foreign {
            name: "fk_my_contacts_my_companies".to_owned(),
            columns: vec!["company_id".into()],
            ref_table: ObjectName {
                schema: Some("my".into()),
                name: "companies".into(),
            },
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
            ref_table: ObjectName {
                schema: Some("my".into()),
                name: "companies".into(),
            },
            ref_columns: vec!["id".into()],
            match_type: Some(ForeignConstraintMatchType::Simple),
            events: Some(vec![
                ForeignConstraintEvent::Update(ForeignConstraintAction::Cascade),
                ForeignConstraintEvent::Delete(ForeignConstraintAction::NoAction),
            ]),
        });
        existing_database.tables.push(existing_table);
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.drop_foreign_key_constraints = Toggle::Allow;

        let mut change_set = Vec::new();
        // First, check that source table changes do nothing
        let _ = (&source_table).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(change_set).is_empty();

        // Now we check with a linked table constraint
        let result = LinkedTableConstraint {
            table: &source_table,
            constraint: &source_table.constraints.first().unwrap(),
        }
        .generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // Primary keys cannot be altered, so we drop/create
        assert_that!(change_set).has_length(2);
        match change_set[0] {
            ChangeInstruction::DropConstraint(ref table, ref name) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                assert_that!(*name).is_equal_to("fk_my_contacts_my_companies".to_owned());
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }
        match change_set[1] {
            ChangeInstruction::AddConstraint(ref table, ref constraint) => {
                assert_that!(table.name.to_string()).is_equal_to("my.contacts".to_owned());
                match *constraint {
                    TableConstraint::Foreign {
                        name,
                        columns,
                        ref_table,
                        ref_columns,
                        match_type,
                        events,
                    } => {
                        assert_that!(*name).is_equal_to("fk_my_contacts_my_companies".to_owned());
                        assert_that!(*columns).has_length(1);
                        assert_that!(columns.iter()).contains("company_id".to_owned());
                        assert_that!(ref_table.to_string()).is_equal_to("my.companies".to_owned());
                        assert_that!(*ref_columns).has_length(1);
                        assert_that!(ref_columns.iter()).contains("id".to_owned());
                        assert_that!(*match_type)
                            .is_some()
                            .is_equal_to(ForeignConstraintMatchType::Simple);
                        assert_that!(*events).is_some().has_length(2); // We test this further below
                    }
                    unxpected => panic!("Unexpected constraint: {:?}", unxpected),
                }
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(change_set[0].to_sql(&log))
            .is_equal_to("ALTER TABLE my.contacts\nDROP CONSTRAINT fk_my_contacts_my_companies".to_owned());
        assert_that!(change_set[1].to_sql(&log)).is_equal_to(
            "ALTER TABLE my.contacts\n\
             ADD CONSTRAINT fk_my_contacts_my_companies FOREIGN KEY (company_id) \
             REFERENCES my.companies (id) MATCH SIMPLE ON UPDATE NO ACTION ON DELETE NO ACTION"
                .to_owned(),
        );
    }

    #[test]
    fn it_can_add_a_new_index() {
        let log = empty_logger();
        let source_index = IndexDefinition {
            name: "idx_contacts_first_name".to_owned(),
            table: ObjectName {
                schema: Some("public".to_owned()),
                name: "contacts".to_owned(),
            },
            columns: vec![IndexColumn {
                name: "first_name".to_owned(),
                order: Some(IndexOrder::Ascending),
                null_position: Some(IndexPosition::Last),
            }],
            unique: true,
            index_type: Some(IndexType::BTree),
            storage_parameters: None,
        };

        // Create a database with no indexes defined.
        let existing_database = Package::new();
        let publish_profile = PublishProfile::default();
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };

        let mut change_set = Vec::new();
        let result = (&source_index).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have a single instruction to create a new index
        assert_that!(change_set).has_length(1);
        match change_set[0] {
            ChangeInstruction::AddIndex(ref index, concurrently) => {
                assert_that!(index.name).is_equal_to("idx_contacts_first_name".to_owned());
                assert_that!(index.table.to_string()).is_equal_to("public.contacts".to_owned());
                assert_that!(concurrently).is_true();
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(change_set[0].to_sql(&log)).is_equal_to(
            "CREATE UNIQUE INDEX CONCURRENTLY idx_contacts_first_name \
             ON public.contacts USING btree (first_name ASC NULLS LAST)"
                .to_owned(),
        );
    }

    #[test]
    fn it_can_remove_an_existing_index() {
        let log = empty_logger();
        let source_package = Package::new();

        // Create a database with the index already defined.
        fn existing_db() -> Option<Package> {
            let mut existing_database = Package::new();
            existing_database.indexes.push(IndexDefinition {
                name: "idx_contacts_first_name".to_owned(),
                table: ObjectName {
                    schema: Some("public".to_owned()),
                    name: "contacts".to_owned(),
                },
                columns: vec![IndexColumn {
                    name: "first_name".to_owned(),
                    order: Some(IndexOrder::Ascending),
                    null_position: Some(IndexPosition::Last),
                }],
                unique: true,
                index_type: Some(IndexType::BTree),
                storage_parameters: None,
            });
            Some(existing_database)
        }
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.drop_indexes = Toggle::Error;

        // First of all, make sure an error is generated
        let result = Delta::generate(
            &log,
            &source_package,
            existing_db(),
            "dbname",
            &capabilities,
            &publish_profile,
        );
        assert_that!(result).is_err();
        // Now run it again - it should be ok now
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.drop_indexes = Toggle::Allow;
        let result = Delta::generate(
            &log,
            &source_package,
            existing_db(),
            "dbname",
            &capabilities,
            &publish_profile,
        );
        assert_that!(result).is_ok();
        let change_set = match result.unwrap() {
            Delta(c) => c,
        };

        // We should have a single instruction to remove an index (first will be use database)
        assert_that!(change_set).has_length(2);
        match change_set[1] {
            ChangeInstruction::DropIndex(ref index, concurrently) => {
                assert_that!(*index).is_equal_to("public.idx_contacts_first_name".to_owned());
                assert_that!(concurrently).is_true();
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(change_set[1].to_sql(&log))
            .is_equal_to("DROP INDEX CONCURRENTLY IF EXISTS public.idx_contacts_first_name".to_owned());
    }

    #[test]
    fn it_can_modify_an_existing_index() {
        let log = empty_logger();
        let source_index = IndexDefinition {
            name: "idx_contacts_name".to_owned(),
            table: ObjectName {
                schema: Some("public".to_owned()),
                name: "contacts".to_owned(),
            },
            columns: vec![
                IndexColumn {
                    name: "first_name".to_owned(),
                    order: Some(IndexOrder::Ascending),
                    null_position: Some(IndexPosition::Last),
                },
                IndexColumn {
                    name: "last_name".to_owned(),
                    order: Some(IndexOrder::Descending),
                    null_position: Some(IndexPosition::First),
                },
            ],
            unique: false,
            index_type: Some(IndexType::BTree),
            storage_parameters: None,
        };

        // Create a database with a single index defined.
        let mut existing_database = Package::new();
        existing_database.indexes.push(IndexDefinition {
            name: "idx_contacts_name".to_owned(),
            table: ObjectName {
                schema: Some("public".to_owned()),
                name: "contacts".to_owned(),
            },
            columns: vec![IndexColumn {
                name: "first_name".to_owned(),
                order: Some(IndexOrder::Ascending),
                null_position: Some(IndexPosition::Last),
            }],
            unique: true,
            index_type: Some(IndexType::BTree),
            storage_parameters: None,
        });
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let publish_profile = PublishProfile::default();

        let mut change_set = Vec::new();
        let result = (&source_index).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have two instructions to drop/create a new index
        assert_that!(change_set).has_length(2);
        match change_set[0] {
            ChangeInstruction::DropIndex(ref index_name, concurrently) => {
                assert_that!(*index_name).is_equal_to("public.idx_contacts_name".to_owned());
                assert_that!(concurrently).is_true();
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }
        match change_set[1] {
            ChangeInstruction::AddIndex(ref index, concurrently) => {
                assert_that!(index.name).is_equal_to("idx_contacts_name".to_owned());
                assert_that!(index.table.to_string()).is_equal_to("public.contacts".to_owned());
                assert_that!(concurrently).is_true();
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(change_set[0].to_sql(&log))
            .is_equal_to("DROP INDEX CONCURRENTLY IF EXISTS public.idx_contacts_name".to_owned());
        assert_that!(change_set[1].to_sql(&log)).is_equal_to(
            "CREATE INDEX CONCURRENTLY idx_contacts_name \
             ON public.contacts USING btree (first_name ASC NULLS LAST, last_name DESC NULLS FIRST)"
                .to_owned(),
        );
    }

    #[test]
    fn it_can_create_an_extension_that_exists_and_is_not_installed_with_version() {
        let log = empty_logger();
        let requested_extension = ExtensionRequest {
            name: &"postgis".to_owned(),
            version: &Some(Semver::new(2, 3, Some(7))),
        };

        // Create a database with a single extension available.
        let existing_database = Package::new();
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: vec![Extension {
                name: "postgis".to_owned(),
                version: Semver::new(2, 3, Some(7)),
                installed: false,
            }],
            database_exists: true,
        };
        let publish_profile = PublishProfile::default();

        let mut change_set = Vec::new();
        let result = (&requested_extension).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have one instruction to create an extension
        assert_that!(change_set).has_length(1);
        match change_set[0] {
            ChangeInstruction::CreateExtension(ref name, ref _version) => {
                assert_that!(name).is_equal_to(&"postgis".to_owned());
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(change_set[0].to_sql(&log))
            .is_equal_to("CREATE EXTENSION IF NOT EXISTS \"postgis\" WITH VERSION \"2.3.7\"".to_owned());
    }

    #[test]
    fn it_can_create_an_extension_that_exists_and_is_not_installed_without_version() {
        let log = empty_logger();
        let requested_extension = ExtensionRequest {
            name: &"postgis".to_owned(),
            version: &None,
        };

        // Create a database with a single extension available.
        let existing_database = Package::new();
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: vec![Extension {
                name: "postgis".to_owned(),
                version: Semver::new(2, 3, Some(7)),
                installed: false,
            }],
            database_exists: true,
        };
        let publish_profile = PublishProfile::default();

        let mut change_set = Vec::new();
        let result = (&requested_extension).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have one instruction to create an extension
        assert_that!(change_set).has_length(1);
        match change_set[0] {
            ChangeInstruction::CreateExtension(ref name, ref _version) => {
                assert_that!(name).is_equal_to(&"postgis".to_owned());
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(change_set[0].to_sql(&log)).is_equal_to("CREATE EXTENSION IF NOT EXISTS \"postgis\"".to_owned());
    }

    #[test]
    fn it_errors_when_creating_an_extension_that_exists_and_different_version() {
        let log = empty_logger();
        let requested_extension = ExtensionRequest {
            name: &"postgis".to_owned(),
            version: &Some(Semver::new(2, 4, Some(7))),
        };

        // Create a database with a single extension available.
        let existing_database = Package::new();
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: vec![Extension {
                name: "postgis".to_owned(),
                version: Semver::new(2, 3, Some(7)),
                installed: false,
            }],
            database_exists: true,
        };
        let publish_profile = PublishProfile::default();

        let mut change_set = Vec::new();
        let result = (&requested_extension).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_err();

        let err = result.err().unwrap();
        let expect = "Publish error: ExtensionRequest postgis version 2.4.7 not available to install".to_owned();
        assert_that!(format!("{}", err)).is_equal_to(&expect);
    }

    #[test]
    fn it_errors_when_creating_an_extension_that_does_not_exist() {
        let log = empty_logger();
        let requested_extension = ExtensionRequest {
            name: &"postgis".to_owned(),
            version: &Some(Semver::new(2, 3, Some(7))),
        };

        // Create a database with no extensions available.
        let existing_database = Package::new();
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: Vec::new(),
            database_exists: true,
        };
        let publish_profile = PublishProfile::default();

        let mut change_set = Vec::new();
        let result = (&requested_extension).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_err();

        let err = result.err().unwrap();
        let expect = "Publish error: ExtensionRequest postgis version 2.3.7 not available to install".to_owned();
        assert_that!(format!("{}", err)).is_equal_to(&expect);
    }

    #[test]
    fn it_can_upgrade_an_installed_extension_with_newer_version_specified_and_available() {
        let log = empty_logger();
        let requested_extension = ExtensionRequest {
            name: &"postgis".to_owned(),
            version: &Some(Semver::new(3, 8, None)),
        };

        // Create a database with two extensions available.
        let existing_database = Package::new();
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: vec![
                Extension {
                    name: "postgis".to_owned(),
                    version: Semver::new(2, 3, Some(7)),
                    installed: true,
                },
                Extension {
                    name: "postgis".to_owned(),
                    version: Semver::new(3, 8, None),
                    installed: false,
                },
            ],
            database_exists: true,
        };
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.upgrade_extensions = Toggle::Allow;

        let mut change_set = Vec::new();
        let result = (&requested_extension).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have one instruction to upgrade an extension
        assert_that!(change_set).has_length(1);
        match change_set[0] {
            ChangeInstruction::UpgradeExtension(ref name, ref _version) => {
                assert_that!(name).is_equal_to(&"postgis".to_owned());
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(change_set[0].to_sql(&log))
            .is_equal_to("ALTER EXTENSION \"postgis\" UPDATE TO \"3.8\"".to_owned());
    }

    #[test]
    fn it_does_not_modify_an_extension_that_is_already_installed() {
        let log = empty_logger();
        let requested_extension = ExtensionRequest {
            name: &"postgis".to_owned(),
            version: &None,
        };

        // Create a database with a single extension available.
        let existing_database = Package::new();
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: vec![Extension {
                name: "postgis".to_owned(),
                version: Semver::new(2, 3, Some(7)),
                installed: true,
            }],
            database_exists: true,
        };
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.upgrade_extensions = Toggle::Allow;

        let mut change_set = Vec::new();
        let result = (&requested_extension).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have no instructions
        assert_that!(change_set).is_empty();
    }

    #[test]
    fn it_can_upgrade_an_installed_extension_with_no_version_specified_and_newer_version_available_when_profile_set_to_allow(
    ) {
        let log = empty_logger();
        let requested_extension = ExtensionRequest {
            name: &"postgis".to_owned(),
            version: &None,
        };

        // Create a database with multiple extensions available.
        let existing_database = Package::new();
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: vec![
                Extension {
                    name: "postgis".to_owned(),
                    version: Semver::new(2, 3, Some(7)),
                    installed: true,
                },
                Extension {
                    name: "postgis".to_owned(),
                    version: Semver::new(3, 8, None),
                    installed: false,
                },
            ],
            database_exists: true,
        };
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.upgrade_extensions = Toggle::Allow;

        let mut change_set = Vec::new();
        let result = (&requested_extension).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have one instruction to upgrade an extension
        assert_that!(change_set).has_length(1);
        match change_set[0] {
            ChangeInstruction::UpgradeExtension(ref name, ref _version) => {
                assert_that!(name).is_equal_to(&"postgis".to_owned());
            }
            ref unexpected => panic!("Unexpected instruction type: {:?}", unexpected),
        }

        // Check the SQL generation
        assert_that!(change_set[0].to_sql(&log)).is_equal_to("ALTER EXTENSION \"postgis\" UPDATE".to_owned());
    }

    #[test]
    fn it_doesnt_upgrade_an_installed_extension_with_no_version_specified_and_newer_version_available_when_profile_set_to_ignore(
    ) {
        let log = empty_logger();
        let requested_extension = ExtensionRequest {
            name: &"postgis".to_owned(),
            version: &None,
        };

        // Create a database with multiple extensions available.
        let existing_database = Package::new();
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: vec![
                Extension {
                    name: "postgis".to_owned(),
                    version: Semver::new(2, 3, Some(7)),
                    installed: true,
                },
                Extension {
                    name: "postgis".to_owned(),
                    version: Semver::new(3, 8, None),
                    installed: false,
                },
            ],
            database_exists: true,
        };
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.upgrade_extensions = Toggle::Ignore;

        let mut change_set = Vec::new();
        let result = (&requested_extension).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_ok();

        // We should have no instructions
        assert_that!(change_set).is_empty();
    }

    #[test]
    fn it_doesnt_upgrade_an_installed_extension_with_no_version_specified_and_newer_version_available_when_profile_set_to_error(
    ) {
        let log = empty_logger();
        let requested_extension = ExtensionRequest {
            name: &"postgis".to_owned(),
            version: &None,
        };

        // Create a database with a single extension available.
        let existing_database = Package::new();
        let capabilities = Capabilities {
            server_version: Semver::new(9, 6, None),
            extensions: vec![
                Extension {
                    name: "postgis".to_owned(),
                    version: Semver::new(2, 3, Some(7)),
                    installed: true,
                },
                Extension {
                    name: "postgis".to_owned(),
                    version: Semver::new(3, 8, None),
                    installed: false,
                },
            ],
            database_exists: true,
        };
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.upgrade_extensions = Toggle::Error;

        let mut change_set = Vec::new();
        let result = (&requested_extension).generate(
            &mut change_set,
            &existing_database,
            &capabilities,
            &publish_profile,
            &log,
        );
        assert_that!(result).is_err();

        let err = result.err().unwrap();
        let expect =
            "Couldn't publish database due to an unsafe operation: ExtensionRequest postgis version 3.8 is available to upgrade"
                .to_owned();
        assert_that!(format!("{}", err)).is_equal_to(&expect);
    }
}
