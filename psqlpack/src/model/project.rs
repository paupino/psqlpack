use std::default::Default;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use glob::glob;
use serde_json;
use slog::Logger;

use crate::Semver;
use crate::sql::ast::*;
use crate::sql::lexer;
use crate::sql::parser::StatementListParser;
use crate::model::Package;
use crate::errors::{PsqlpackError, PsqlpackResult, PsqlpackResultExt};
use crate::errors::PsqlpackErrorKind::*;

#[cfg(feature = "symbols")]
macro_rules! dump_statement {
    ($log:ident, $statement:ident) => {
        let log = $log.new(o!("symbols" => "ast"));
        trace!(log, "{:?}", $statement);
    };
}

#[cfg(not(feature = "symbols"))]
macro_rules! dump_statement {
    ($log:ident, $statement:ident) => {}
}

#[derive(Deserialize, Serialize)]
pub struct Project {
    // Internal only tracking for the project path
    #[serde(skip_serializing)] project_file_path: Option<PathBuf>,

    /// The version of this profile file format
    pub version: Semver,

    /// The default schema for the database. Typically `public`
    #[serde(rename = "defaultSchema")]
    pub default_schema: String,

    /// An array of scripts to run before anything is deployed
    #[serde(rename = "preDeployScripts")]
    pub pre_deploy_scripts: Vec<String>,

    /// An array of scripts to run after everything has been deployed
    #[serde(rename = "postDeployScripts")]
    pub post_deploy_scripts: Vec<String>,

    /// An array of extensions to include within this project
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<Dependency>>,

    /// An array of globs representing files/folders to be included within your project. Defaults to `["**/*.sql"]`.
    #[serde(rename = "fileIncludeGlobs", skip_serializing_if = "Option::is_none")]
    pub include_globs: Option<Vec<String>>,

    /// An array of globs representing files/folders to be excluded within your project.
    #[serde(rename = "fileExcludeGlobs", skip_serializing_if = "Option::is_none")]
    pub exclude_globs: Option<Vec<String>>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Semver>,
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
                serde_json::from_reader(file).chain_err(|| ProjectParseError(project_file_path.to_path_buf()))
            })
            .and_then(|mut project: Project| {
                project.project_file_path = Some(project_file_path.to_path_buf());
                if project.default_schema.is_empty() {
                    project.default_schema = "public".into();
                }
                Ok(project)
            })
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
                    contents: contents,
                });
            } else if let Some(pos) = postdeploy_paths.iter().position(|x| real_path.eq(x)) {
                trace!(log, "Found postdeploy script");
                package.push_script(ScriptDefinition {
                    name: path.file_name().unwrap().to_str().unwrap().to_owned(),
                    kind: ScriptKind::PostDeployment,
                    order: pos,
                    contents: contents,
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
                            ).into(),
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
                                },
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
        package.set_defaults(self);
        trace!(log, "Validating package");
        package.validate()?;

        Ok(package)
    }

    // Walk the files according to the include and exclude globs. This could be made more efficient with an iterator
    // in the future (may want to extend glob). One downside of the current implementation is that pre/post deploy
    // scripts could be inadvertantly excluded
    fn walk_files(&self, parent: &Path) -> PsqlpackResult<Vec<PathBuf>> {
        let include_globs = match self.include_globs {
            Some(ref globs) => globs.to_owned(),
            None => vec![ "**/*.sql".into() ],
        };
        let mut exclude_paths = Vec::new();
        match self.exclude_globs {
            Some(ref globs) => {
                for exclude_glob in globs {
                    for entry in glob(&format!("{}/{}", parent.to_str().unwrap(), exclude_glob)).map_err(|err| GlobPatternError(err))? {
                        let path = entry.unwrap().canonicalize().unwrap();
                        exclude_paths.push(path);
                    }
                }
            }
            None => {}
        }

        let mut paths = Vec::new();
        for include_glob in include_globs {
            for entry in glob(&format!("{}/{}", parent.to_str().unwrap(), include_glob)).map_err(|err| GlobPatternError(err))? {
                // Get the path entry
                let path = entry.unwrap();

                // If this has been explicitly excluded then continue
                let real_path = path.to_path_buf().canonicalize().unwrap();
                if exclude_paths.iter().position(|x| real_path.eq(x)).is_some() {
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

    use std::path::Path;
    use super::Project;
    use spectral::prelude::*;

    #[test]
    fn it_can_iterate_default_include_exclude_globs_correctly() {
        // This test relies on the `simple` samples directory
        let parent = Path::new("../samples/simple");
        let project = Project::default();
        let result = project.walk_files(&parent);

        // Check the expected files were returned
        assert_that!(result).is_ok().has_length(4);
        let result = result.unwrap();
        let result : Vec<&str> = result.iter().map(|x| x.to_str().unwrap()).collect();
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
        };
        let result = project.walk_files(&parent);

        // Check the expected files were returned
        assert_that!(result).is_ok().has_length(3);
        let result = result.unwrap();
        let result : Vec<&str> = result.iter().map(|x| x.to_str().unwrap()).collect();
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
            exclude_globs: Some(vec![
                "**/geo/**/*.sql".into(),
                "**/geo.*".into(),
            ]),
        };
        let result = project.walk_files(&parent);

        // Check the expected files were returned
        assert_that!(result).is_ok();
        let result = result.unwrap();
        let result : Vec<&str> = result.iter().map(|x| x.to_str().unwrap()).collect();
        assert_that!(result).does_not_contain(&"../samples/complex/geo/functions/fn_do_any_coordinates_fall_inside.sql");
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
        };
        let result = project.walk_files(&parent);

        // Check the expected files were returned
        assert_that!(result).is_ok().has_length(1);
        let result = result.unwrap();
        let result : Vec<&str> = result.iter().map(|x| x.to_str().unwrap()).collect();
        assert_that!(result).contains_all_of(&vec![
            &"../samples/simple/public/tables/public.organisation.sql",
        ]);
    }

}
