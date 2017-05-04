extern crate chrono;
extern crate clap;
extern crate pg_dacpac;

use std::env;
use std::path::Path;
use std::time::Instant;

use clap::{Arg, ArgMatches, App, SubCommand};
use pg_dacpac::{Dacpac, DacpacResult, DacpacErrorKind, ParseError};

fn main() {
    let matches = App::new("DACPAC for PostgreSQL")
        .version("1.0")
        .author("Paul Mason <paul.mason@xero.com>")
        .subcommand(SubCommand::with_name("package")
            .about("creates a DACPAC from the specified target")
            .arg(Arg::with_name("SOURCE")
                .long("source")
                .required(false)
                .takes_value(true)
                .help("The source project JSON file"))
            .arg(Arg::with_name("OUT")
                .long("out")
                .required(true)
                .takes_value(true)
                .help("The location of the folder to export the dacpac to")))
        .subcommand(SubCommand::with_name("publish")
            .about("publishes a DACPAC to target")
            .arg(Arg::with_name("SOURCE")
                .long("source")
                .required(true)
                .takes_value(true)
                .help("The source dacpac to use for publishing"))
            .arg(Arg::with_name("TARGET")
                .long("target")
                .required(true)
                .takes_value(true)
                .help("The target database to publish to"))
            .arg(Arg::with_name("PROFILE")
                .long("profile")
                .required(true)
                .takes_value(true)
                .help("The publish profile to use for publishing")))
        .subcommand(SubCommand::with_name("script")
            .about("outputs the SQL file that would be executed against the target")
            .arg(Arg::with_name("SOURCE")
                .long("source")
                .required(false)
                .takes_value(true)
                .help("The source dacpac to use for the deploy report"))
            .arg(Arg::with_name("TARGET")
                .long("target")
                .required(true)
                .takes_value(true)
                .help("The target database to compare to"))
            .arg(Arg::with_name("PROFILE")
                .long("profile")
                .required(true)
                .takes_value(true)
                .help("The publish profile to use for the deploy report"))
            .arg(Arg::with_name("OUT")
                .long("out")
                .required(true)
                .takes_value(true)
                .help("The SQL file to generate")))
        .subcommand(SubCommand::with_name("report")
            .about("outputs a JSON deployment report for what would be executed against the \
                    target")
            .arg(Arg::with_name("SOURCE")
                .long("source")
                .required(true)
                .takes_value(true)
                .help("The source dacpac to use for the deploy report"))
            .arg(Arg::with_name("TARGET")
                .long("target")
                .required(true)
                .takes_value(true)
                .help("The target database to compare to"))
            .arg(Arg::with_name("PROFILE")
                .long("profile")
                .required(true)
                .takes_value(true)
                .help("The publish profile to use for the deploy report"))
            .arg(Arg::with_name("OUT")
                .long("out")
                .required(true)
                .takes_value(true)
                .help("The report file to generate")))
        .get_matches();

    // Time how long this takes
    let time_stamp = Instant::now();

    // Handle the user input.
    match handle(matches) {
        HandleResult::UnknownSubcommand => println!("Command is required"),
        HandleResult::Outcome(action, Err(error)) => {
            println!("Error encountered during {} command:", action);
            print_error(&error);
        }
        HandleResult::Outcome(action, _) => {
            // Capture how long was elapsed
            let elapsed = time_stamp.elapsed();
            let elapsed = elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1000_000_000.0;
            println!("Completed {} command in {}s", action, elapsed);
        }
    }
}

enum HandleResult {
    UnknownSubcommand,
    Outcome(String, DacpacResult<()>),
}

fn handle(matches: ArgMatches) -> HandleResult {
    // TODO: do some validation
    match matches.subcommand() {
        (command @ "package", Some(package)) => {
            // Source is a directory to begin with
            // If the source is provided, use that, else use the current dir + project.json
            let source = match package.value_of("SOURCE") {
                Some(cmd_source) => cmd_source.into(),
                None => {
                    let mut path = env::current_dir().unwrap().to_path_buf();
                    path.push("project.json");
                    path
                }
            };
            let output = Path::new(package.value_of("OUT").unwrap());
            HandleResult::Outcome(command.to_owned(), Dacpac::package_project(&source, &output))
        }
        (command @ "publish", Some(publish)) => {
            // Source is the dacpac, target is the DB
            let source = Path::new(publish.value_of("SOURCE").unwrap());
            let target = String::from(publish.value_of("TARGET").unwrap());
            let profile = Path::new(publish.value_of("PROFILE").unwrap());
            HandleResult::Outcome(command.to_owned(), Dacpac::publish(&source, target, &profile))
        }
        (command @ "script", Some(script)) => {
            // Source is the dacpac, target is the DB
            let source = Path::new(script.value_of("SOURCE").unwrap());
            let target = String::from(script.value_of("TARGET").unwrap());
            let profile = Path::new(script.value_of("PROFILE").unwrap());
            let output_file = Path::new(script.value_of("OUT").unwrap());
            HandleResult::Outcome(command.to_owned(),
                                  Dacpac::generate_sql(&source, target, &profile, &output_file))
        }
        (command @ "report", Some(report)) => {
            // Source is the dacpac, target is the DB
            let source = Path::new(report.value_of("SOURCE").unwrap());
            let target = String::from(report.value_of("TARGET").unwrap());
            let profile = Path::new(report.value_of("PROFILE").unwrap());
            let output_file = Path::new(report.value_of("OUT").unwrap());
            HandleResult::Outcome(command.to_owned(),
                                  Dacpac::generate_report(&source, target, &profile, &output_file))
        }
        _ => HandleResult::UnknownSubcommand,
    }
}

pub fn print_error(error: &DacpacErrorKind) {
    match *error {
        DacpacErrorKind::Connection(ref inner) => {
            println!("Invalid connection string");
            println!("  {}", inner);
            println!();
        }
        DacpacErrorKind::Msg(ref message) => {
            println!("Unknown Error");
            println!("  {}", message);
            println!();
        }
        DacpacErrorKind::IOError(ref file, ref message) => {
            println!("IO Error when reading {}", file);
            println!("  {}", message);
            println!();
        }
        DacpacErrorKind::FormatError(ref file, ref message) => {
            println!("Formatting Error when reading {}", file);
            println!("  {}", message);
            println!();
        }
        DacpacErrorKind::SyntaxError(ref file, ref line, line_number, start_pos, end_pos) => {
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
        }
        DacpacErrorKind::ParseError(ref file, ref errors) => {
            println!("Error in {}", file);
            for e in errors.iter() {
                match *e {
                    ParseError::InvalidToken { .. } => {
                        println!("  Invalid token");
                    }
                    ParseError::UnrecognizedToken {
                        ref token,
                        ref expected,
                    } => {
                        if let Some(ref x) = *token {
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
                    }
                    ParseError::ExtraToken { ref token } => {
                        println!("  Extra token detectd: {:?}", token);
                    }
                    ParseError::User { ref error } => {
                        println!("  {:?}", error);
                    }
                }
            }
            println!();
        }
        DacpacErrorKind::GenerationError(ref message) => {
            println!("Error generating DACPAC");
            println!("  {}", message);
            println!();
        }
        DacpacErrorKind::DatabaseError(ref message) => {
            println!("Database error:");
            println!("  {}", message);
            println!();
        }
        DacpacErrorKind::ProjectError(ref message) => {
            println!("Project format error:");
            println!("  {}", message);
            println!();
        }
        DacpacErrorKind::MultipleErrors(ref errors) => {
            for error in errors {
                print_error(error);
            }
        }
    }
}
