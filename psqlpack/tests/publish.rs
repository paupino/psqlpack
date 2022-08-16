extern crate psqlpack;
#[macro_use]
extern crate slog;

#[macro_use]
mod common;

use psqlpack::ast::*;
use psqlpack::*;
use slog::{Discard, Drain, Logger};

macro_rules! publish_package {
    ($db_name:ident, $connection:ident, $package:ident) => {{
        // Use the default publish profile
        let mut publish_profile = PublishProfile::default();
        publish_profile.generation_options.drop_tables = Toggle::Ignore; // We reuse the same database
        publish_profile.generation_options.drop_columns = Toggle::Allow; // We allow this in some tests
        publish_profile.generation_options.drop_indexes = Toggle::Allow; // We allow this in some tests

        // Create a target package from connection string
        let log = Logger::root(Discard.fuse(), o!());
        let capabilities = Capabilities::from_connection(&log, &$connection).unwrap();
        let target_package = Package::from_connection(&log, &$connection, &capabilities).unwrap();

        // Generate delta and apply
        let delta = Delta::generate(
            &log,
            &$package,
            target_package,
            $db_name,
            &capabilities,
            &publish_profile,
        )
        .unwrap();
        delta.apply(&log, &$connection).unwrap();

        // Confirm db exists with data
        let capabilities = Capabilities::from_connection(&log, &$connection).unwrap();
        Package::from_connection(&log, &$connection, &capabilities)
            .unwrap()
            .unwrap()
    }};
}

#[test]
fn it_can_create_a_database_that_doesnt_exist() {
    const DB_NAME: &str = "psqlpack_new_db";
    const NAMESPACE: &str = "it_can_create_a_database_that_doesnt_exist";

    // Preliminary: remove existing database
    let connection = ConnectionBuilder::new(DB_NAME, "localhost", "postgres")
        .with_password(std::env::var("POSTGRES_PASSWORD").unwrap_or("".into()))
        .build()
        .unwrap();
    let mut client = connection.connect_host().unwrap();
    drop_db!(client, connection.database());

    // Publish with basic assert
    let package = generate_simple_package!(NAMESPACE);
    let final_package = publish_package!(DB_NAME, connection, package);
    assert_simple_package!(final_package, NAMESPACE);
}

#[test]
fn it_can_add_a_new_table_to_an_existing_database() {
    const DB_NAME: &str = "psqlpack_existing_db";
    const NAMESPACE: &str = "it_can_add_a_new_table_to_an_existing_database";

    // Preliminary: create a database with no tables
    let connection = ConnectionBuilder::new(DB_NAME, "localhost", "postgres")
        .with_password(std::env::var("POSTGRES_PASSWORD").unwrap_or("".into()))
        .build()
        .unwrap();
    dump_capabilities!(connection);
    let mut client = create_db!(connection);
    drop_table!(client, NAMESPACE, "contacts");

    // Publish with basic assert
    let package = generate_simple_package!(NAMESPACE);
    let final_package = publish_package!(DB_NAME, connection, package);
    assert_simple_package!(final_package, NAMESPACE);
}

#[test]
fn it_can_add_a_new_column_to_an_existing_table() {
    const DB_NAME: &str = "psqlpack_existing_db";
    const NAMESPACE: &str = "it_can_add_a_new_column_to_an_existing_table";

    // Preliminary: create a database with a partial table
    let connection = ConnectionBuilder::new(DB_NAME, "localhost", "postgres")
        .with_password(std::env::var("POSTGRES_PASSWORD").unwrap_or("".into()))
        .build()
        .unwrap();
    dump_capabilities!(connection);
    let mut client = create_db!(connection);
    drop_table!(client, NAMESPACE, "contacts");
    client
        .batch_execute(&format!("CREATE SCHEMA IF NOT EXISTS {}", NAMESPACE))
        .unwrap();
    client
        .batch_execute(&format!(
            "CREATE TABLE {}.contacts (id serial PRIMARY KEY NOT NULL)",
            NAMESPACE
        ))
        .unwrap();

    // Publish with basic assert
    let package = generate_simple_package!(NAMESPACE);
    let final_package = publish_package!(DB_NAME, connection, package);
    assert_simple_package!(final_package, NAMESPACE);
}

#[test]
fn it_can_modify_an_existing_column_on_a_table() {
    const DB_NAME: &str = "psqlpack_existing_db";
    const NAMESPACE: &str = "it_can_modify_an_existing_column_on_a_table";

    // Preliminary: create a database with a partial table
    let connection = ConnectionBuilder::new(DB_NAME, "localhost", "postgres")
        .with_password(std::env::var("POSTGRES_PASSWORD").unwrap_or("".into()))
        .build()
        .unwrap();
    dump_capabilities!(connection);
    let mut client = create_db!(connection);
    drop_table!(client, NAMESPACE, "contacts");
    client
        .batch_execute(&format!("CREATE SCHEMA IF NOT EXISTS {}", NAMESPACE))
        .unwrap();
    client
        .batch_execute(&format!(
            "CREATE TABLE {}.contacts (id serial PRIMARY KEY NOT NULL, name character varying(10) NULL)",
            NAMESPACE
        ))
        .unwrap();

    // Publish with basic assert
    let package = generate_simple_package!(NAMESPACE);
    let final_package = publish_package!(DB_NAME, connection, package);
    assert_simple_package!(final_package, NAMESPACE);
}

