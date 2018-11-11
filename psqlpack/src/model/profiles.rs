//! Profiles are configurations that affect how an operation is applied.
//!
//! For instance, a `PublishProfile` might determine how unknown entities in the
//! target are handled when performing a `publish` operation.

use std::default::Default;
use std::path::Path;
use std::fs::File;

use serde_json;

use errors::{PsqlpackResult, PsqlpackResultExt};
use errors::PsqlpackErrorKind::*;

#[derive(Deserialize, Serialize)]
pub struct PublishProfile {
    pub version: String,
    #[serde(rename = "generationOptions")] pub generation_options: GenerationOptions,
}

#[derive(Deserialize, Serialize)]
pub enum Toggle {
    Allow,
    Ignore,
    Error,
}

impl Toggle {
    fn allow() -> Toggle {
        Toggle::Allow
    }

    fn ignore() -> Toggle {
        Toggle::Ignore
    }

    fn error() -> Toggle {
        Toggle::Error
    }
}

struct Bool;
impl Bool {
    fn t() -> bool {
        true
    }
}

#[derive(Deserialize, Serialize)]
pub struct GenerationOptions {
    /// If set to true, the database will always be recereated
    #[serde(rename = "alwaysRecreateDatabase")] pub always_recreate_database: bool,

    /// Enum values are typically unsafe to delete. If set to Allow, psqlpack will attempt to delete.
    /// Default: Error
    #[serde(rename = "dropEnumValues", default = "Toggle::error")]
    pub drop_enum_values: Toggle,
    /// Tables may have data in them which may not be intended to be deleted. If set to Allow, psqlpack will drop the table.
    /// Default: Error
    #[serde(rename = "dropTables", default = "Toggle::error")]
    pub drop_tables: Toggle,
    /// Columns may have data in them which may not be intended to be deleted. If set to Allow, psqlpack will drop the column.
    /// Default: Error
    #[serde(rename = "dropColumns", default = "Toggle::error")]
    pub drop_columns: Toggle,
    /// Primary Keys define how a table is looked up on disk. If set to Allow, psqlpack will drop the primary key.
    /// Default: Error
    #[serde(rename = "dropPrimaryKeyConstraints", default = "Toggle::error")]
    pub drop_primary_key_constraints: Toggle,
    /// Foreign Keys define a constraint to another table. If set to Allow, psqlpack will drop the foreign key.
    /// Default: Allow
    #[serde(rename = "dropForeignKeyConstraints", default = "Toggle::allow")]
    pub drop_foreign_key_constraints: Toggle,
    /// Functions may not be intended to be deleted. If set to Allow, psqlpack will drop the function.
    /// Default: Error
    #[serde(rename = "dropFunctions", default = "Toggle::error")]
    pub drop_functions: Toggle,
    /// Indexes may not be intended to be deleted. If set to Allow, psqlpack will drop the index.
    /// Default: Allow
    #[serde(rename = "dropIndexes", default = "Toggle::allow")]
    pub drop_indexes: Toggle,

    /// Extensions may not be intended to be upgraded automatically. If set to Allow, psqlpack will upgrade the extension as necessary.
    /// Default: Ignore
    #[serde(rename = "upgradeExtensions", default = "Toggle::ignore")]
    pub upgrade_extensions: Toggle,

    /// Forces index changes to be made concurrently to avoid locking on table writes.
    /// Default: true
    #[serde(rename = "forceConcurrentIndexes", default = "Bool::t")]
    pub force_concurrent_indexes: bool,
}

impl Default for PublishProfile {
    fn default() -> Self {
        PublishProfile {
            version: "1.0".to_owned(),
            generation_options: GenerationOptions {
                always_recreate_database: false,

                drop_enum_values: Toggle::Error,
                drop_tables: Toggle::Error,
                drop_columns: Toggle::Error,
                drop_primary_key_constraints: Toggle::Error,
                drop_foreign_key_constraints: Toggle::Allow,
                drop_functions: Toggle::Error,
                drop_indexes: Toggle::Allow,

                upgrade_extensions: Toggle::Ignore,

                force_concurrent_indexes: true,
            },
        }
    }
}

impl PublishProfile {
    pub fn from_path(profile_path: &Path) -> PsqlpackResult<PublishProfile> {
        File::open(profile_path)
            .chain_err(|| PublishProfileReadError(profile_path.to_path_buf()))
            .and_then(|file| {
                serde_json::from_reader(file).chain_err(|| PublishProfileParseError(profile_path.to_path_buf()))
            })
    }
}
