use std::io::{self,Read};
use std::path::Path;
use std::fs::{self,DirEntry,File};
use std::result::Result as StdResult;
use sql::{self};

pub type DacpacErrors = Vec<String>;

pub struct Dacpac;

impl Dacpac {
    pub fn package_project(source_project_folder: String, output_file: String) -> StdResult<(), DacpacErrors> {

        // Create a tax specification to populate via parsing
        let mut project = Project::new();

        let path = Path::new(&source_project_folder[..]);

        // Visit the project directory
        let result = Dacpac::visit_dirs(path, &mut project, &|spec_cb: &mut Project, e: &DirEntry| { 
            let path = e.path();
            if let Some(ext) = path.extension() {
                if ext == "sql" {
                    // Read in the file contents
                    let mut s = String::new();
                    if let Err(err) = File::open(&path).and_then(|mut f| f.read_to_string(&mut s)) {
                        println!("Input `{}`: I/O Error {}",
                                 path.display(), err);
                        return;
                    }

                    match sql::parse_statement_list(&s[..]) {
                        Ok(statement_list) => { 
                            for statement in statement_list {
                                println!("Do something");
                            }
                        },
                        Err(err) => { 
                            let errors = vec!(err);
                            println!("Error in file {}", path.display());
                            for e in errors.iter() {
                                println!("  {:?}", e);     
                            }
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

struct Project;

impl Project {
    fn new() -> Project {
        Project {}
    }
}

