use lalrpop_util::ParseError;

use lexer;
use connection::{ConnectionError, ConnectionErrorKind};

error_chain! {
    types {
        DacpacError, DacpacErrorKind, DacpacResultExt, DacpacResult;
    }
    links {
        Connection(ConnectionError, ConnectionErrorKind);
    }
    errors {
        IOError(file: String, message: String)
        SyntaxError(file: String, line: String, line_number: i32, start_pos: i32, end_pos: i32)
        ParseError(file: String, errors: Vec<ParseError<(), lexer::Token, ()>>)
        GenerationError(message: String)
        FormatError(file: String, message: String)
        DatabaseError(message: String)
        ProjectError(message: String)
        MultipleErrors(errors: Vec<DacpacErrorKind>)
    }
}
