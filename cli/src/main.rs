extern crate clap;
extern crate psqlpack;

use std::env;
use std::path::Path;
use std::time::Instant;

use clap::{Arg, ArgMatches, App, SubCommand};
use psqlpack::{Psqlpack, PsqlpackResult, ChainedError};

fn main() {
    let matches = App::new("psqlpack")
        .version("1.0")
        .author("Paul Mason <paul.mason@xero.com>")
        .subcommand(SubCommand::with_name("package")
            .about("creates a psqlpack from the specified target")
            .arg(Arg::with_name("SOURCE")
                .long("source")
                .required(false)
                .takes_value(true)
                .help("The source project JSON file"))
            .arg(Arg::with_name("OUT")
                .long("out")
                .required(true)
                .takes_value(true)
                .help("The location of the folder to export the psqlpack to")))
        .subcommand(SubCommand::with_name("publish")
            .about("publishes a psqlpack to target")
            .arg(Arg::with_name("SOURCE")
                .long("source")
                .required(true)
                .takes_value(true)
                .help("The source psqlpack to use for publishing"))
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
                .help("The source psqlpack to use for the deploy report"))
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
            .about("outputs a JSON deployment report for proposed changes to the target")
            .arg(Arg::with_name("SOURCE")
                .long("source")
                .required(true)
                .takes_value(true)
                .help("The source psqlpack to use for the deploy report"))
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
            println!("Error encountered during {} command:\n{}",
                     action,
                     error.display());
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
    Outcome(String, PsqlpackResult<()>),
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
            HandleResult::Outcome(command.to_owned(),
                                  Psqlpack::package_project(&source, &output))
        }
        (command @ "publish", Some(publish)) => {
            // Source is the psqlpack, target is the DB
            let source = Path::new(publish.value_of("SOURCE").unwrap());
            let target = String::from(publish.value_of("TARGET").unwrap());
            let profile = Path::new(publish.value_of("PROFILE").unwrap());
            HandleResult::Outcome(command.to_owned(),
                                  Psqlpack::publish(&source, target, &profile))
        }
        (command @ "script", Some(script)) => {
            // Source is the psqlpack, target is the DB
            let source = Path::new(script.value_of("SOURCE").unwrap());
            let target = String::from(script.value_of("TARGET").unwrap());
            let profile = Path::new(script.value_of("PROFILE").unwrap());
            let output_file = Path::new(script.value_of("OUT").unwrap());
            HandleResult::Outcome(command.to_owned(),
                                  Psqlpack::generate_sql(&source, target, &profile, &output_file))
        }
        (command @ "report", Some(report)) => {
            // Source is the psqlpack, target is the DB
            let source = Path::new(report.value_of("SOURCE").unwrap());
            let target = String::from(report.value_of("TARGET").unwrap());
            let profile = Path::new(report.value_of("PROFILE").unwrap());
            let output_file = Path::new(report.value_of("OUT").unwrap());
            HandleResult::Outcome(command.to_owned(),
                                  Psqlpack::generate_report(&source,
                                                            target,
                                                            &profile,
                                                            &output_file))
        }
        _ => HandleResult::UnknownSubcommand,
    }
}
