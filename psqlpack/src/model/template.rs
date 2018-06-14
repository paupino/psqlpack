use std::fs::File;
use std::path::Path;

use serde_json;

use super::PublishProfile;
use errors::{PsqlpackError, PsqlpackResult, PsqlpackResultExt};
use errors::PsqlpackErrorKind::*;

pub fn generate_project(output_path: &Path, name: &str) -> PsqlpackResult<()> {
    Ok(())
}

pub fn generate_publish_profile(output_path: &Path, name: &str) -> PsqlpackResult<()> {
    let publish_profile = PublishProfile::default();
    let mut output = output_path.to_path_buf();
    output.push(format!("{}.publish", name));
    File::create(&output)
        .chain_err(|| TemplateGenerationError(format!("Failed to create file at: {}", output.to_str().unwrap())))
        .and_then(|file| {
            serde_json::to_writer_pretty(file, &publish_profile)
                .chain_err(|| TemplateGenerationError(format!("Failed to serialize JSON for file at: {}", output.to_str().unwrap())))
        })?;
    Ok(())
}
