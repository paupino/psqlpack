#[macro_use]
extern crate clap;
extern crate psqlpack;
#[macro_use]
extern crate slog;
extern crate slog_term;

use std::env;
use std::path::Path;
use std::time::Instant;
use std::sync::{atomic, Arc};
use std::sync::atomic::Ordering;
use std::result;

use clap::{App, Arg, ArgMatches, SubCommand};
use slog::{Drain, Logger};
use psqlpack::{operation, ChainedError, PsqlpackResult};

/// A threadsafe toggle.
#[derive(Clone)]
struct Toggle(Arc<atomic::AtomicBool>);

impl Toggle {
    pub fn new() -> Toggle {
        Toggle(Arc::new(atomic::AtomicBool::new(false)))
    }

    pub fn get(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    pub fn set(&self, value: bool) {
        self.0.store(value, Ordering::Relaxed)
    }
}

/// A `slog::Drain` that toggles the trace level.
struct TraceFilter<D: Drain> {
    drain: D,
    toggle: Toggle,
}

impl<D: Drain> TraceFilter<D> {
    pub fn new(drain: D, toggle: Toggle) -> TraceFilter<D> {
        TraceFilter {
            drain: drain,
            toggle: toggle,
        }
    }
}

impl<D: Drain> Drain for TraceFilter<D> {
    type Ok = Option<D::Ok>;
    type Err = Option<D::Err>;

