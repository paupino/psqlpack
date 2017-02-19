use ast::*;
use lexer::{self};
use lalrpop_util::ParseError;
use serde_json::{self};
use std::io::{self,Read};
use std::io::prelude::*;
use std::path::Path;
use std::fs::{self,DirEntry,File};
use std::result::Result as StdResult;
use sql::{self};
use walkdir::WalkDir;
use zip::{ZipArchive,ZipWriter};
use zip::write::FileOptions;

macro_rules! ztry {
    ($expr:expr) => {{ 
        match $expr {
            Ok(_) => {},
            Err(e) => return Err(vec!(DacpacError::GenerationError { 
                message: format!("Failed to write DACPAC: {}", e),
            })),
        }
    }};
}

macro_rules! load_file {
    ($file_type:ty, $coll:ident, $file:ident) => {{
        let mut contents = String::new();
        $file.read_to_string(&mut contents).unwrap();
        let object : $file_type = serde_json::from_str(&contents).unwrap();
        $coll.push(object);
    }};
}

pub struct Dacpac;

impl Dacpac {
    pub fn package_project(source_project_file: String, output_file: String) -> StdResult<(), Vec<DacpacError>> {

        // Load the project file
        let project_path = Path::new(&source_project_file[..]);
        if !project_path.is_file() {
            return Err(vec!(DacpacError::IOError {
                file: format!("{}", project_path.display()),
                message: "Project file does not exist".to_owned(),
            }));
        }
        let mut project_source = String::new();
        if let Err(err) = File::open(&project_path).and_then(|mut f| f.read_to_string(&mut project_source)) {
            return Err(vec!(DacpacError::IOError {
                     file: format!("{}", project_path.display()),
                     message: format!("Failed to read project file: {}", err)
                 }));
        }

        // Load the project
        let project_config : ProjectConfig = serde_json::from_str(&project_source).unwrap();
        let mut project = Project::new();
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
        project.set_defaults(project_config);
        try!(project.validate());

        // Now generate the dacpac
        let output_path = Path::new(&output_file[..]);
        if output_path.parent().is_some() {
            fs::create_dir_all(format!("{}", output_path.parent().unwrap().display()));
        }

        let output_file = match File::create(&output_path) {
            Ok(f) => f,
            Err(e) => return Err(vec!(DacpacError::GenerationError { 
                message: format!("Failed to write DACPAC: {}", e),
            })),
        };
        let mut zip = ZipWriter::new(output_file);

        ztry!(zip.add_directory("tables/", FileOptions::default()));

        for table in project.tables {
            ztry!(zip.start_file(format!("tables/{}.json", table.name), FileOptions::default()));
            let json = match serde_json::to_string_pretty(&table) {
                Ok(j) => j,
                Err(e) => return Err(vec!(DacpacError::GenerationError {
                    message: format!("Failed to write DACPAC: {}", e),
                })),
            };
            ztry!(zip.write_all(json.as_bytes()));
        }
        ztry!(zip.finish());

        Ok(())
    }

    pub fn publish(source_dacpac_file: String, target_connection_string: String, publish_profile: String) -> StdResult<(), Vec<DacpacError>> {
        
        let project = try!(Dacpac::load_project(source_dacpac_file));
        let publish_profile = try!(Dacpac::load_publish_profile(publish_profile));
        try!(Dacpac::test_connection(target_connection_string));

        Ok(())
    }

    pub fn generate_report(source_dacpac_file: String, target_connection_string: String, publish_profile: String) -> StdResult<(), Vec<DacpacError>> {

        let project = try!(Dacpac::load_project(source_dacpac_file));
        let publish_profile = try!(Dacpac::load_publish_profile(publish_profile));
        try!(Dacpac::test_connection(target_connection_string));

        Ok(())
    }

    fn load_project(source_dacpac_file: String) -> StdResult<Project, Vec<DacpacError>> {
        // Load the DACPAC
        let source_path = Path::new(&source_dacpac_file[..]);
        if !source_path.is_file() {
            return Err(vec!(DacpacError::IOError {
                file: format!("{}", source_path.display()),
                message: "DACPAC file does not exist".to_owned(),
            }));
        }
        let file = match fs::File::open(&source_path) {
            Ok(o) => o,
            Err(e) => return Err(vec!(DacpacError::IOError {
                file: format!("{}", source_path.display()),
                message: format!("Failed to open DACPAC file: {}", e),
            })),
        };
        let mut archive = match ZipArchive::new(file) {
            Ok(o) => o,
            Err(e) => return Err(vec!(DacpacError::IOError {
                file: format!("{}", source_path.display()),
                message: format!("Failed to open DACPAC file: {}", e),
            })),
        };

        let mut tables = Vec::new();
        for i in 0..archive.len()
        {
            let mut file = archive.by_index(i).unwrap();
            if file.size() == 0 {
                continue;
            }
            if file.name().starts_with("tables/") {
                load_file!(TableDefinition, tables, file);
            }
        }
        Ok(Project {
            tables: tables
        })
    }

    fn load_publish_profile(publish_profile: String) -> StdResult<PublishProfile, Vec<DacpacError>> {
        Err(vec!())
    }

    fn test_connection(target_connection_string: String) -> StdResult<(), Vec<DacpacError>> {
        Ok(())
    }
}

#[derive(Deserialize)]
struct ProjectConfig {
    default_schema: String,
}

struct Project {
    tables: Vec<TableDefinition>,
}

impl Project {

    fn new() -> Self {
        Project {
            tables: Vec::new(),
        }
    }

    fn push_table(&mut self, table: TableDefinition) {
        self.tables.push(table);
    }

    fn set_defaults(&mut self, config: ProjectConfig) { 

        // Set default schema's
        for table in self.tables.iter_mut() {
            if table.name.schema.is_none() {
                table.name.schema = Some(config.default_schema.clone());
            }
        }
    }

    fn validate(&self) -> Result<(), Vec<DacpacError>> {

        // TODO: Validate references etc
        Ok(())
    }
}

struct PublishProfile;

pub enum DacpacError {
    IOError { file: String, message: String },
    SyntaxError { file: String, line: String, line_number: i32, start_pos: i32, end_pos: i32 },
    ParseError { file: String, errors: Vec<ParseError<(), lexer::Token, ()>> },
    GenerationError { message: String },
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
            },
            DacpacError::GenerationError { ref message } => {
                println!("Error generating DACPAC");
                println!("  {}", message);
                println!();
            },
        }        
    }
}