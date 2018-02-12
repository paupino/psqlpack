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

macro_rules! publish_simple_package {
    ($namespace:ident, $db_name:ident, $connection:ident) => {{
        // Create a package
        let package = generate_simple_package!($namespace);

        // Use the default publish profile
        let publish_profile = PublishProfile::new();

        // Create a target package from connection string
        let log = Logger::root(Discard.fuse(), o!());
        let target_package = Package::from_connection(&log, &$connection).unwrap();

        // Generate delta and apply
        let delta = Delta::generate(
            &log,
            &package,
            target_package,
            $db_name.into(),
            publish_profile,
        ).unwrap();
        delta.apply(&log, &$connection).unwrap();

        // Confirm db exists with data
        let final_package = Package::from_connection(&log, &$connection).unwrap().unwrap();
        // TODO: Would be nice to be able to do this, however we'd need to implement PartialEq manually
        //assert_that!(final_package).is_equal_to(&package);
        assert_simple_package!(final_package, $namespace);
    }};
}

#[test]
fn it_can_create_a_database_that_doesnt_exist() {
    const DB_NAME : &str = "psqlpack_new_db";
    const NAMESPACE : &str = "it_can_create_a_database_that_doesnt_exist";

    // Preliminary: remove existing database
    let connection = ConnectionBuilder::new(DB_NAME, "localhost", "postgres").build().unwrap();
    let mut conn = connection.connect_host().unwrap();
    drop_db!(conn, connection.database());
    conn.finish().unwrap();

    // Publish with basic assert
    publish_simple_package!(NAMESPACE, DB_NAME, connection);
}

#[test]
fn it_can_add_a_new_table_to_an_existing_database() {
    const DB_NAME : &str = "psqlpack_existing_db";
    const NAMESPACE : &str = "it_can_add_a_new_table_to_an_existing_database";

    // Preliminary: create a database with no tables
    let connection = ConnectionBuilder::new(DB_NAME, "localhost", "postgres").build().unwrap();
    let conn = create_db!(connection);
    drop_table!(conn, format!("{}.contacts", NAMESPACE));
    conn.finish().unwrap();

    // Publish with basic assert
    publish_simple_package!(NAMESPACE, DB_NAME, connection);
}

#[test]
fn it_can_add_a_new_column_to_an_existing_table() {
    const DB_NAME : &str = "psqlpack_existing_db";
    const NAMESPACE : &str = "it_can_add_a_new_column_to_an_existing_table";

    // Preliminary: create a database with a partial table
    let connection = ConnectionBuilder::new(DB_NAME, "localhost", "postgres").build().unwrap();
    let conn = create_db!(connection);
    drop_table!(conn, format!("{}.contacts", NAMESPACE));
    conn.batch_execute(&format!("CREATE SCHEMA IF NOT EXISTS {}", NAMESPACE)).unwrap();
    conn.batch_execute(&format!("CREATE TABLE {}.contacts (id serial PRIMARY KEY NOT NULL)", NAMESPACE)).unwrap();
    conn.finish().unwrap();

    // Publish with basic assert
    publish_simple_package!(NAMESPACE, DB_NAME, connection);
}

#[test]
fn it_can_modify_an_existing_column_on_a_table() {
    const DB_NAME : &str = "psqlpack_existing_db";
    const NAMESPACE : &str = "it_can_modify_an_existing_column_on_a_table";

    // Preliminary: create a database with a partial table
    let connection = ConnectionBuilder::new(DB_NAME, "localhost", "postgres").build().unwrap();
    let conn = create_db!(connection);
    drop_table!(conn, format!("{}.contacts", NAMESPACE));
    conn.batch_execute(&format!("CREATE SCHEMA IF NOT EXISTS {}", NAMESPACE)).unwrap();
    conn.batch_execute(&format!("CREATE TABLE {}.contacts (id serial PRIMARY KEY NOT NULL, name character varying(10) NULL)", NAMESPACE)).unwrap();
    conn.finish().unwrap();

    // Publish with basic assert
    publish_simple_package!(NAMESPACE, DB_NAME, connection);

    // SHOULD FAIL AT THE MOMENT
}

#[test]
fn it_can_drop_an_existing_column_on_a_table() {
    const DB_NAME : &str = "psqlpack_existing_db";
    const NAMESPACE : &str = "it_can_drop_an_existing_column_on_a_table";

    // Preliminary: create a database with a partial table
    let connection = ConnectionBuilder::new(DB_NAME, "localhost", "postgres").build().unwrap();
    let conn = create_db!(connection);
    drop_table!(conn, format!("{}.contacts", NAMESPACE));
    conn.batch_execute(&format!("CREATE SCHEMA IF NOT EXISTS {}", NAMESPACE)).unwrap();
    conn.batch_execute(&format!("CREATE TABLE {}.contacts (id serial PRIMARY KEY NOT NULL, name character varying(50) NOT NULL, last_name character varying(10))", NAMESPACE)).unwrap();
    conn.finish().unwrap();

    // Publish with basic assert
    publish_simple_package!(NAMESPACE, DB_NAME, connection);
}
