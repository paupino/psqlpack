use lexer::{self};
use lalrpop_util::ParseError;
use serde_json::{self};
use std::io::{self,Read};
use std::path::Path;
use std::fs::{self,DirEntry,File};
use std::result::Result as StdResult;
use sql::{self};

pub type DacpacErrors = Vec<String>;

pub struct Dacpac;

impl Dacpac {
    pub fn package_project(source_project_file: String, output_file: String) -> StdResult<(), DacpacErrors> {

        // Create a tax specification to populate via parsing
        let mut project_source = String::new();
        let project_path = Path::new(&source_project_file[..]);
        if let Err(err) = File::open(&project_path).and_then(|mut f| f.read_to_string(&mut project_source)) {
            return Err(vec!(format!("Input `{}`: I/O Error {}",
                     project_path.display(), err)));
        }
        let mut project = serde_json::from_str(&project_source).unwrap();

        // Visit the project directory
        let result = Dacpac::visit_dirs(project_path.parent().unwrap(), &mut project, &|spec_cb: &mut Project, e: &DirEntry| { 
            let path = e.path();
            if let Some(ext) = path.extension() {
                if ext == "sql" {
                    // Read in the file contents
                    let mut s = String::new();
                    if let Err(err) = File::open(&path).and_then(|mut f| f.read_to_string(&mut s)) {
                        println!("Input `{}`: I/O Error {}",
                                 path.display(), err);
                        println!();
                        return;
                    }

                    let tokens = match lexer::tokenize(&s[..]) {
                        Ok(t) => t,
                        Err(e) => {
                            println!("Syntax error in {} on line {}", path.display(), e.line_number);
                            println!("  {}", e.line);
                            print!("  ");
                            for i in 0..e.start_pos {
                                print!(" ");
                            }
                            for i in e.start_pos..e.end_pos {
                                print!("^");
                            }
                            println!();
                            return;
                        },
                    };

                    match sql::parse_statement_list(tokens) {
                        Ok(statement_list) => { 
                            for statement in statement_list {
                                println!("Parsed {} successfully", path.display());
                                println!();
                            }
                        },
                        Err(err) => { 
                            let errors = vec!(err);
                            println!("Error in {}", path.display());
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
        });
        Ok(())
    }

    // one possible implementation of walking a directory only visiting files
    fn visit_dirs(dir: &Path, project: &mut Project, cb: &Fn(&mut Project, &DirEntry)) -> io::Result<()> {
        if dir.is_dir() {
            for entry in try!(fs::read_dir(dir)) {
                let entry = try!(entry);
                let path = entry.path();

                if path.is_dir() {
                    try!(Dacpac::visit_dirs(&path, project, cb));
                } else {
                    cb(project, &entry);
                }
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct Project {
    default_schema: String,
}