#[test]
fn it_can_drop_an_existing_column_on_a_table() {
    const DB_NAME: &str = "psqlpack_existing_db";
    const NAMESPACE: &str = "it_can_drop_an_existing_column_on_a_table";

    // Preliminary: create a database with a partial table
    let connection = ConnectionBuilder::new(DB_NAME, "localhost", "postgres")
        .with_password(std::env::var("POSTGRES_PASSWORD").unwrap_or("".into()))
        .build()
        .unwrap();
    dump_capabilities!(connection);
    let mut client = create_db!(connection);
    drop_table!(client, NAMESPACE, "contacts");
    client
        .batch_execute(&format!("CREATE SCHEMA IF NOT EXISTS {}", NAMESPACE))
        .unwrap();
    client.batch_execute(&format!("CREATE TABLE {}.contacts (id serial PRIMARY KEY NOT NULL, name character varying(50) NOT NULL, last_name character varying(10))", NAMESPACE)).unwrap();

    // Publish with basic assert
    let package = generate_simple_package!(NAMESPACE);
    let final_package = publish_package!(DB_NAME, connection, package);
    assert_simple_package!(final_package, NAMESPACE);
}

#[test]
fn it_can_add_a_new_index_to_an_existing_table() {
    const DB_NAME: &str = "psqlpack_existing_db";
    const NAMESPACE: &str = "it_can_add_a_new_index_to_an_existing_table";

    // Preliminary: create a database with a table but no indexes
    let connection = ConnectionBuilder::new(DB_NAME, "localhost", "postgres")
        .with_password(std::env::var("POSTGRES_PASSWORD").unwrap_or("".into()))
        .build()
        .unwrap();
    dump_capabilities!(connection);
    let mut client = create_db!(connection);
    drop_table!(client, NAMESPACE, "contacts");
    client
        .batch_execute(&format!("CREATE SCHEMA IF NOT EXISTS {}", NAMESPACE))
        .unwrap();
    client
        .batch_execute(&format!(
            "CREATE TABLE {}.contacts (id serial PRIMARY KEY NOT NULL, name character varying(50) NULL)",
            NAMESPACE
        ))
        .unwrap();

    // Publish with basic assert
    let package = generate_simple_package!(NAMESPACE);
    let final_package = publish_package!(DB_NAME, connection, package);
    assert_simple_package!(final_package, NAMESPACE);
}

#[test]
fn it_can_modify_an_index_on_a_table() {
    const DB_NAME: &str = "psqlpack_existing_db";
    const NAMESPACE: &str = "it_can_modify_an_index_on_a_table";

    // Preliminary: create a database with a table but a broad index
    let connection = ConnectionBuilder::new(DB_NAME, "localhost", "postgres")
        .with_password(std::env::var("POSTGRES_PASSWORD").unwrap_or("".into()))
        .build()
        .unwrap();
    dump_capabilities!(connection);
    let mut client = create_db!(connection);
    drop_table!(client, NAMESPACE, "contacts");
    client
        .batch_execute(&format!("CREATE SCHEMA IF NOT EXISTS {}", NAMESPACE))
        .unwrap();
    client.batch_execute(&format!("CREATE TABLE {}.contacts (id serial PRIMARY KEY NOT NULL, name character varying(50) NULL, name2 character varying(50) NULL)", NAMESPACE)).unwrap();
    client
        .batch_execute(&format!(
            "CREATE INDEX idx_contacts_name ON {}.contacts (name, name2)",
            NAMESPACE
        ))
        .unwrap();

    // Publish with basic assert
    let package = generate_simple_package!(NAMESPACE);
    let final_package = publish_package!(DB_NAME, connection, package);
    assert_simple_package!(final_package, NAMESPACE);
}

#[test]
fn it_can_drop_an_index_on_a_table() {
    const DB_NAME: &str = "psqlpack_existing_db";
    const NAMESPACE: &str = "it_can_drop_an_index_on_a_table";

    // Preliminary: create a database with a table and extra index
    let connection = ConnectionBuilder::new(DB_NAME, "localhost", "postgres")
        .with_password(std::env::var("POSTGRES_PASSWORD").unwrap_or("".into()))
        .build()
        .unwrap();
    dump_capabilities!(connection);
    let mut client = create_db!(connection);
    drop_table!(client, NAMESPACE, "contacts");
    client
        .batch_execute(&format!("CREATE SCHEMA IF NOT EXISTS {}", NAMESPACE))
        .unwrap();
    client.batch_execute(&format!("CREATE TABLE {}.contacts (id serial PRIMARY KEY NOT NULL, name character varying(50) NOT NULL, name2 character varying(50) NULL)", NAMESPACE)).unwrap();
    client
        .batch_execute(&format!(
            "CREATE INDEX idx_contacts_name ON {}.contacts (name)",
            NAMESPACE
        ))
        .unwrap();
    client
        .batch_execute(&format!(
            "CREATE INDEX idx_contacts_name_2 ON {}.contacts (name2)",
            NAMESPACE
        ))
        .unwrap();

    // Publish with basic assert
    let package = generate_simple_package!(NAMESPACE);
    let final_package = publish_package!(DB_NAME, connection, package);
    assert_simple_package!(final_package, NAMESPACE);
}
