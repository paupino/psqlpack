macro_rules! drop_db {
    ($connection:expr, $database:expr) => {{
        $connection
            .query(
                "SELECT pg_terminate_backend(pg_stat_activity.pid) \
                 FROM pg_stat_activity \
                 WHERE pg_stat_activity.datname = $1;",
                &[&$database],
            )
            .unwrap();
        $connection
            .batch_execute(&format!("DROP DATABASE IF EXISTS {}", $database))
            .unwrap();
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
        let mut client = $connection.connect_host().unwrap();
        let result = client
            .query("SELECT 1 FROM pg_database WHERE datname=$1", &[&$connection.database()])
            .unwrap();
        if result.is_empty() {
            client
                .batch_execute(&format!("CREATE DATABASE {}", $connection.database()))
                .unwrap();
        }
        $connection.connect_database().unwrap()
    }};
}

macro_rules! drop_table {
    ($connection:expr, $schema:expr, $name:expr) => {{
        let result = $connection
            .query("SELECT 1 FROM pg_namespace WHERE nspname = $1", &[&$schema])
            .unwrap();
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
                        sql_type: SqlType::Simple(SimpleSqlType::Serial, None),
                        constraints: vec![ColumnConstraint::PrimaryKey, ColumnConstraint::NotNull],
                    },
                    ColumnDefinition {
                        name: "name".into(),
                        sql_type: SqlType::Simple(SimpleSqlType::VariableLengthString(50), None),
                        constraints: vec![ColumnConstraint::NotNull],
                    },
                ],
                constraints: Vec::new(),
            });
            package.push_index(IndexDefinition {
                name: "idx_contacts_name".to_owned(),
                table: table_name,
                columns: vec![IndexColumn {
                    name: "name".to_owned(),
                    order: None,
                    null_position: None,
                }],
                unique: false,
                index_type: None,
                storage_parameters: None,
            });
            package.set_defaults(&Project::default());
            package.validate(&Vec::new()).unwrap();
            package
        }
    }};
}

macro_rules! assert_simple_package {
    ($package:ident, $namespace:ident) => {{
        // We don't assert the length of collection's, only that our one's exist

        // Assert that the package exists in the expected format
        assert!(&$package.schemas.iter().any(|s| s.name.eq($namespace)));
        assert!(&$package.schemas.iter().any(|s| s.name.eq("public")));

        // Validate the table
        let table = $package
            .tables
            .iter()
            .find(|s| s.name.to_string().eq(&format!("{}.contacts", $namespace)));
        assert!(table.is_some());
        let table = table.unwrap();
        assert_eq!(table.name.to_string(), format!("{}.contacts", $namespace));
        assert_eq!(table.columns.len(), 2);
        assert_eq!(table.constraints.len(), 1);

        // Validate the id column
        let col_id = &table.columns[0];
        assert_eq!(col_id.name, "id");
        assert_eq!(col_id.sql_type, SqlType::Simple(SimpleSqlType::Serial, None));
        assert_eq!(col_id.constraints.len(), 1);
        assert!(col_id.constraints.contains(&ColumnConstraint::NotNull));

        // Validate the name column
        let col_name = &table.columns[1];
        assert_eq!(col_name.name, "name");
        assert_eq!(
            col_name.sql_type,
            SqlType::Simple(SimpleSqlType::VariableLengthString(50), None)
        );
        assert_eq!(col_name.constraints.len(), 1);
        assert_eq!(col_name.constraints[0], ColumnConstraint::NotNull);

        // We can't assert indexes since we share a database but separate by schema.
        // To get around this we'll filter by schema first.
        let schema_indexes: Vec<&IndexDefinition> = $package
            .indexes
            .iter()
            .filter(|ref i| {
                if let Some(ref schema) = i.table.schema {
                    schema.eq($namespace)
                } else {
                    false
                }
            })
            .collect();
        assert_eq!(schema_indexes.len(), 1);
        let index = &schema_indexes[0];
        assert_eq!(index.name, "idx_contacts_name");
        assert_eq!(index.table.to_string(), format!("{}.contacts", $namespace));
        assert!(!index.unique);
        assert!(index.index_type.is_some());
        assert_eq!(index.index_type.as_ref().unwrap(), &IndexType::BTree);
        assert!(index.storage_parameters.is_none());
        assert_eq!(index.columns.len(), 1);
        let index_col = &index.columns[0];
        assert_eq!(index_col.name, "name");
        assert!(index_col.order.is_some());
        assert_eq!(index_col.order.as_ref().unwrap(), &IndexOrder::Ascending);
        assert!(index_col.null_position.is_some());
        assert_eq!(index_col.null_position.as_ref().unwrap(), &IndexPosition::Last);
    }};
}
