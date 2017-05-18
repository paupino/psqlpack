use std::path::Path;
use std::fs::File;

use serde_json;

use errors::{PsqlpackResult, PsqlpackResultExt};
use errors::PsqlpackErrorKind::*;

#[derive(Deserialize)]
pub struct Project {
    pub version: String,
    #[serde(rename = "defaultSchema")]
    pub default_schema: String,
    #[serde(rename = "preDeployScripts")]
    pub pre_deploy_scripts: Vec<String>,
    #[serde(rename = "postDeployScripts")]
    pub post_deploy_scripts: Vec<String>,
}

impl Project {
    pub fn from_path(path: &Path) -> PsqlpackResult<Project> {
        File::open(path)
        .chain_err(|| ProjectReadError(path.to_path_buf()))
        .and_then(|file| {
            serde_json::from_reader(file)
            .chain_err(|| ProjectParseError(path.to_path_buf()))
        })
    }
}
