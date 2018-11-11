use std::path::Path;

use slog::Logger;

use psqlpack::{
    Capabilities,
    Delta,
    Package,
    Project,
    PsqlpackResult,
    PsqlpackErrorKind,
    PublishProfile,
    Semver,
    template
};

pub fn package<L: Into<Logger>>(log: L, project_file: &Path, output_path: &Path) -> PsqlpackResult<()> {
    let log = log.into().new(o!("operation" => "package"));
    trace!(log, "Loading Project from project file"; "source" => project_file.to_str().unwrap());
    let project = Project::from_project_file(&log, project_file)?;
    trace!(log, "Generating Package from Project");
    let package = project.build_package(&log)?;
    trace!(log, "Writing Package"; "output" => output_path.to_str().unwrap());
    package.write_to(output_path)
}

pub fn extract_database<L: Into<Logger>>(log: L, source_connection_string: &str, target_package_path: &Path) -> PsqlpackResult<()> {
    let log = log.into().new(o!("operation" => "extract_database"));
    let connection = source_connection_string.parse()?;

    trace!(log, "Loading Server Capabilities");
    let capabilities = Capabilities::from_connection(&log, &connection)?;

    trace!(log, "Loading Package from connection");
    let package = Package::from_connection(&log, &connection, &capabilities)?;
    match package {
        Some(data) => {
            trace!(log, "Writing Package"; "output" => target_package_path.to_str().unwrap());
            data.write_to(target_package_path)
        }
        None => Err(PsqlpackErrorKind::PackageCreationError("database does not exist".into()).into()),
    }
}

pub fn extract_extension<L: Into<Logger>>(log: L,
                                          source_connection_string: &str,
                                          target_path: &Path,
                                          extension_name: String,
                                          extension_version: Option<Semver>)
    -> PsqlpackResult<()> {
    let log = log.into().new(o!("operation" => "extract_extension"));
    let connection = source_connection_string.parse()?;

    trace!(log, "Loading Server Capabilities");
    let capabilities = Capabilities::from_connection(&log, &connection)?;

    trace!(log, "Loading Extension from connection");
    let extensions = capabilities.available_extensions(&extension_name, extension_version);
    if !extensions.is_empty() {
        if !extensions[0].installed {
            return Err(PsqlpackErrorKind::ExtractError("Extension was found but not installed".into()).into())
        }
    } else if let Some(version) = extension_version {
        return Err(PsqlpackErrorKind::ExtractError(format!("No extension found with version {}", version)).into())
    } else {
        return Err(PsqlpackErrorKind::ExtractError("No extension found".into()).into())
    }
    let extension = extensions[0];
    let package = extension.build_package_from_connection(&log, &connection, &capabilities)?;
    let mut output_path = target_path.to_path_buf();
    output_path.push(format!("{}-{}.psqlpack", extension.name, extension.version));
    trace!(log, "Writing Package"; "output" => output_path.to_str().unwrap());
    package.write_to(&output_path)
}

pub fn generate_template<L: Into<Logger>>(log: L, template: &str, output_path: &Path, name: &str) -> PsqlpackResult<()> {
    let log = log.into().new(o!("operation" => "generate_template"));
    match template {
        "project" => {
            info!(log, "Generating project"; "destination" => output_path.to_str().unwrap());
            template::generate_project(output_path, name)
        }
        "publishprofile" | "publish_profile" => {
            info!(log, "Generating publish profile"; "destination" => output_path.to_str().unwrap());
            template::generate_publish_profile(output_path, name)
        }
        unrecognized => Err(PsqlpackErrorKind::TemplateGenerationError(format!("Template not found: {}", unrecognized)).into())
    }
}

pub fn publish<L: Into<Logger>>(
    log: L,
    source_file: &Path,
    target_connection_string: &str,
    publish_profile: &Path,
) -> PsqlpackResult<()> {
    let log = log.into().new(o!("operation" => "publish"));
    let package = Package::from_path(&log, source_file)?;
    let publish_profile = PublishProfile::from_path(publish_profile)?;
    let connection = target_connection_string.parse()?;

    trace!(log, "Loading Server Capabilities");
    let capabilities = Capabilities::from_connection(&log, &connection)?;

    // Now we generate our instructions
    let target_package = Package::from_connection(&log, &connection, &capabilities)?;
    let target_database_name = connection.database().to_owned();
    let delta = Delta::generate(
        &log,
        &package,
        target_package,
        target_database_name,
        capabilities,
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
    let package = Package::from_path(&log, source_package_path)?;
    let publish_profile = PublishProfile::from_path(publish_profile)?;
    let connection = target_connection_string.parse()?;

    trace!(log, "Loading Server Capabilities");
    let capabilities = Capabilities::from_connection(&log, &connection)?;

    // Now we generate our instructions
    let target_package = Package::from_connection(&log, &connection, &capabilities)?;
    let target_database_name = connection.database().to_owned();
    let delta = Delta::generate(
        &log,
        &package,
        target_package,
        target_database_name,
        capabilities,
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
    let package = Package::from_path(&log, source_package_path)?;
    let publish_profile = PublishProfile::from_path(publish_profile)?;
    let connection = target_connection_string.parse()?;

    trace!(log, "Loading Server Capabilities");
    let capabilities = Capabilities::from_connection(&log, &connection)?;

    // Now we generate our instructions
    let target_package = Package::from_connection(&log, &connection, &capabilities)?;
    let target_database_name = connection.database().to_owned();
    let delta = Delta::generate(
        &log,
        &package,
        target_package,
        target_database_name,
        capabilities,
        publish_profile,
    )?;
    delta.write_report(output_file)
}
