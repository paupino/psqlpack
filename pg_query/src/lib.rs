mod pg_query {
    include!(concat!(env!("OUT_DIR"), "/pg_query/mod.rs"));

    use pg_query::*;
}

pub fn parse(stmt: &str) {}

#[cfg(test)]
mod tests {
    use crate::pg_query::ParseResult;

    #[test]
    fn it_works() {
        let mut result = ParseResult::new();
        result.set_stmts("CREATE INDEX ix_test ON contacts.person (id, ssn) WHERE ssn IS NOT NULL;");
        // println!(
        //     "{:?}",
        //     pg_query::parse("CREATE INDEX ix_test ON contacts.person (id, ssn) WHERE ssn IS NOT NULL;").unwrap()
        // );
        assert!(false);
    }
}
