extern crate psqlpack;
#[macro_use]
extern crate slog;

#[macro_use]
mod common;

use psqlpack::*;
use slog::{Discard, Drain, Logger};

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
    delta.apply(&log, &connection).ok();

    // Confirm db exists with data
    assert_simple_package!(DB_NAME, NAMESPACE, connection);
}