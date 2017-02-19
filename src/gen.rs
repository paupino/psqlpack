use ast::*;
use lexer::{self};
use lalrpop_util::ParseError;
use serde_json::{self};
use std::io::{self,Read};
use std::path::Path;
use std::fs::{self,DirEntry,File};
use std::result::Result as StdResult;
use sql::{self};
use walkdir::WalkDir;

pub enum DacpacError {
    IOError { file: String, message: String },
    SyntaxError { file: String, line: String, line_number: i32, start_pos: i32, end_pos: i32 },
    ParseError { file: String, errors: Vec<ParseError<(), lexer::Token, ()>> },
}

pub struct Dacpac;

impl Dacpac {
    pub fn package_project(source_project_file: String, output_file: String) -> StdResult<(), Vec<DacpacError>> {

        // Create a tax specification to populate via parsing
        let mut project_source = String::new();
        let project_path = Path::new(&source_project_file[..]);
        if let Err(err) = File::open(&project_path).and_then(|mut f| f.read_to_string(&mut project_source)) {
            return Err(vec!(DacpacError::IOError {
                     file: format!("{}", project_path.display()),
                     message: format!("Failed to read project file: {}", err)
                 }));
        }

        // Load the project
        let project_config : ProjectConfig = serde_json::from_str(&project_source).unwrap();
        let mut project = Project::new(project_config);
        let mut errors = Vec::new();

        // Enumerate the directory
        for entry in WalkDir::new(project_path.parent().unwrap()).follow_links(false) {
            // Read in the file contents
            let e = entry.unwrap();
            let path = e.path();
            if path.extension().is_none() || path.extension().unwrap() != "sql" {
                continue;
            }

            let mut contents = String::new();
            if let Err(err) = File::open(&path).and_then(|mut f| f.read_to_string(&mut contents)) {
                errors.push(DacpacError::IOError { 
                    file: format!("{}", path.display()), 
                    message: format!("{}", err) 
                });
                continue;
            }

            let tokens = match lexer::tokenize(&contents[..]) {
                Ok(t) => t,
                Err(e) => {
                    errors.push(DacpacError::SyntaxError { 
                        file: format!("{}", path.display()), 
                        line: e.line.to_owned(), 
                        line_number: e.line_number, 
                        start_pos: e.start_pos, 
                        end_pos: e.end_pos 
                    });
                    continue;
                },
            };

            match sql::parse_statement_list(tokens) {
                Ok(statement_list) => { 
                    for statement in statement_list {
                        match statement {
                            Statement::Table(table_definition) => project.push_table(table_definition),
                        }
                    }
                },
                Err(err) => { 
                    errors.push(DacpacError::ParseError { 
                        file: format!("{}", path.display()), 
                        errors: vec!(err), 
                    });
                    continue;
                }
            }            
        }

        // Early exit if errors
        if !errors.is_empty() {
            return Err(errors);
        }

        // First up validate the dacpac
        project.set_defaults();
        try!(project.validate());

        // Now generate the dacpac
        for table in project.tables {
            println!("{}", serde_json::to_string_pretty(&table).unwrap());
        }

        Ok(())
    }
}

#[derive(Deserialize)]
struct ProjectConfig {
    default_schema: String,
}

struct Project {
    config: ProjectConfig,
    tables: Vec<TableDefinition>,
}

impl Project {

    fn new(config: ProjectConfig) -> Self {
        Project {
            config: config,
            tables: Vec::new(),
        }
    }

    fn push_table(&mut self, table: TableDefinition) {
        self.tables.push(table);
    }

    fn set_defaults(&mut self) { 

        // Set default schema's
        for table in self.tables.iter_mut() {
            if table.name.schema.is_none() {
                table.name.schema = Some(self.config.default_schema.clone());
            }
        }
    }

    fn validate(&self) -> Result<(), Vec<DacpacError>> {

        // TODO: Validate references etc
        Ok(())
    }
}

impl DacpacError {
    pub fn print(&self) {
        match *self {
            DacpacError::IOError { ref file, ref message } => {
                println!("IO Error when reading {}", file);
                println!("  {}", message);
                println!();
            },
            DacpacError::SyntaxError { ref file, ref line, line_number, start_pos, end_pos } => {
                println!("Syntax error in {} on line {}", file, line_number);
                println!("  {}", line);
                print!("  ");
                for _ in 0..start_pos {
                    print!(" ");
                }
                for _ in start_pos..end_pos {
                    print!("^");
                }
                println!();
            },
            DacpacError::ParseError { ref file, ref errors } => {
                println!("Error in {}", file);
                for e in errors.iter() {
                    match *e {
                        ParseError::InvalidToken { ref location } => { 
                            println!("  Invalid token");
                        },
                        ParseError::UnrecognizedToken { ref token, ref expected } => {
                            if let &Some(ref x) = token {
                                println!("  Unexpected {:?}.", x.1);
                            } else {
                                println!("  Unexpected end of file");
                            }
                            print!("  Expected one of: ");
                            let mut first = true;
                            for expect in expected {
                                if first {
                                    first = false;
                                } else {
                                    print!(", ");
                                }
                                print!("{}", expect);
                            }
                            println!();
                        },
                        ParseError::ExtraToken { ref token } => {
                            println!("  Extra token detectd: {:?}", token);
                        },
                        ParseError::User { ref error } => {
                            println!("  {:?}", error);
                        },
                    }
                }
                println!();                            
            }
        }        
    }
}