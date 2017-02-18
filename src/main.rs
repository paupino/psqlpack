mod ast;
mod sql;

fn main() {
    match sql::parse_statement_list("CREATE TABLE tax_table") {
        Ok(_) => println!("OK"),
        Err(e) => println!("Error: {:?}", e),
    }
}
