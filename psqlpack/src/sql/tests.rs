use crate::sql::ast::*;
use crate::sql::lexer;
use crate::sql::parser::{FunctionArgumentListParser, StatementListParser};

use spectral::prelude::*;

#[test]
fn it_can_parse_a_basic_function_definition() {
    let sql = "CREATE OR REPLACE FUNCTION index(index int)
               RETURNS int
               AS $body$
                   SELECT index
               $body$
               LANGUAGE SQL;";

    let tokens = lexer::tokenize_stmt(sql);
    assert_that!(tokens).is_ok();
    let tokens = tokens.unwrap();

    let statements = StatementListParser::new().parse(tokens);
    assert_that!(statements).is_ok();
    let statements = statements.unwrap();
    assert_that!(statements).has_length(1);
    let stmt = &statements[0];

    assert_that!(*stmt).is_equal_to(Statement::Function(FunctionDefinition {
        name: ObjectName {
            schema: None,
            name: "index".into(),
        },
        arguments: vec![FunctionArgument {
            mode: None,
            name: Some("index".into()),
            sql_type: SqlType::Simple(SimpleSqlType::Integer, None),
            default: None,
        }],
        return_type: FunctionReturnType::SqlType(SqlType::Simple(SimpleSqlType::Integer, None)),
        body: "SELECT index".into(),
        language: FunctionLanguage::SQL,
    }));
}

#[test]
fn it_can_parse_a_function_definition_with_simple_literals() {
    let sql = "CREATE OR REPLACE FUNCTION public.x()
               RETURNS int
               AS $$
                   SELECT 1
               $$
               LANGUAGE SQL;";

    let tokens = lexer::tokenize_stmt(sql);
    assert_that!(tokens).is_ok();
    let tokens = tokens.unwrap();

    let statements = StatementListParser::new().parse(tokens);
    assert_that!(statements).is_ok();
    let statements = statements.unwrap();
    assert_that!(statements).has_length(1);
    let stmt = &statements[0];

    assert_that!(*stmt).is_equal_to(Statement::Function(FunctionDefinition {
        name: ObjectName {
            schema: Some("public".into()),
            name: "x".into(),
        },
        arguments: Vec::new(),
        return_type: FunctionReturnType::SqlType(SqlType::Simple(SimpleSqlType::Integer, None)),
        body: "SELECT 1".into(),
        language: FunctionLanguage::SQL,
    }));
}

#[test]
fn it_can_parse_a_function_definition_returning_table() {
    let sql = "CREATE OR REPLACE FUNCTION reference_data.fn_countries()
               RETURNS TABLE (
                   name character varying(80),
                   iso character varying(2)
               )
               AS $$
                   SELECT countries.name, countries.iso
                   FROM reference_data.countries
                   WHERE countries.enabled=true
                   ORDER BY countries.iso
               $$
               LANGUAGE SQL;";

    let tokens = lexer::tokenize_stmt(sql);
    assert_that!(tokens).is_ok();
    let tokens = tokens.unwrap();

    let statements = StatementListParser::new().parse(tokens);
    assert_that!(statements).is_ok();
    let statements = statements.unwrap();
    assert_that!(statements).has_length(1);
    let stmt = &statements[0];

    assert_that!(*stmt).is_equal_to(Statement::Function(FunctionDefinition {
        name: ObjectName {
            schema: Some("reference_data".into()),
            name: "fn_countries".into(),
        },
        arguments: Vec::new(),
        return_type: FunctionReturnType::Table(vec![
            ColumnDefinition {
                name: "name".into(),
                sql_type: SqlType::Simple(SimpleSqlType::VariableLengthString(80), None),
                constraints: Vec::new(),
            },
            ColumnDefinition {
                name: "iso".into(),
                sql_type: SqlType::Simple(SimpleSqlType::VariableLengthString(2), None),
                constraints: Vec::new(),
            },
        ]),
        body: "SELECT countries.name, countries.iso
                   FROM reference_data.countries
                   WHERE countries.enabled=true
                   ORDER BY countries.iso"
            .into(),
        language: FunctionLanguage::SQL,
    }));
}