    fn log(&self, record: &slog::Record, values: &slog::OwnedKVList) -> result::Result<Self::Ok, Self::Err> {
        let current_level = if self.toggle.get() {
            slog::Level::Trace
        } else {
            slog::Level::Info
        };

        if record.level().is_at_least(current_level) {
            self.drain.log(record, values).map(Some).map_err(Some)
        } else {
            Ok(None)
        }
    }
}

fn main() {
    let trace_on = Toggle::new();

    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::CompactFormat::new(decorator).build();
    let drain = TraceFilter::new(drain, trace_on.clone()).fuse();
    let drain = std::sync::Mutex::new(drain).fuse();
    let log = slog::Logger::root(drain, o!());

    let matches = App::new("psqlpack")
        .version(crate_version!())
        .author(crate_authors!())
        .subcommand(
            SubCommand::with_name("package")
                .about("creates a psqlpack from the specified target")
                .arg(
                    Arg::with_name("SOURCE")
                        .long("source")
                        .required(false)
                        .takes_value(true)
                        .help("The source project JSON file"),
                )
                .arg(
                    Arg::with_name("OUT")
                        .long("out")
                        .required(true)
                        .takes_value(true)
                        .help("The location of the folder to export the psqlpack to"),
                ),
        )
        .subcommand(
            SubCommand::with_name("extract")
                .about("creates a psqlpack from an existing database")
                .arg(
                    Arg::with_name("SOURCE")
                        .long("source")
                        .required(false)
                        .takes_value(true)
                        .help("The source database connection string"),
                )
                .arg(
                    Arg::with_name("OUT")
                        .long("out")
                        .required(true)
                        .takes_value(true)
                        .help("The location of the folder to export the psqlpack to"),
                ),
        )
        .subcommand(
            SubCommand::with_name("publish")
                .about("publishes a psqlpack to target")
                .arg(
                    Arg::with_name("SOURCE")
                        .long("source")
                        .required(true)
                        .takes_value(true)
                        .help("The source package to use for publishing"),
                )
                .arg(
                    Arg::with_name("TARGET")
                        .long("target")
                        .required(true)
                        .takes_value(true)
                        .help("The target database to publish to"),
                )
                .arg(
                    Arg::with_name("PROFILE")
                        .long("profile")
                        .required(true)
                        .takes_value(true)
                        .help("The publish profile to use for publishing"),
                ),
        )
        .subcommand(
            SubCommand::with_name("script")
                .about("outputs the SQL file that would be executed against the target")
                .arg(
                    Arg::with_name("SOURCE")
                        .long("source")
                        .required(false)
                        .takes_value(true)
                        .help("The source package to use for the deploy report"),
                )
                .arg(
                    Arg::with_name("TARGET")
                        .long("target")
                        .required(true)
                        .takes_value(true)
                        .help("The target database to compare to"),
                )
                .arg(
                    Arg::with_name("PROFILE")
                        .long("profile")
                        .required(true)
                        .takes_value(true)
                        .help("The publish profile to use for the deploy report"),
                )
                .arg(
                    Arg::with_name("OUT")
                        .long("out")
                        .required(true)
                        .takes_value(true)
                        .help("The SQL file to generate"),
                ),
        )
        .subcommand(
            SubCommand::with_name("report")
                .about("outputs a JSON deployment report for proposed changes to the target")
                .arg(
                    Arg::with_name("SOURCE")
                        .long("source")
                        .required(true)
                        .takes_value(true)
                        .help("The source package to use for the deploy report"),
                )
                .arg(
                    Arg::with_name("TARGET")
                        .long("target")
                        .required(true)
                        .takes_value(true)
                        .help("The target database to compare to"),
                )
                .arg(
                    Arg::with_name("PROFILE")
                        .long("profile")
                        .required(true)
                        .takes_value(true)
                        .help("The publish profile to use for the deploy report"),
                )
                .arg(
                    Arg::with_name("OUT")
                        .long("out")
                        .required(true)
                        .takes_value(true)
                        .help("The report file to generate"),
                ),
        )
        .arg(
            Arg::with_name("trace")
                .short("t")
                .long("trace")
                .global(true)
                .help("Enables trace level logging"),
        )
        .get_matches();

    // Checks if a flag is present at the top level or in any subcommand.
    fn is_present_recursive<'args, S: Into<&'args str>>(matches: &ArgMatches, flag: S) -> bool {
        let flag = flag.into();
        match matches.subcommand() {
            (_, Some(sub)) => is_present_recursive(sub, flag) || matches.is_present(flag),
            (_, None) => matches.is_present(flag),
        }
    }

    // Enable tracing if required.
    trace_on.set(is_present_recursive(&matches, "trace"));
    trace!(log, "psqlpack started");

    // Time how long this takes
    let time_stamp = Instant::now();

    // Handle the user input.
    match handle(&log, &matches) {
        HandleResult::UnknownSubcommand => {
            error!(
                log,
                "No command found\nCommand is required\nFor more information try --help"
            );
        }
        HandleResult::Outcome(action, Err(error)) => {
            error!(
                log,
                "encountered during {} command:\n{}",
                action,
                error.display_chain()
            );
        }
        HandleResult::Outcome(action, _) => {
            // Capture how long was elapsed
            let elapsed = time_stamp.elapsed();
            let elapsed = elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1000_000_000.0;
            info!(log, "Completed {} command in {}s", action, elapsed);
        }
    }
}

enum HandleResult {
    UnknownSubcommand,
    Outcome(String, PsqlpackResult<()>),
}

fn handle(log: &Logger, matches: &ArgMatches) -> HandleResult {
    // TODO: do some validation
    match matches.subcommand() {
        (command @ "package", Some(package)) => {
            let log = log.new(o!("command" => command.to_owned()));
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
            info!(log, "Project file path"; "source" => source.to_str().unwrap());
            let output = Path::new(package.value_of("OUT").unwrap());
            info!(log, "Output path"; "output" => output.to_str().unwrap());
            let result = operation::package(log, &source, output);
            HandleResult::Outcome(command.to_owned(), result)
        }
        (command @ "extract", Some(extract)) => {
            let log = log.new(o!("command" => command.to_owned()));
            // Source is a DB, target is a path
            let source = String::from(extract.value_of("SOURCE").unwrap());
            info!(log, "Source connection string"; "source" => &source);
            let output = Path::new(extract.value_of("OUT").unwrap());
            info!(log, "Output path"; "output" => output.to_str().unwrap());
            let result = operation::extract(log, &source, output);
            HandleResult::Outcome(command.to_owned(), result)
        }
        (command @ "publish", Some(publish)) => {
            let log = log.new(o!("command" => command.to_owned()));
            // Source is the psqlpack, target is the DB
            let source = Path::new(publish.value_of("SOURCE").unwrap());
            let target = String::from(publish.value_of("TARGET").unwrap());
            let profile = Path::new(publish.value_of("PROFILE").unwrap());
            let result = operation::publish(log, source, &target, profile);
            HandleResult::Outcome(command.to_owned(), result)
        }
        (command @ "script", Some(script)) => {
            let log = log.new(o!("command" => command.to_owned()));
            // Source is the psqlpack, target is the DB
            let source = Path::new(script.value_of("SOURCE").unwrap());
            let target = String::from(script.value_of("TARGET").unwrap());
            let profile = Path::new(script.value_of("PROFILE").unwrap());
            let output_file = Path::new(script.value_of("OUT").unwrap());
            let result = operation::generate_sql(log, source, &target, profile, output_file);
            HandleResult::Outcome(command.to_owned(), result)
        }
        (command @ "report", Some(report)) => {
            let log = log.new(o!("command" => command.to_owned()));
            // Source is the psqlpack, target is the DB
            let source = Path::new(report.value_of("SOURCE").unwrap());
            let target = String::from(report.value_of("TARGET").unwrap());
            let profile = Path::new(report.value_of("PROFILE").unwrap());
            let output_file = Path::new(report.value_of("OUT").unwrap());
            let result = operation::generate_report(log, source, &target, profile, output_file);
            HandleResult::Outcome(command.to_owned(), result)
        }
        _ => HandleResult::UnknownSubcommand,
    }
}
