use std::default::Default;
use std::fmt;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use glob::glob;
use serde_json;
use slog::Logger;

use crate::errors::PsqlpackErrorKind::*;
use crate::errors::{PsqlpackError, PsqlpackResult, PsqlpackResultExt};
use crate::model::Package;
use crate::sql::ast::*;
use crate::sql::lexer;
use crate::sql::parser::StatementListParser;
use crate::Semver;

#[cfg(feature = "symbols")]
macro_rules! dump_statement {
    ($log:ident, $statement:ident) => {
        let log = $log.new(o!("symbols" => "ast"));
        trace!(log, "{:?}", $statement);
    };
}

#[cfg(not(feature = "symbols"))]
macro_rules! dump_statement {
    ($log:ident, $statement:ident) => {};
}

#[derive(Deserialize, Serialize)]
pub struct Project {
    // Internal only tracking for the project path
    #[serde(skip_serializing)]
    project_file_path: Option<PathBuf>,

    /// The version of this profile file format
    pub version: Semver,

    /// The default schema for the database. Typically `public`
    #[serde(alias = "defaultSchema")]
    pub default_schema: String,

    /// An array of scripts to run before anything is deployed
    #[serde(alias = "preDeployScripts")]
    pub pre_deploy_scripts: Vec<String>,

    /// An array of scripts to run after everything has been deployed
    #[serde(alias = "postDeployScripts")]
    pub post_deploy_scripts: Vec<String>,

    /// An array of extensions to include within this project
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<Dependency>>,

    /// An array of globs representing files/folders to be included within your project. Defaults to `["**/*.sql"]`.
    #[serde(
        alias = "fileIncludeGlobs",
        alias = "file_include_globs",
        skip_serializing_if = "Option::is_none"
    )]
    pub include_globs: Option<Vec<String>>,

    /// An array of globs representing files/folders to be excluded within your project.
    #[serde(
        alias = "fileExcludeGlobs",
        alias = "file_exclude_globs",
        skip_serializing_if = "Option::is_none"
    )]
    pub exclude_globs: Option<Vec<String>>,

    /// An array of search paths to look in outside of the standard paths (./lib, ~/.psqlpack/lib).
    #[serde(alias = "referenceSearchPaths", skip_serializing_if = "Option::is_none")]
    pub reference_search_paths: Option<Vec<String>>,
}

#[derive(Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Semver>,
}

impl fmt::Display for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(version) = self.version {
            write!(f, "{}-{}", self.name, version)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

impl Default for Project {
    fn default() -> Self {
        Project {
            project_file_path: None,
            version: "1.0".into(),
            default_schema: "public".into(),
            pre_deploy_scripts: Vec::new(),
            post_deploy_scripts: Vec::new(),
            extensions: None,
            include_globs: None,
            exclude_globs: None,
            reference_search_paths: None,
        }
    }
}

impl Project {
    pub fn from_project_file(log: &Logger, project_file_path: &Path) -> PsqlpackResult<Project> {
        let log = log.new(o!("project" => "from_project_file"));
        trace!(log, "Attempting to open project file"; "project_file_path" => project_file_path.to_str().unwrap());
        File::open(project_file_path)
            .chain_err(|| ProjectReadError(project_file_path.to_path_buf()))
            .and_then(|file| {
                trace!(log, "Parsing project file");
                Self::from_reader(file)
            })
            .and_then(|mut project: Project| {
                project.project_file_path = Some(project_file_path.to_path_buf());
                if project.default_schema.is_empty() {
                    project.default_schema = "public".into();
                }
                Ok(project)
            })
    }

    fn from_reader<R>(reader: R) -> PsqlpackResult<Project>
    where
        R: Read,
    {
        let mut buffered_reader = BufReader::new(reader);
        let mut contents = String::new();
        if buffered_reader
            .read_to_string(&mut contents)
            .chain_err(|| ProjectParseError("Failed to read contents".into()))?
            == 0
        {
            bail!(ProjectParseError("Data was empty".into()))
        }

        let trimmed = contents.trim_start();
        if trimmed.starts_with('{') {
            serde_json::from_str(&contents).chain_err(|| ProjectParseError("Failed to read JSON".into()))
        } else {
            toml::from_str(&contents).chain_err(|| ProjectParseError("Failed to read TOML".into()))
        }
    }

