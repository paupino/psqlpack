macro_rules! drop_db {
    ($connection:expr) => {{
        let connection = $connection.connect_host().unwrap();
        connection.query("SELECT pg_terminate_backend(pg_stat_activity.pid) \
                          FROM pg_stat_activity \
                          WHERE pg_stat_activity.datname = $1;", &[&$connection.database()]).unwrap();
        connection.query(&format!("DROP DATABASE IF EXISTS {}", $connection.database()), &[]).unwrap();
        connection.finish().unwrap();
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
            package.validate().unwrap();
            package
        }
    }};
}

macro_rules! assert_simple_package {
    ($package:ident, $namespace:ident) => {{
        // Assert that the package exists in the expected format
        assert_that!($package.schemas).has_length(1);
        let schema = &$package.schemas[0];
        assert_that!(schema.name).is_equal_to($namespace.to_string());

        // Validate the table
        assert_that!($package.tables).has_length(1);
        let table = &$package.tables[0];
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