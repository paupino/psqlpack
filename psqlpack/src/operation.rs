use std::path::Path;

use model::{Project, PublishProfile, Package, Delta};
use errors::PsqlpackResult;

pub fn package(project_path: &Path, output_path: &Path) -> PsqlpackResult<()> {
    let project = Project::from_path(project_path)?;
    project.to_package(output_path)
}

pub fn publish(source_package_path: &Path, target_connection_string: &str, publish_profile: &Path) -> PsqlpackResult<()> {
    let package = Package::from_path(source_package_path)?;
    let publish_profile = PublishProfile::from_path(publish_profile)?;
    let connection = target_connection_string.parse()?;

    // Now we generate our instructions
    let delta = Delta::generate(&package, &connection, publish_profile)?;
    delta.apply(&connection)
}

pub fn generate_sql(source_package_path: &Path, target_connection_string: &str, publish_profile: &Path, output_file: &Path) -> PsqlpackResult<()> {
    let package = Package::from_path(source_package_path)?;
    let publish_profile = PublishProfile::from_path(publish_profile)?;
    let connection = target_connection_string.parse()?;

    // Now we generate our instructions
    let delta = Delta::generate(&package, &connection, publish_profile)?;
    delta.write_sql(output_file)
}

pub fn generate_report(source_package_path: &Path, target_connection_string: &str, publish_profile: &Path, output_file: &Path) -> PsqlpackResult<()> {
    let package = Package::from_path(source_package_path)?;
    let publish_profile = PublishProfile::from_path(publish_profile)?;
    let connection = target_connection_string.parse()?;

    // Now we generate our instructions
    let delta = Delta::generate(&package, &connection, publish_profile)?;
    delta.write_report(output_file)
}