#[test]
fn it_can_parse_a_function_definition_with_parameters() {
    let sql = "CREATE OR REPLACE FUNCTION reference_data.fn_states(country character varying(2))
               RETURNS TABLE (
                   name character varying(80),
                   iso character varying(10)
               )
               AS $$
                   SELECT states.name, states.iso
                   FROM reference_data.states
                   INNER JOIN reference_data.countries ON countries.id=states.country_id
                   WHERE countries.iso = $1 AND countries.enabled=true AND states.enabled=true
                   ORDER BY states.iso
               $$
               LANGUAGE SQL;";

    let tokens = lexer::tokenize_stmt(sql);
    assert_that!(tokens).is_ok();
    let tokens = tokens.unwrap();

    let statements = StatementListParser::new().parse(tokens);
    assert_that!(statements).is_ok();
    let statements = statements.unwrap();
    assert_that!(statements).has_length(1);
    let stmt = &statements[0];

    assert_that!(*stmt).is_equal_to(Statement::Function(FunctionDefinition {
        name: ObjectName {
            schema: Some("reference_data".into()),
            name: "fn_states".into(),
        },
        arguments: vec![FunctionArgument {
            mode: None,
            name: Some("country".into()),
            sql_type: SqlType::Simple(SimpleSqlType::VariableLengthString(2), None),
            default: None,
        }],
        return_type: FunctionReturnType::Table(vec![
            ColumnDefinition {
                name: "name".into(),
                sql_type: SqlType::Simple(SimpleSqlType::VariableLengthString(80), None),
                constraints: Vec::new(),
            },
            ColumnDefinition {
                name: "iso".into(),
                sql_type: SqlType::Simple(SimpleSqlType::VariableLengthString(10), None),
                constraints: Vec::new(),
            },
        ]),
        body: "SELECT states.name, states.iso
                   FROM reference_data.states
                   INNER JOIN reference_data.countries ON countries.id=states.country_id
                   WHERE countries.iso = $1 AND countries.enabled=true AND states.enabled=true
                   ORDER BY states.iso"
            .into(),
        language: FunctionLanguage::SQL,
    }));
}

#[test]
fn it_can_parse_function_arguments() {
    let sql = "geom geometry,
               scalex double precision,
               scaley double precision,
               gridx double precision DEFAULT NULL::double precision,
               gridy double precision DEFAULT NULL::double precision,
               pixeltype text[] DEFAULT ARRAY['8BUI'::text],
               value double precision[] DEFAULT ARRAY[(1)::double precision],
               nodataval double precision[] DEFAULT ARRAY[(0)::double precision],
               skewx double precision DEFAULT 0,
               skewy double precision DEFAULT 0,
               touched boolean DEFAULT false";

    let tokens = lexer::tokenize_body(sql);
    assert_that!(tokens).is_ok();
    let tokens = tokens.unwrap();

    let arguments = FunctionArgumentListParser::new().parse(tokens);
    assert_that!(arguments).is_ok();
    let arguments = arguments.unwrap();
    assert_that!(arguments).has_length(11);
    assert_that!(arguments).is_equal_to(vec![
        FunctionArgument {
            mode: None,
            name: Some("geom".into()),
            sql_type: SqlType::Custom(
                ObjectName {
                    schema: None,
                    name: "geometry".to_string(),
                },
                vec![],
                None,
            ),
            default: None,
        },
        FunctionArgument {
            mode: None,
            name: Some("scalex".into()),
            sql_type: SqlType::Simple(SimpleSqlType::Double, None),
            default: None,
        },
        FunctionArgument {
            mode: None,
            name: Some("scaley".into()),
            sql_type: SqlType::Simple(SimpleSqlType::Double, None),
            default: None,
        },
        FunctionArgument {
            mode: None,
            name: Some("gridx".into()),
            sql_type: SqlType::Simple(SimpleSqlType::Double, None),
            default: Some(AnyValue::Null(Some(SqlType::Simple(SimpleSqlType::Double, None)))),
        },
        FunctionArgument {
            mode: None,
            name: Some("gridy".into()),
            sql_type: SqlType::Simple(SimpleSqlType::Double, None),
            default: Some(AnyValue::Null(Some(SqlType::Simple(SimpleSqlType::Double, None)))),
        },
        FunctionArgument {
            mode: None,
            name: Some("pixeltype".into()),
            sql_type: SqlType::Simple(SimpleSqlType::Text, Some(1)),
            default: Some(AnyValue::Array(
                vec![AnyValue::String(
                    "8BUI".into(),
                    Some(SqlType::Simple(SimpleSqlType::Text, None)),
                )],
                None,
            )),
        },
        FunctionArgument {
            mode: None,
            name: Some("value".into()),
            sql_type: SqlType::Simple(SimpleSqlType::Double, Some(1)),
            default: Some(AnyValue::Array(
                vec![AnyValue::Integer(1, Some(SqlType::Simple(SimpleSqlType::Double, None)))],
                None,
            )),
        },
        FunctionArgument {
            mode: None,
            name: Some("nodataval".into()),
            sql_type: SqlType::Simple(SimpleSqlType::Double, Some(1)),
            default: Some(AnyValue::Array(
                vec![AnyValue::Integer(0, Some(SqlType::Simple(SimpleSqlType::Double, None)))],
                None,
            )),
        },
        FunctionArgument {
            mode: None,
            name: Some("skewx".into()),
            sql_type: SqlType::Simple(SimpleSqlType::Double, None),
            default: Some(AnyValue::Integer(0, None)),
        },
        FunctionArgument {
            mode: None,
            name: Some("skewy".into()),
            sql_type: SqlType::Simple(SimpleSqlType::Double, None),
            default: Some(AnyValue::Integer(0, None)),
        },
        FunctionArgument {
            mode: None,
            name: Some("touched".into()),
            sql_type: SqlType::Simple(SimpleSqlType::Boolean, None),
            default: Some(AnyValue::Boolean(false, None)),
        },
    ]);
}