    pub fn build_package(&self, log: &Logger) -> PsqlpackResult<Package> {
        let log = log.new(o!("project" => "build_package"));

        // Turn the pre/post into paths to quickly check
        let parent = match self.project_file_path {
            Some(ref path) => path.parent().unwrap().canonicalize().unwrap(),
            None => bail!(GenerationError("Project path not set".to_owned())),
        };
        let make_path = |script: &str| {
            parent
                .join(Path::new(script))
                .canonicalize()
                .chain_err(|| InvalidScriptPath(script.to_owned()))
        };

        trace!(log, "Canonicalizing predeploy paths");
        let mut predeploy_paths = Vec::new();
        for script in &self.pre_deploy_scripts {
            predeploy_paths.push(make_path(script)?);
        }
        trace!(log, "Done predeploy paths"; "count" => predeploy_paths.len());

        trace!(log, "Canonicalizing postdeploy paths");
        let mut postdeploy_paths = Vec::new();
        for script in &self.post_deploy_scripts {
            postdeploy_paths.push(make_path(script)?);
        }
        trace!(log, "Done postdeploy paths"; "count" => postdeploy_paths.len());

        // Start the package
        let mut package = Package::new();
        let mut errors: Vec<PsqlpackError> = Vec::new();

        // Add extensions into package
        if let Some(ref extensions) = self.extensions {
            for extension in extensions {
                package.push_extension(Dependency {
                    name: extension.name.clone(),
                    version: extension.version,
                });
            }
        }

        // Enumerate the glob paths
        for path in self.walk_files(&parent)? {
            let log = log.new(o!("file" => path.to_str().unwrap().to_owned()));

            let mut contents = String::new();
            if let Err(err) = File::open(&path).and_then(|mut f| f.read_to_string(&mut contents)) {
                error!(log, "Error reading file");
                errors.push(IOError(format!("{}", path.display()), format!("{}", err)).into());
                continue;
            }

            // Figure out if it's a pre/post deployment script
            let real_path = path.to_path_buf().canonicalize().unwrap();
            if let Some(pos) = predeploy_paths.iter().position(|x| real_path.eq(x)) {
                trace!(log, "Found predeploy script");
                package.push_script(ScriptDefinition {
                    name: path.file_name().unwrap().to_str().unwrap().to_owned(),
                    kind: ScriptKind::PreDeployment,
                    order: pos,
                    contents,
                });
            } else if let Some(pos) = postdeploy_paths.iter().position(|x| real_path.eq(x)) {
                trace!(log, "Found postdeploy script");
                package.push_script(ScriptDefinition {
                    name: path.file_name().unwrap().to_str().unwrap().to_owned(),
                    kind: ScriptKind::PostDeployment,
                    order: pos,
                    contents,
                });
            } else {
                trace!(log, "Tokenizing file");
                let tokens = match lexer::tokenize_stmt(&contents[..]) {
                    Ok(t) => t,
                    Err(e) => {
                        errors.push(
                            SyntaxError(
                                format!("{}", path.display()),
                                e.line.to_owned(),
                                e.line_number as usize,
                                e.start_pos as usize,
                                e.end_pos as usize,
                            )
                            .into(),
                        );
                        continue;
                    }
                };
                trace!(log, "Finished tokenizing"; "count" => tokens.len());

                trace!(log, "Parsing file");
                // TODO: In the future it'd be nice to allow the parser to generate
                //       shift/reduce rules when dump-symbols is defined
                match StatementListParser::new().parse(tokens) {
                    Ok(statement_list) => {
                        trace!(log, "Finished parsing statements"; "count" => statement_list.len());
                        for statement in statement_list {
                            dump_statement!(log, statement);
                            match statement {
                                Statement::Error(kind) => {
                                    errors.push(HandledParseError(kind).into());
                                }
                                Statement::Function(function_definition) => package.push_function(function_definition),
                                Statement::Index(index_definition) => package.push_index(index_definition),
                                Statement::Schema(schema_definition) => package.push_schema(schema_definition),
                                Statement::Table(table_definition) => package.push_table(table_definition),
                                Statement::Type(type_definition) => package.push_type(type_definition),
                            }
                        }
                    }
                    Err(err) => {
                        errors.push(ParseError(format!("{}", path.display()), vec![err]).into());
                        continue;
                    }
                }
            }
        }

        // Early exit if errors
        if !errors.is_empty() {
            bail!(MultipleErrors(errors));
        }

        // Update any missing defaults, then try to validate the project
        trace!(log, "Setting defaults");
        package.set_defaults(self);
        trace!(log, "Load references");
        let references = package.load_references(self, &log);
        trace!(log, "Validating package");
        package.validate(&references)?;

        Ok(package)
    }

