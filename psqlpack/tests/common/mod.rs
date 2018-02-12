macro_rules! drop_db {
    ($connection:expr, $database:expr) => {{
        $connection.query("SELECT pg_terminate_backend(pg_stat_activity.pid) \
                          FROM pg_stat_activity \
                          WHERE pg_stat_activity.datname = $1;", &[&$database]).unwrap();
        $connection.batch_execute(&format!("DROP DATABASE IF EXISTS {}", $database)).unwrap();
    }};
}

macro_rules! create_db {
    ($connection:expr) => {{
        let conn = $connection.connect_host().unwrap();
        let result = conn.query("SELECT 1 FROM pg_database WHERE datname=$1", &[&$connection.database()]).unwrap();
        if result.is_empty() {
            conn.batch_execute(&format!("CREATE DATABASE IF NOT EXIST {}", $connection.database())).unwrap();
        }
        conn.finish().unwrap();
        $connection.connect_database().unwrap()
    }};
}

macro_rules! drop_table {
    ($connection:expr, $name:expr) => {{
        let cmd = format!("DROP TABLE IF EXISTS {}", $name);
        $connection.batch_execute(&cmd).unwrap();
    }};
}

macro_rules! generate_simple_package {
    ($namespace:ident) => {{
        {
            let mut package = Package::new();
            package.push_schema(SchemaDefinition {
                name: $namespace.to_string(),
            });
            package.push_table(TableDefinition {
                name: ObjectName {
                    schema: Some($namespace.to_string()),
                    name: "contacts".into()
                },
                columns: vec![
                    ColumnDefinition {
                        name: "id".into(),
                        sql_type: SqlType::Simple(SimpleSqlType::Serial),
                        constraints: Some(vec![
                            ColumnConstraint::PrimaryKey,
                            ColumnConstraint::NotNull
                        ])
                    },
                    ColumnDefinition {
                        name: "name".into(),
                        sql_type: SqlType::Simple(SimpleSqlType::VariableLengthString(50)),
                        constraints: Some(vec![ColumnConstraint::NotNull])
                    },
                ],
                constraints: None,
            });
            package.set_defaults(&Project::default());
            package.validate().unwrap();
            package
        }
    }};
}

macro_rules! assert_simple_package {
    ($package:ident, $namespace:ident) => {{
        // We don't assert the length of collection's, only that our one's exist

        // Assert that the package exists in the expected format
        //assert_that!($package.schemas).has_length(2);
        assert!(&$package.schemas.iter().any(|s| s.name.eq($namespace)));
        assert!(&$package.schemas.iter().any(|s| s.name.eq("public")));

        // Validate the table
        //assert_that!($package.tables).has_length(1);
        let table = $package.tables.iter().find(|s| s.name.to_string().eq(&format!("{}.contacts", $namespace)));
        assert_that!(table).is_some();
        let table = table.unwrap();
        assert_that!(table.name.to_string()).is_equal_to(format!("{}.contacts", $namespace));
        assert_that!(table.columns).has_length(2);
        assert_that!(table.constraints).is_none();

        // Validate the id column
        let col_id = &table.columns[0];
        assert_that!(col_id.name).is_equal_to("id".to_string());
        assert_that!(col_id.sql_type).is_equal_to(SqlType::Simple(SimpleSqlType::Serial));
        assert_that!(col_id.constraints).is_some().has_length(2);
        match col_id.constraints {
            Some(ref constraints) => {
                let constraints = constraints.iter();
                assert_that!(constraints).contains(ColumnConstraint::PrimaryKey);
                assert_that!(constraints).contains(ColumnConstraint::NotNull);
            }
            None => panic!("Expected constraints to exist for contacts.id"),
        }

        // Validate the name column
        let col_name = &table.columns[1];
        assert_that!(col_name.name).is_equal_to("name".to_string());
        assert_that!(col_name.sql_type).is_equal_to(SqlType::Simple(SimpleSqlType::VariableLengthString(50)));
        assert_that!(col_name.constraints).is_some().has_length(1);
        match col_name.constraints {
            Some(ref constraints) => {
                let constraints = constraints.iter();
                assert_that!(constraints).contains(ColumnConstraint::NotNull);
            }
            None => panic!("Expected constraints to exist for contacts.name"),
        }
    }};
}
