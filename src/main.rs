extern crate chrono;
#[macro_use] 
extern crate clap;
#[macro_use]
extern crate lazy_static;
extern crate lalrpop_util;
extern crate regex;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate walkdir;

mod ast;
mod gen;
mod lexer;
mod sql;

use std::time::Instant;
use gen::{Dacpac};

fn main() {
    let matches = clap_app!(myapp =>
            (version: "1.0")
            (author: "Paul Mason <paul.mason@xero.com>")
            (about: "DACPAC for PostgreSQL")
            (@subcommand package =>
                (about: "creates a DACPAC from the specified target")
                (@arg SOURCE: --source +required +takes_value "The source project JSON file")
                (@arg OUT: --out +required +takes_value "The location of the folder to export the dacpac to")
            )
            (@subcommand publish =>
                (about: "publishes a DACPAC to a the specified database")
                (@arg SOURCE: --source +required +takes_value "The source dacpac to use for publishing")
                (@arg TARGET: --target +required +takes_value "The target database to publish to")
                (@arg PROFILE: --profile +required +takes_value "The publish profile to use for publishing")
            )
            (@subcommand report =>
                (about: "outputs a deployment report for a DACPAC to a the specified database")
                (@arg SOURCE: --source +required +takes_value "The source dacpac to use for the deploy report")
                (@arg TARGET: --target +required +takes_value "The target database to compare to")
                (@arg PROFILE: --profile +required +takes_value "The publish profile to use for the deploy report")
            )
        ).get_matches();

    // Time how long this takes
    let time_stamp = Instant::now();

    // Parse the subcommand
    if let Some(package) = matches.subcommand_matches("package") {
        // Source is a directory to begin with
        let source = String::from(package.value_of("SOURCE").unwrap());
        let output = String::from(package.value_of("OUT").unwrap());
        match Dacpac::package_project(source, output) {
            Ok(_) => { },
            Err(errors) => { 
                for error in errors {
                    error.print();
                }
            }
        }
    } else if let Some(publish) = matches.subcommand_matches("publish") {
        // Source is the dacpac
        //source = String::from(publish.value_of("SOURCE").unwrap());
        // Target is the database
        //target = Some(String::from(publish.value_of("TARGET").unwrap()));
        //profile = Some(String::from(publish.value_of("PROFILE").unwrap()));
    } else if let Some(report) = matches.subcommand_matches("report") {
        // Source is the dacpac
        //source = String::from(report.value_of("SOURCE").unwrap());
        // Target is the database
        //target = Some(String::from(report.value_of("TARGET").unwrap()));
        //profile = Some(String::from(report.value_of("PROFILE").unwrap()));
    } else {
        println!("Subcommand is required");
        std::process::exit(1);
    } 

    // Capture how long was elapsed
    let elapsed = time_stamp.elapsed();
    let elapsed = elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1000_000_000.0;
    println!("Action took {}s", elapsed);
}
