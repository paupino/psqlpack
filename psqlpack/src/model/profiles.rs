//! Profiles are configurations that affect how an operation is applied.
//!
//! For instance, a `PublishProfile` might determine how unknown entities in the
//! target are handled when performing a `publish` operation.

use std::path::Path;
use std::fs::File;

use serde_json;

use errors::{PsqlpackResult, PsqlpackResultExt};
use errors::PsqlpackErrorKind::*;

#[derive(Deserialize)]
pub struct PublishProfile {
    pub version: String,
    #[serde(rename = "generationOptions")] pub generation_options: GenerationOptions,
}

#[derive(Deserialize)]
pub struct GenerationOptions {
    #[serde(rename = "alwaysRecreateDatabase")] pub always_recreate_database: bool,
    #[serde(rename = "allowUnsafeOperations")] pub allow_unsafe_operations: bool,
}

impl PublishProfile {
    pub fn new() -> Self {
        PublishProfile {
            version: "1.0".to_owned(),
            generation_options: GenerationOptions {
                always_recreate_database: false,
                allow_unsafe_operations: false,
            },
        }
    }

    pub fn from_path(profile_path: &Path) -> PsqlpackResult<PublishProfile> {
        File::open(profile_path)
            .chain_err(|| PublishProfileReadError(profile_path.to_path_buf()))
            .and_then(|file| {
                serde_json::from_reader(file).chain_err(|| PublishProfileParseError(profile_path.to_path_buf()))
            })
    }
}
