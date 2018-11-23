use sql::ast::*;
use sql::lexer;
use sql::parser::StatementListParser;

use spectral::prelude::*;

#[test]
fn it_can_parse_basic_function_definition() {
    let sql = "CREATE OR REPLACE FUNCTION index(index int)
               RETURNS int
               AS $body$
                   SELECT index
               $body$
               LANGUAGE SQL;";

    let tokens = lexer::tokenize(sql);
    assert_that!(tokens).is_ok();
    let tokens = tokens.unwrap();

    let statements = StatementListParser::new().parse(tokens);
    assert_that!(statements).is_ok();
    let statements = statements.unwrap();
    assert_that!(statements).has_length(1);
    let stmt = &statements[0];

    assert_that!(*stmt).is_equal_to(
        Statement::Function(FunctionDefinition {
            name: ObjectName { schema: None, name: "index".into() },
            arguments: vec![
                FunctionArgument {
                    mode: None,
                    name: Some("index".into()),
                    sql_type: SqlType::Simple(SimpleSqlType::Integer, None),
                    default: None,
                },
            ],
            return_type: FunctionReturnType::SqlType(SqlType::Simple(SimpleSqlType::Integer, None)),
            body: "SELECT index".into(),
            language: FunctionLanguage::SQL,
        }),
    );
}

#[test]
fn it_can_parse_function_definition_with_simple_literals() {
    let sql = "CREATE OR REPLACE FUNCTION public.x()
               RETURNS int
               AS $$
                   SELECT 1
               $$
               LANGUAGE SQL;";

    let tokens = lexer::tokenize(sql);
    assert_that!(tokens).is_ok();
    let tokens = tokens.unwrap();

    let statements = StatementListParser::new().parse(tokens);
    assert_that!(statements).is_ok();
    let statements = statements.unwrap();
    assert_that!(statements).has_length(1);
    let stmt = &statements[0];

    assert_that!(*stmt).is_equal_to(
        Statement::Function(FunctionDefinition {
            name: ObjectName { schema: Some("public".into()), name: "x".into() },
            arguments: Vec::new(),
            return_type: FunctionReturnType::SqlType(SqlType::Simple(SimpleSqlType::Integer, None)),
            body: "SELECT 1".into(),
            language: FunctionLanguage::SQL,
        }),
    );
}
