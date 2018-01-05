use std::io::Read;
use std::path::{Path, PathBuf};
use std::fs::{self, File};

use slog::Logger;
use serde_json;
use walkdir::WalkDir;

use sql::ast::*;
use sql::{lexer, parser};
use model::Package;
use errors::{PsqlpackError, PsqlpackResult, PsqlpackResultExt};
use errors::PsqlpackErrorKind::*;

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

#[derive(Deserialize)]
pub struct Project {
    #[serde(skip_serializing)] path: Option<PathBuf>,
    pub version: String,
    #[serde(rename = "defaultSchema")] pub default_schema: String,
    #[serde(rename = "preDeployScripts")] pub pre_deploy_scripts: Vec<String>,
    #[serde(rename = "postDeployScripts")] pub post_deploy_scripts: Vec<String>,
    pub extensions: Option<Vec<String>>,
}

impl Project {
    #[allow(dead_code)]
    pub fn default() -> Self {
        Project {
            path: None,
            version: "1.0".into(),
            default_schema: "public".into(),
            pre_deploy_scripts: Vec::new(),
            post_deploy_scripts: Vec::new(),
            extensions: None,
        }
    }

    pub fn from_path(log: &Logger, path: &Path) -> PsqlpackResult<Project> {
        let log = log.new(o!("project" => "from_path"));
        trace!(log, "Attempting to open project file"; "path" => path.to_str().unwrap());
        File::open(path)
            .chain_err(|| ProjectReadError(path.to_path_buf()))
            .and_then(|file| {
                trace!(log, "Parsing project file");
                serde_json::from_reader(file).chain_err(|| ProjectParseError(path.to_path_buf()))
            })
            .and_then(|mut project: Project| {
                project.path = Some(path.to_path_buf());
                if project.default_schema.is_empty() {
                    project.default_schema = "public".into();
                }
                Ok(project)
            })
    }

    pub fn to_package(&self, log: &Logger, output_path: &Path) -> PsqlpackResult<()> {
        let log = log.new(o!("project" => "to_package"));

        // Turn the pre/post into paths to quickly check
        let parent = match self.path {
            Some(ref path) => path.parent().unwrap(),
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

        // Add extensions
        if let Some(ref extensions) = self.extensions {
            for extension in extensions {
                package.push_extension(ExtensionDefinition {
                    name: extension.clone(),
                });
            }
        }

        let mut errors: Vec<PsqlpackError> = Vec::new();

        // Enumerate the directory
        for entry in WalkDir::new(parent).follow_links(false) {
            // Read in the file contents
            let e = entry.unwrap();
            let path = e.path();
            if path.extension().is_none() || path.extension().unwrap() != "sql" {
                continue;
            }

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
                let tokens = match lexer::tokenize(&contents[..]) {
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
                match parser::parse_statement_list(tokens) {
                    Ok(statement_list) => {
                        trace!(log, "Finished parsing statements"; "count" => statement_list.len());
                        for statement in statement_list {
                            dump_statement!(log, statement);
                            match statement {
                                Statement::Extension(_) => warn!(log, "Extension statement found, ignoring"),
                                Statement::Function(function_definition) => package.push_function(function_definition),
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

        // Now generate the package
        trace!(log, "Creating package directory");
        if let Some(parent) = output_path.parent() {
            match fs::create_dir_all(parent) {
                Ok(_) => {}
                Err(e) => bail!(GenerationError(
                    format!("Failed to create package directory: {}", e)
                )),
            }
        }

        trace!(log, "Writing package file"; "output" => output_path.to_str().unwrap());
        package.write_to(output_path)
    }
}
