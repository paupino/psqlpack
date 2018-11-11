macro_rules! drop_db {
    ($connection:expr, $database:expr) => {{
        $connection.query("SELECT pg_terminate_backend(pg_stat_activity.pid) \
                          FROM pg_stat_activity \
                          WHERE pg_stat_activity.datname = $1;", &[&$database]).unwrap();
        $connection.batch_execute(&format!("DROP DATABASE IF EXISTS {}", $database)).unwrap();
    }};
}

macro_rules! dump_capabilities {
    ($connection: expr) => {{
        let log = Logger::root(Discard.fuse(), o!());
        let capabilities = Capabilities::from_connection(&log, &$connection).unwrap();
        println!("PG Version: {}", capabilities.server_version);
    }};
}

macro_rules! create_db {
    ($connection:expr) => {{
        let conn = $connection.connect_host().unwrap();
        let result = conn.query("SELECT 1 FROM pg_database WHERE datname=$1", &[&$connection.database()]).unwrap();
        if result.is_empty() {
            conn.batch_execute(&format!("CREATE DATABASE {}", $connection.database())).unwrap();
        }
        conn.finish().unwrap();
        $connection.connect_database().unwrap()
    }};
}

macro_rules! drop_table {
    ($connection:expr, $schema:expr, $name:expr) => {{
        let result = $connection.query("SELECT 1 FROM pg_namespace WHERE nspname = $1", &[&$schema]).unwrap();
        if !result.is_empty() {
            let cmd = format!("DROP TABLE IF EXISTS {}.{}", $schema, $name);
            $connection.batch_execute(&cmd).unwrap();
        }
    }};
}

macro_rules! generate_simple_package {
    ($namespace:ident) => {{
        {
            let mut package = Package::new();
            package.push_schema(SchemaDefinition {
                name: $namespace.to_string(),
            });
            let table_name = ObjectName {
                schema: Some($namespace.to_string()),
                name: "contacts".to_string(),
            };
            package.push_table(TableDefinition {
                name: table_name.clone(),
                columns: vec![
                    ColumnDefinition {
                        name: "id".into(),
                        sql_type: SqlType::Simple(SimpleSqlType::Serial),
                        constraints: vec![
                            ColumnConstraint::PrimaryKey,
                            ColumnConstraint::NotNull
                        ]
                    },
                    ColumnDefinition {
                        name: "name".into(),
                        sql_type: SqlType::Simple(SimpleSqlType::VariableLengthString(50)),
                        constraints: vec![ColumnConstraint::NotNull]
                    },
                ],
                constraints: Vec::new(),
            });
            package.push_index(IndexDefinition {
                name: "idx_contacts_name".to_owned(),
                table: table_name,
                columns: vec![
                    IndexColumn {
                        name: "name".to_owned(),
                        order: None,
                        null_position: None,
                    },
                ],
                unique: false,
                index_type: None,
                storage_parameters: None,
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
        assert_that!(table.columns).named(&"table.columns").has_length(2);
        assert_that!(table.constraints).named(&"table.constraints").has_length(1);

        // Validate the id column
        let col_id = &table.columns[0];
        assert_that!(col_id.name).is_equal_to("id".to_string());
        assert_that!(col_id.sql_type).is_equal_to(SqlType::Simple(SimpleSqlType::Serial));
        assert_that!(col_id.constraints).named(&"col_id.constraints").has_length(1);
        let constraints = col_id.constraints.iter();
        assert_that!(constraints).contains(ColumnConstraint::NotNull);

        // Validate the name column
        let col_name = &table.columns[1];
        assert_that!(col_name.name).is_equal_to("name".to_string());
        assert_that!(col_name.sql_type).is_equal_to(SqlType::Simple(SimpleSqlType::VariableLengthString(50)));
        assert_that!(col_name.constraints).named(&"col_name.constraints").has_length(1);
        assert_that!(col_name.constraints[0]).is_equal_to(ColumnConstraint::NotNull);

        // We can't assert indexes since we share a database but separate by schema.
        // To get around this we'll filter by schema first.
        let schema_indexes : Vec<&IndexDefinition> = $package.indexes
                .iter()
                .filter(|ref i|
                    if let Some(ref schema) = i.table.schema {
                        schema.eq($namespace)
                    } else {
                        false
                    }
                )
                .collect();
        assert_that!(schema_indexes).named("package.indexes").has_length(1);
        let index = &schema_indexes[0];
        assert_that!(index.name).is_equal_to("idx_contacts_name".to_string());
        assert_that!(index.table.to_string()).is_equal_to(format!("{}.contacts", $namespace));
        assert_that!(index.unique).is_false();
        assert_that!(index.index_type).is_some().is_equal_to(IndexType::BTree);
        assert_that!(index.storage_parameters).is_none();
        assert_that!(index.columns).named("index.columns").has_length(1);
        let index_col = &index.columns[0];
        assert_that!(index_col.name).is_equal_to("name".to_string());
        assert_that!(index_col.order).is_some().is_equal_to(IndexOrder::Ascending);
        assert_that!(index_col.null_position).is_some().is_equal_to(IndexPosition::Last);
    }};
}
