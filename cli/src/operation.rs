use std::path::Path;

use slog::Logger;

use psqlpack::{Delta, Package, Project, PsqlpackResult, PsqlpackErrorKind, PublishProfile};

pub fn package<L: Into<Logger>>(log: L, project_path: &Path, output_path: &Path) -> PsqlpackResult<()> {
    let log = log.into().new(o!("operation" => "package"));
    trace!(log, "Loading Project from path"; "source" => project_path.to_str().unwrap());
    let project = Project::from_path(&log, project_path)?;
    trace!(log, "Writing Project to Package"; "output" => output_path.to_str().unwrap());
    project.to_package(&log, output_path)
}

pub fn extract<L: Into<Logger>>(log: L, source_connection_string: &str, target_package_path: &Path) -> PsqlpackResult<()> {
    let log = log.into().new(o!("operation" => "extract"));
    let connection = source_connection_string.parse()?;

    trace!(log, "Loading Package from connection");
    let package = Package::from_connection(&log, &connection)?;
    match package {
        Some(data) => {
            trace!(log, "Writing Package"; "output" => target_package_path.to_str().unwrap());
            data.write_to(target_package_path)
        }
        None => Err(PsqlpackErrorKind::PackageCreationError("database does not exist".into()).into()),
    }
}

pub fn publish<L: Into<Logger>>(
    log: L,
    source_package_path: &Path,
    target_connection_string: &str,
    publish_profile: &Path,
) -> PsqlpackResult<()> {
    let log = log.into().new(o!("operation" => "publish"));
    let package = Package::from_path(source_package_path)?;
    let publish_profile = PublishProfile::from_path(publish_profile)?;
    let connection = target_connection_string.parse()?;

    // Now we generate our instructions
    let target_package = Package::from_connection(&log, &connection)?;
    let target_database_name = connection.database().to_owned();
    let delta = Delta::generate(
        &log,
        &package,
        target_package,
        target_database_name,
        publish_profile,
    )?;
    delta.apply(&log, &connection)
}

pub fn generate_sql<L: Into<Logger>>(
    log: L,
    source_package_path: &Path,
    target_connection_string: &str,
    publish_profile: &Path,
    output_file: &Path,
) -> PsqlpackResult<()> {
    let log = log.into().new(o!("operation" => "generate_sql"));
    let package = Package::from_path(source_package_path)?;
    let publish_profile = PublishProfile::from_path(publish_profile)?;
    let connection = target_connection_string.parse()?;

    // Now we generate our instructions
    let target_package = Package::from_connection(&log, &connection)?;
    let target_database_name = connection.database().to_owned();
    let delta = Delta::generate(
        &log,
        &package,
        target_package,
        target_database_name,
        publish_profile,
    )?;
    delta.write_sql(&log, output_file)
}

pub fn generate_report<L: Into<Logger>>(
    log: L,
    source_package_path: &Path,
    target_connection_string: &str,
    publish_profile: &Path,
    output_file: &Path,
) -> PsqlpackResult<()> {
    let log = log.into().new(o!("operation" => "generate_report"));
    let package = Package::from_path(source_package_path)?;
    let publish_profile = PublishProfile::from_path(publish_profile)?;
    let connection = target_connection_string.parse()?;

    // Now we generate our instructions
    let target_package = Package::from_connection(&log, &connection)?;
    let target_database_name = connection.database().to_owned();
    let delta = Delta::generate(
        &log,
        &package,
        target_package,
        target_database_name,
        publish_profile,
    )?;
    delta.write_report(output_file)
}