//! Profiles are configurations that affect how an operation is applied.
//!
//! For instance, a `PublishProfile` might determine how unknown entities in the
//! target are handled when performing a `publish` operation.

use std::default::Default;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use serde_json;

use crate::errors::PsqlpackErrorKind::*;
use crate::errors::{PsqlpackResult, PsqlpackResultExt};
use crate::semver::Semver;

#[derive(Deserialize, Serialize)]
pub struct PublishProfile {
    pub version: Semver,
    #[serde(alias = "generationOptions")]
    pub generation_options: GenerationOptions,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub enum Toggle {
    Allow,
    Ignore,
    Error,
}

impl Toggle {
    fn allow() -> Toggle { Toggle::Allow }
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
    #[serde(alias = "alwaysRecreateDatabase")]
    pub always_recreate_database: bool,

    /// Enum values are typically unsafe to delete. If set to Allow, psqlpack will attempt to delete.
    /// Default: Error
    #[serde(alias = "dropEnumValues", default = "Toggle::error")]
    pub drop_enum_values: Toggle,
    /// Tables may have data in them which may not be intended to be deleted. If set to Allow, psqlpack will drop the table.
    /// Default: Error
    #[serde(alias = "dropTables", default = "Toggle::error")]
    pub drop_tables: Toggle,
    /// Columns may have data in them which may not be intended to be deleted. If set to Allow, psqlpack will drop the column.
    /// Default: Error
    #[serde(alias = "dropColumns", default = "Toggle::error")]
    pub drop_columns: Toggle,
    /// Primary Keys define how a table is looked up on disk. If set to Allow, psqlpack will drop the primary key.
    /// Default: Error
    #[serde(alias = "dropPrimaryKeyConstraints", default = "Toggle::error")]
    pub drop_primary_key_constraints: Toggle,
    /// Foreign Keys define a constraint to another table. If set to Allow, psqlpack will drop the foreign key.
    /// Default: Allow
    #[serde(alias = "dropForeignKeyConstraints", default = "Toggle::allow")]
    pub drop_foreign_key_constraints: Toggle,
    /// Functions may not be intended to be deleted. If set to Allow, psqlpack will drop the function.
    /// Default: Error
    #[serde(alias = "dropFunctions", default = "Toggle::error")]
    pub drop_functions: Toggle,
    /// Indexes may not be intended to be deleted. If set to Allow, psqlpack will drop the index.
    /// Default: Allow
    #[serde(alias = "dropIndexes", default = "Toggle::allow")]
    pub drop_indexes: Toggle,

    /// Extensions may not be intended to be upgraded automatically. If set to Allow, psqlpack will upgrade the extension as necessary.
    /// Default: Ignore
    #[serde(alias = "upgradeExtensions", default = "Toggle::ignore")]
    pub upgrade_extensions: Toggle,

    /// Forces index changes to be made concurrently to avoid locking on table writes.
    /// Default: true
    #[serde(alias = "forceConcurrentIndexes", default = "Bool::t")]
    pub force_concurrent_indexes: bool,
}

impl Default for PublishProfile {
    fn default() -> Self {
        PublishProfile {
            version: "1.0".into(),
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
            .and_then(|file| Self::from_reader(file))
    }

    fn from_reader<R>(reader: R) -> PsqlpackResult<PublishProfile>
        where R: Read {

        let mut buffered_reader = BufReader::new(reader);
        let mut contents = String::new();
        if buffered_reader.read_to_string(&mut contents)
            .chain_err(|| PublishProfileParseError("Failed to read contents".into()))? == 0 {
            bail!(PublishProfileParseError("Data was empty".into()))
        }

        let trimmed = contents.trim_start();
        if trimmed.starts_with('{') {
            serde_json::from_str(&contents).chain_err(|| PublishProfileParseError("Failed to read JSON".into()))
        } else {
            toml::from_str(&contents).chain_err(|| PublishProfileParseError("Failed to read TOML".into()))
        }
    }

}

#[cfg(test)]
mod tests {
    use crate::{PublishProfile, Semver};
    use crate::model::Toggle;
    use spectral::prelude::*;

    #[test]
    fn it_can_add_read_a_publish_profile_in_json_format() {
        const DATA: &str = r#"
            {
              "version": "1.0",
              "generationOptions": {
                "alwaysRecreateDatabase": false,
                "dropEnumValues": "Error",
                "dropFunctions": "Error",
                "dropTables": "Error",
                "dropColumns": "Error",
                "dropPrimaryKeyConstraints": "Error",
                "dropForeignKeyConstraints": "Allow",
                "dropIndexes": "Ignore",
                "forceConcurrentIndexes": false
              }
            }
        "#;
        let publish_profile = PublishProfile::from_reader(DATA.as_bytes());
        let publish_profile = publish_profile.unwrap();
        assert_that!(publish_profile.version).is_equal_to(Semver::new(1, 0, None));
        let options = publish_profile.generation_options;
        assert_that!(options.always_recreate_database).is_false();
        assert_that!(options.drop_enum_values).is_equal_to(Toggle::Error);
        assert_that!(options.drop_functions).is_equal_to(Toggle::Error);
        assert_that!(options.drop_tables).is_equal_to(Toggle::Error);
        assert_that!(options.drop_columns).is_equal_to(Toggle::Error);
        assert_that!(options.drop_primary_key_constraints).is_equal_to(Toggle::Error);
        assert_that!(options.drop_foreign_key_constraints).is_equal_to(Toggle::Allow);
        assert_that!(options.drop_indexes).is_equal_to(Toggle::Ignore);
        assert_that!(options.force_concurrent_indexes).is_false();
    }

    #[test]
    fn it_can_add_read_a_publish_profile_in_toml_format() {
        const DATA: &str = r#"
            version = "1.0"

            [generationOptions]
            always_recreate_database = false
            drop_enum_values = "Error"
            drop_functions = "Error"
            drop_tables = "Error"
            drop_columns = "Error"
            drop_primary_key_constraints = "Error"
            drop_foreign_key_constraints = "Allow"
            drop_indexes = "Ignore"
            force_concurrent_indexes = false
        "#;
        let publish_profile = PublishProfile::from_reader(DATA.as_bytes());
        let publish_profile = publish_profile.unwrap();
        assert_that!(publish_profile.version).is_equal_to(Semver::new(1, 0, None));
        let options = publish_profile.generation_options;
        assert_that!(options.always_recreate_database).is_false();
        assert_that!(options.drop_enum_values).is_equal_to(Toggle::Error);
        assert_that!(options.drop_functions).is_equal_to(Toggle::Error);
        assert_that!(options.drop_tables).is_equal_to(Toggle::Error);
        assert_that!(options.drop_columns).is_equal_to(Toggle::Error);
        assert_that!(options.drop_primary_key_constraints).is_equal_to(Toggle::Error);
        assert_that!(options.drop_foreign_key_constraints).is_equal_to(Toggle::Allow);
        assert_that!(options.drop_indexes).is_equal_to(Toggle::Ignore);
        assert_that!(options.force_concurrent_indexes).is_false();
    }
}