    // Walk the files according to the include and exclude globs. This could be made more efficient with an iterator
    // in the future (may want to extend glob). One downside of the current implementation is that pre/post deploy
    // scripts could be inadvertantly excluded
    fn walk_files(&self, parent: &Path) -> PsqlpackResult<Vec<PathBuf>> {
        let include_globs = match self.include_globs {
            Some(ref globs) => globs.to_owned(),
            None => vec!["**/*.sql".into()],
        };
        let mut exclude_paths = Vec::new();
        if let Some(ref globs) = self.exclude_globs {
            for exclude_glob in globs {
                for entry in
                    glob(&format!("{}/{}", parent.to_str().unwrap(), exclude_glob)).map_err(GlobPatternError)?
                {
                    let path = entry.unwrap().canonicalize().unwrap();
                    exclude_paths.push(path);
                }
            }
        }

        let mut paths = Vec::new();
        for include_glob in include_globs {
            for entry in glob(&format!("{}/{}", parent.to_str().unwrap(), include_glob)).map_err(GlobPatternError)? {
                // Get the path entry
                let path = entry.unwrap();

                // If this has been explicitly excluded then continue
                let real_path = path.to_path_buf().canonicalize().unwrap();
                if exclude_paths.iter().any(|x| real_path.eq(x)) {
                    continue;
                }

                paths.push(path);
            }
        }
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {

    use crate::model::project::Project;
    use crate::{Dependency, Semver};
    use spectral::prelude::*;
    use std::path::Path;

    #[test]
    fn it_can_iterate_default_include_exclude_globs_correctly() {
        // This test relies on the `simple` samples directory
        let parent = Path::new("../samples/simple");
        let project = Project::default();
        let result = project.walk_files(&parent);

        // Check the expected files were returned
        assert_that!(result).is_ok().has_length(4);
        let result = result.unwrap();
        let result: Vec<&str> = result.iter().map(|x| x.to_str().unwrap()).collect();
        assert_that!(result).contains_all_of(&vec![
            &"../samples/simple/public/tables/public.expense_status.sql",
            &"../samples/simple/public/tables/public.organisation.sql",
            &"../samples/simple/public/tables/public.tax_table.sql",
            &"../samples/simple/public/tables/public.vendor.sql",
        ]);
    }

    #[test]
    fn it_can_iterate_custom_exclude_globs_correctly() {
        // This test relies on the `simple` samples directory
        let parent = Path::new("../samples/simple");
        let project = Project {
            project_file_path: None,
            version: "1.0".into(),
            default_schema: "public".into(),
            pre_deploy_scripts: Vec::new(),
            post_deploy_scripts: Vec::new(),
            extensions: None,
            include_globs: None,
            exclude_globs: Some(vec!["**/*org*".into()]),
            reference_search_paths: None,
        };
        let result = project.walk_files(&parent);

        // Check the expected files were returned
        assert_that!(result).is_ok().has_length(3);
        let result = result.unwrap();
        let result: Vec<&str> = result.iter().map(|x| x.to_str().unwrap()).collect();
        assert_that!(result).does_not_contain(&"../samples/simple/public/tables/public.organisation.sql");
        assert_that!(result).contains_all_of(&vec![
            &"../samples/simple/public/tables/public.expense_status.sql",
            &"../samples/simple/public/tables/public.tax_table.sql",
            &"../samples/simple/public/tables/public.vendor.sql",
        ]);
    }

    #[test]
    fn it_can_iterate_custom_exclude_globs_correctly_2() {
        // This test relies on the `simple` samples directory
        let parent = Path::new("../samples/complex");
        let project = Project {
            project_file_path: None,
            version: "1.0".into(),
            default_schema: "public".into(),
            pre_deploy_scripts: Vec::new(),
            post_deploy_scripts: Vec::new(),
            extensions: None,
            include_globs: None,
            exclude_globs: Some(vec!["**/geo/**/*.sql".into(), "**/geo.*".into()]),
            reference_search_paths: None,
        };
        let result = project.walk_files(&parent);

        // Check the expected files were returned
        assert_that!(result).is_ok();
        let result = result.unwrap();
        let result: Vec<&str> = result.iter().map(|x| x.to_str().unwrap()).collect();
        assert_that!(result)
            .does_not_contain(&"../samples/complex/geo/functions/fn_do_any_coordinates_fall_inside.sql");
        assert_that!(result).does_not_contain(&"../samples/complex/geo/tables/states.sql");
        assert_that!(result).does_not_contain(&"../samples/complex/scripts/seed/geo.states.sql");
    }

    #[test]
    fn it_can_iterate_custom_include_globs_correctly() {
        // This test relies on the `simple` samples directory
        let parent = Path::new("../samples/simple");
        let project = Project {
            project_file_path: None,
            version: "1.0".into(),
            default_schema: "public".into(),
            pre_deploy_scripts: Vec::new(),
            post_deploy_scripts: Vec::new(),
            extensions: None,
            include_globs: Some(vec!["**/*org*.sql".into()]),
            exclude_globs: None,
            reference_search_paths: None,
        };
        let result = project.walk_files(&parent);

        // Check the expected files were returned
        assert_that!(result).is_ok().has_length(1);
        let result = result.unwrap();
        let result: Vec<&str> = result.iter().map(|x| x.to_str().unwrap()).collect();
        assert_that!(result).contains_all_of(&vec![&"../samples/simple/public/tables/public.organisation.sql"]);
    }

    #[test]
    fn it_can_add_read_a_project_in_json_format() {
        const DATA: &str = r#"
            {
                "version": "1.0",
                "defaultSchema": "public",
                "preDeployScripts": [
                    "scripts/pre-deploy/drop-something.sql"
                ],
                "postDeployScripts": [
                    "scripts/seed/seed1.sql",
                    "scripts/seed/seed2.sql"
                ],
                "fileExcludeGlobs": [
                    "**/ex/**/*.sql",
                    "**/ex.*"
                ],
                "extensions": [
                    { "name": "postgis" },
                    { "name": "postgis_topology" },
                    { "name": "postgis_tiger_geocoder" }
                ]
            }
        "#;
        let project = Project::from_reader(DATA.as_bytes());
        let project = project.unwrap();
        assert_that!(project.version).is_equal_to(Semver::new(1, 0, None));
        assert_that!(project.default_schema).is_equal_to("public".to_owned());

        assert_that!(project.pre_deploy_scripts).has_length(1);
        assert_that!(project.pre_deploy_scripts[0]).is_equal_to("scripts/pre-deploy/drop-something.sql".to_owned());

        assert_that!(project.post_deploy_scripts).has_length(2);
        assert_that!(project.post_deploy_scripts[0]).is_equal_to("scripts/seed/seed1.sql".to_owned());
        assert_that!(project.post_deploy_scripts[1]).is_equal_to("scripts/seed/seed2.sql".to_owned());

        assert_that!(project.exclude_globs).is_some();
        let exclude_globs = project.exclude_globs.unwrap();
        assert_that!(exclude_globs).has_length(2);
        assert_that!(exclude_globs[0]).is_equal_to("**/ex/**/*.sql".to_owned());
        assert_that!(exclude_globs[1]).is_equal_to("**/ex.*".to_owned());

        assert_that!(project.extensions).is_some();
        let extensions = project.extensions.unwrap();
        assert_that!(extensions).has_length(3);
        assert_that!(extensions[0]).is_equal_to(Dependency {
            name: "postgis".into(),
            version: None,
        });
        assert_that!(extensions[1]).is_equal_to(Dependency {
            name: "postgis_topology".into(),
            version: None,
        });
        assert_that!(extensions[2]).is_equal_to(Dependency {
            name: "postgis_tiger_geocoder".into(),
            version: None,
        });
    }

    #[test]
    fn it_can_add_read_a_project_in_toml_format() {
        const DATA: &str = r#"
            version = "1.0"
            default_schema = "public"
            pre_deploy_scripts = [
                "scripts/pre-deploy/drop-something.sql"
            ]
            post_deploy_scripts = [
                "scripts/seed/seed1.sql",
                "scripts/seed/seed2.sql"
            ]
            file_exclude_globs = [
                "**/ex/**/*.sql",
                "**/ex.*"
            ]
            extensions = [
                { name = "postgis" },
                { name = "postgis_topology" },
                { name = "postgis_tiger_geocoder" }
            ]
        "#;
        let project = Project::from_reader(DATA.as_bytes());
        let project = project.unwrap();
        assert_that!(project.version).is_equal_to(Semver::new(1, 0, None));
        assert_that!(project.default_schema).is_equal_to("public".to_owned());

        assert_that!(project.pre_deploy_scripts).has_length(1);
        assert_that!(project.pre_deploy_scripts[0]).is_equal_to("scripts/pre-deploy/drop-something.sql".to_owned());

        assert_that!(project.post_deploy_scripts).has_length(2);
        assert_that!(project.post_deploy_scripts[0]).is_equal_to("scripts/seed/seed1.sql".to_owned());
        assert_that!(project.post_deploy_scripts[1]).is_equal_to("scripts/seed/seed2.sql".to_owned());

        assert_that!(project.exclude_globs).is_some();
        let exclude_globs = project.exclude_globs.unwrap();
        assert_that!(exclude_globs).has_length(2);
        assert_that!(exclude_globs[0]).is_equal_to("**/ex/**/*.sql".to_owned());
        assert_that!(exclude_globs[1]).is_equal_to("**/ex.*".to_owned());

        assert_that!(project.extensions).is_some();
        let extensions = project.extensions.unwrap();
        assert_that!(extensions).has_length(3);
        assert_that!(extensions[0]).is_equal_to(Dependency {
            name: "postgis".into(),
            version: None,
        });
        assert_that!(extensions[1]).is_equal_to(Dependency {
            name: "postgis_topology".into(),
            version: None,
        });
        assert_that!(extensions[2]).is_equal_to(Dependency {
            name: "postgis_tiger_geocoder".into(),
            version: None,
        });
    }
}
