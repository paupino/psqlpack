pub use error_chain::ChainedError;
pub use lalrpop_util::ParseError;

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
        IOError(file: String, message: String) {
            description("IO error when reading a file")
            display("IO error when reading {}: {}", file, message)
        }
        SyntaxError(file: String, line: String, line_number: i32, start_pos: i32, end_pos: i32) {
            description("SQL syntax error encountered")
            display(
                "SQL syntax error encountered in {} on line {}:\n  {}\n  {}{}",
                file, line_number, line, " ".repeat(*start_pos as usize), "^".repeat((end_pos - start_pos) as usize))
        }
        ParseError(file: String, errors: Vec<ParseError<(), lexer::Token, ()>>) {
            description("Parser error")
            display("Parser errors in {}:\n{}", file, ParseErrorFormatter(errors))
        }
        GenerationError(message: String) {
            description("Error generating DACPAC")
            display("Error generating DACPAC: {}", message)
        }
        FormatError(file: String, message: String) {
            description("Format error when reading a file")
            display("Format error when reading {}: {}", file, message)
        }
        DatabaseError(message: String) {
            description("Database error")
            display("Database error: {}", message)
        }
        ProjectError(message: String) {
            description("Project format error")
            display("Project format error: {}", message)
        }
        MultipleErrors(errors: Vec<DacpacError>) {
            description("Multiple errors")
            display("Multiple errors:\n{}", MultipleErrorFormatter(errors))
        }
    }
}

use std::fmt::{Display, Formatter, Result};

struct ParseErrorFormatter<'fmt>(&'fmt Vec<ParseError<(), lexer::Token, ()>>);

impl<'fmt> Display for ParseErrorFormatter<'fmt> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        for (i, error) in self.0.iter().enumerate() {
            write!(f, "{}: ", i)?;
            match *error {
                ParseError::InvalidToken { .. } => {
                    write!(f, "Invalid token")?
                }
                ParseError::UnrecognizedToken {
                    ref token,
                    ref expected,
                } => {
                    match *token {
                        Some(ref x) => writeln!(f, "Unexpected {:?}", x.1),
                        _ => writeln!(f, "Unexpected end of file"),
                    }?;
                    write!(f, "   Expected one of:\n   {}", expected.join(", "))?
                }
                ParseError::ExtraToken { ref token } => {
                    write!(f, "Extra token detected: {:?}", token)?
                }
                ParseError::User { ref error } => {
                    write!(f, "{:?}", error)?
                }
            }
        }
        Ok(())
    }
}

struct MultipleErrorFormatter<'fmt>(&'fmt Vec<DacpacError>);

impl<'fmt> Display for MultipleErrorFormatter<'fmt> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        for (i, error) in self.0.iter().enumerate() {
            write!(f, "--- Error {} ---\n{}", i, error.display())?;
        }
        Ok(())
    }
}
