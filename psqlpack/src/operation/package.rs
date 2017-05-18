use std::path::Path;
use std::fs::{self, File};
use std::io::Read;

use walkdir::WalkDir;

use ast::*;
use lexer;
use sql;
use model::{Project, Package};
use errors::{PsqlpackError, PsqlpackResult, PsqlpackResultExt};
use errors::PsqlpackErrorKind::*;

pub fn package(project_path: &Path, output_path: &Path) -> PsqlpackResult<()> {
    // Load the project
    let project = Project::from_path(project_path)?;

    // Turn the pre/post into paths to quickly check
    let parent = project_path.parent().unwrap();
    let make_path = |script: &str| {
        parent
            .join(Path::new(script))
            .canonicalize()
            .chain_err(|| InvalidScriptPath(script.to_owned()))
    };

    let mut predeploy_paths = Vec::new();
    for script in &project.pre_deploy_scripts {
        predeploy_paths.push(make_path(script)?);
    }

    let mut postdeploy_paths = Vec::new();
    for script in &project.post_deploy_scripts {
        postdeploy_paths.push(make_path(script)?);
    }

    // Start the package
    let mut package = Package::new();
    let mut errors: Vec<PsqlpackError> = Vec::new();

    // Enumerate the directory
    for entry in WalkDir::new(parent).follow_links(false) {
        // Read in the file contents
        let e = entry.unwrap();
        let path = e.path();
        if path.extension().is_none() || path.extension().unwrap() != "sql" {
            continue;
        }

        let mut contents = String::new();
        if let Err(err) = File::open(&path).and_then(|mut f| f.read_to_string(&mut contents)) {
            errors.push(IOError(format!("{}", path.display()), format!("{}", err)).into());
            continue;
        }

        // Figure out if it's a pre/post deployment script
        let real_path = path.to_path_buf().canonicalize().unwrap();
        if let Some(pos) = predeploy_paths.iter().position(|x| real_path.eq(x)) {
            package.push_script(ScriptDefinition {
                name: path.file_name().unwrap().to_str().unwrap().to_owned(),
                kind: ScriptKind::PreDeployment,
                order: pos,
                contents: contents
            });
        } else if let Some(pos) = postdeploy_paths.iter().position(|x| real_path.eq(x)) {
            package.push_script(ScriptDefinition {
                name: path.file_name().unwrap().to_str().unwrap().to_owned(),
                kind: ScriptKind::PostDeployment,
                order: pos,
                contents: contents
            });
        } else {
            let tokens = match lexer::tokenize(&contents[..]) {
                Ok(t) => t,
                Err(e) => {
                    errors.push(SyntaxError(
                        format!("{}", path.display()),
                        e.line.to_owned(),
                        e.line_number as usize,
                        e.start_pos as usize,
                        e.end_pos as usize,
                    ).into());
                    continue;
                },
            };

            match sql::parse_statement_list(tokens) {
                Ok(statement_list) => {
                    for statement in statement_list {
                        match statement {
                            Statement::Extension(extension_definition) => package.push_extension(extension_definition),
                            Statement::Function(function_definition) => package.push_function(function_definition),
                            Statement::Schema(schema_definition) => package.push_schema(schema_definition),
                            Statement::Table(table_definition) => package.push_table(table_definition),
                            Statement::Type(type_definition) => package.push_type(type_definition),
                        }
                    }
                },
                Err(err) => {
                    errors.push(ParseError(format!("{}", path.display()), vec!(err)).into());
                    continue;
                }
            }
        }
    }

    // Early exit if errors
    if !errors.is_empty() {
        bail!(MultipleErrors(errors));
    }

    // Update any missing defaults, create a dependency graph and then try to validate the project
    package.set_defaults(&project);
    try!(package.generate_dependency_graph());
    try!(package.validate());

    // Now generate the prackage
    if let Some(parent) = output_path.parent() {
        match fs::create_dir_all(format!("{}", parent.display())) {
            Ok(_) => {},
            Err(e) => bail!(GenerationError(format!("Failed to create package directory: {}", e))),
        }
    }

    package.write_to(output_path)
}
