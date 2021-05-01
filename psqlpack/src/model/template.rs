use std::fs::{self, File};
use std::path::Path;

use serde::ser::Serialize;

use super::{Project, PublishProfile};
use crate::errors::PsqlpackErrorKind::*;
use crate::errors::{PsqlpackResult, PsqlpackResultExt};

fn assert_directory_exists(path: &Path) -> PsqlpackResult<()> {
    if path.exists() {
        // If it is a file or we don't have permissions then we have a problem
        if !path.is_dir() {
            bail!(TemplateGenerationError(format!(
                "Provided output directory is either a file or does not have permissions for write: {}",
                path.to_str().unwrap()
            )))
        }
    } else {
        // Create the directory tree
        fs::create_dir_all(path).chain_err(|| {
            TemplateGenerationError(format!("Failed to create directory tree: {}", path.to_str().unwrap()))
        })?;
    }
    Ok(())
}

fn save_json<T: Serialize>(path: &Path, obj: &T) -> PsqlpackResult<()> {
    File::create(path)
        .chain_err(|| TemplateGenerationError(format!("Failed to create file at: {}", path.to_str().unwrap())))
        .and_then(|file| {
            serde_json::to_writer_pretty(file, obj).chain_err(|| {
                TemplateGenerationError(format!(
                    "Failed to serialize JSON for file at: {}",
                    path.to_str().unwrap()
                ))
            })
        })?;
    Ok(())
}

pub fn generate_project(output_path: &Path, name: &str) -> PsqlpackResult<()> {
    assert_directory_exists(output_path)?;

    // Our base directory already exists. To set up the project we'll create a directory,
    //  set up a default project file as well as a default publish profile.
    let mut output = output_path.to_path_buf();
    output.push(name);
    fs::create_dir(&output).chain_err(|| {
        TemplateGenerationError(format!(
            "Failed to create project directory: {}",
            output.to_str().unwrap()
        ))
    })?;

    // Default project file
    let project = Project::default();
    output.push(format!("{}.psqlproj", name));
    save_json(&output, &project)?;

    // Finally, set up a default publish profile
    let publish_profile = PublishProfile::default();
    output.set_file_name("local.publish");
    save_json(&output, &publish_profile)
}

pub fn generate_publish_profile(output_path: &Path, name: &str) -> PsqlpackResult<()> {
    assert_directory_exists(output_path)?;

    let mut output = output_path.to_path_buf();

    // Create a default publish profile and create it at the location specified
    let publish_profile = PublishProfile::default();
    output.push(format!("{}.publish", name));
    save_json(&output, &publish_profile)
}
