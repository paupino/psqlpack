extern crate chrono;
extern crate clap;
extern crate pg_dacpac;

use clap::{Arg, App, SubCommand};
use std::env;
use std::time::Instant;
use pg_dacpac::{Dacpac, DacpacErrorKind, ParseError};

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

    // Parse the subcommand
    // TODO: do some validation
    let action;
    if let Some(package) = matches.subcommand_matches("package") {
        // Source is a directory to begin with
        action = "Packaging";

        // If the source is provided, use that, else use the current dir + project.json
        let source;
        if let Some(cmd_source) = package.value_of("SOURCE") {
            source = cmd_source.to_owned();
        } else {
            let current_dir = env::current_dir().unwrap();
            source = format!("{}{}project.json",
                             current_dir.display(),
                             std::path::MAIN_SEPARATOR);
        }
        let output = String::from(package.value_of("OUT").unwrap());
        match Dacpac::package_project(source, output) {
            Ok(_) => {}
            Err(error) => {
                print_error(&error);
            }
        }
    } else if let Some(publish) = matches.subcommand_matches("publish") {
        action = "Publishing";
        // Source is the dacpac, target is the DB
        let source = String::from(publish.value_of("SOURCE").unwrap());
        let target = String::from(publish.value_of("TARGET").unwrap());
        let profile = String::from(publish.value_of("PROFILE").unwrap());
        match Dacpac::publish(source, target, profile) {
            Ok(_) => {}
            Err(error) => {
                print_error(&error);
            }
        }
    } else if let Some(script) = matches.subcommand_matches("script") {
        action = "SQL File Generation";
        // Source is the dacpac, target is the DB
        let source = String::from(script.value_of("SOURCE").unwrap());
        let target = String::from(script.value_of("TARGET").unwrap());
        let profile = String::from(script.value_of("PROFILE").unwrap());
        let output_file = String::from(script.value_of("OUT").unwrap());
        match Dacpac::generate_sql(source, target, profile, output_file) {
            Ok(_) => {}
            Err(error) => {
                print_error(&error);
            }
        }
    } else if let Some(report) = matches.subcommand_matches("report") {
        action = "Report Generation";
        // Source is the dacpac, target is the DB
        let source = String::from(report.value_of("SOURCE").unwrap());
        let target = String::from(report.value_of("TARGET").unwrap());
        let profile = String::from(report.value_of("PROFILE").unwrap());
        let output_file = String::from(report.value_of("OUT").unwrap());
        match Dacpac::generate_report(source, target, profile, output_file) {
            Ok(_) => {}
            Err(error) => {
                print_error(&error);
            }
        }
    } else {
        println!("Subcommand is required");
        std::process::exit(1);
    }

    // Capture how long was elapsed
    let elapsed = time_stamp.elapsed();
    let elapsed = elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1000_000_000.0;
    println!("{} took {}s", action, elapsed);
}

pub fn print_error(error: &DacpacErrorKind) {
    match *error {
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
        DacpacErrorKind::InvalidConnectionString(ref message) => {
            println!("Invalid connection string");
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
                    ParseError::UnrecognizedToken { ref token, ref expected } => {
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
