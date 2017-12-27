extern crate psqlpack;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate spectral;

#[macro_use]
mod common;

use psqlpack::*;
use psqlpack::ast::*;
use slog::{Discard, Drain, Logger};
use spectral::prelude::*;

#[test]
fn it_can_create_a_database_that_doesnt_exist() {
    const DB_NAME : &str = "psqlpack_new_db";
    const NAMESPACE : &str = "it_can_create_a_database_that_doesnt_exist";

    // Preliminary: remove existing database
    let connection = ConnectionBuilder::new(DB_NAME, "localhost", "postgres").build().unwrap();
    drop_db!(connection);

    // Create a package
    let package = generate_simple_package!(NAMESPACE);

    // Use the default publish profile
    let publish_profile = PublishProfile::new();

    // Create a target package from connection string
    let log = Logger::root(Discard.fuse(), o!());
    let target_package = Package::from_connection(&log, &connection).unwrap();

    // Generate delta and apply
    let delta = Delta::generate(
        &log,
        &package,
        target_package,
        DB_NAME.into(),
        publish_profile,
    ).unwrap();
    delta.apply(&log, &connection).unwrap();

    // Confirm db exists with data
    let final_package = Package::from_connection(&log, &connection).unwrap().unwrap();
    assert_that!(final_package).is_equal_to(&package);
    assert_simple_package!(final_package, NAMESPACE);
}