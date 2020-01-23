use glob::PatternError;
use std::fmt::{Display, Formatter, Result};
use std::path::PathBuf;

pub use error_chain::ChainedError;
pub use lalrpop_util::ParseError;

pub use crate::ast::ErrorKind;
use crate::connection::{ConnectionError, ConnectionErrorKind};
pub use crate::model::ValidationKind;
use crate::sql::lexer;

error_chain! {
    types {
        PsqlpackError, PsqlpackErrorKind, PsqlpackResultExt, PsqlpackResult;
    }
    links {
        Connection(ConnectionError, ConnectionErrorKind);
    }
    errors {
        ProjectReadError(path: PathBuf) {
            description("Couldn't read project file")
            display("Couldn't read project file: {}", path.as_path().display())
        }
        ProjectParseError(path: PathBuf) {
            description("Couldn't parse project file")
            display("Couldn't parse project file: {}", path.as_path().display())
        }
        InvalidScriptPath(path: String) {
            description("Invalid script path in project file")
            display("Invalid script path in project file: {}", path)
        }
        PublishProfileReadError(path: PathBuf) {
            description("Couldn't read publish profile file")
            display("Couldn't read publish profile file: {}", path.as_path().display())
        }
        PublishProfileParseError(path: PathBuf) {
            description("Couldn't parse publish profile file")
            display("Couldn't parse publish profile file: {}", path.as_path().display())
        }
        PackageCreationError(message: String) {
            description("Failed to create package")
            display("Failed to create package: {}", message)
        }
        PackageReadError(path: PathBuf) {
            description("Couldn't read package file")
            display("Couldn't read package file: {}", path.as_path().display())
        }
        PackageUnarchiveError(path: PathBuf) {
            description("Couldn't unarchive package file")
            display("Couldn't unarchive package file: {}", path.as_path().display())
        }
        PackageInternalReadError(file_name: String) {
            description("Couldn't read part of the package file")
            display("Couldn't read part of the package file: {}", file_name)
        }
        QueryExtensionsError {
            description("Couldn't query extensions")
        }
        PackageQuerySchemasError {
            description("Couldn't query schemas")
        }
        PackageQueryTypesError {
            description("Couldn't query types")
        }
        PackageQueryFunctionsError {
            description("Couldn't query functions")
        }
        PackageQueryTablesError {
            description("Couldn't query tables")
        }
        PackageQueryColumnsError {
            description("Couldn't query columns")
        }
        PackageQueryTableConstraintsError {
            description("Couldn't query table constraints")
        }
        PackageQueryIndexesError {
            description("Couldn't query indexes")
        }
        PackageFunctionArgsInspectError(args: String) {
            description("Couldn't inspect function args")
            display("Couldn't inspect function args: {}", args)
        }
        PackageFunctionReturnTypeInspectError(return_type: String) {
            description("Couldn't inspect function return type")
            display("Couldn't inspect function return type: {}", return_type)
        }
        PublishInvalidOperationError(message: String) {
            description("Couldn't publish database due to an invalid operation")
            display("Couldn't publish database due to an invalid operation: {}", message)
        }
        PublishUnsafeOperationError(message: String) {
            description("Unsafe Operation")
            display("Couldn't publish database due to an unsafe operation: {}", message)
        }
        GlobPatternError(err: PatternError) {
            description("An error in the glob pattern was found")
            display("An error in the glob pattern was found: {}", err)
        }
        IOError(file: String, message: String) {
            description("IO error when reading a file")
            display("IO error when reading {}: {}", file, message)
        }
        LexicalError(reason: String, line: String, line_number: usize, start: usize, end: usize) {
            description("Lexical error encountered")
            display("Lexical error encountered on line {},{}: {}\n{}",
                line_number, reason, *start, LineFormatter(line, *start, *end))
        }
        SyntaxError(file: String, line: String, line_number: usize, start: usize, end: usize) {
            description("SQL syntax error encountered")
            display(
                "SQL syntax error encountered in {} on line {},{}:\n{}",
                file, line_number, *start, LineFormatter(line, *start, *end))
        }
        ParseError(file: String, errors: Vec<ParseError<(), lexer::Token, &'static str>>) {
            description("Parser error")
            display("Parser errors in {}:\n{}", file, ParseErrorsFormatter(errors))
        }
        InlineParseError(error: ParseError<(), lexer::Token, &'static str>) {
            description("Parser error")
            display("Parser error: {}", ParseErrorFormatter(error))
        }
        HandledParseError(kind: ErrorKind) {
            description("Parser error")
            display("Parser error: {}", kind)
        }
        TemplateGenerationError(message: String) {
            description("Error generating template")
            display("Error generating template: {}", message)
        }
        GenerationError(message: String) {
            description("Error generating package")
            display("Error generating package: {}", message)
        }
        ValidationError(errors: Vec<ValidationKind>) {
            description("Package validation error")
            display("Package validation error{}:\n{}",
                if errors.len() > 1 { "s" } else { "" },
                ValidationErrorFormatter(errors)
            )
        }
        FormatError(file: String, message: String) {
            description("Format error when reading a file")
            display("Format error when reading {}: {}", file, message)
        }
        DatabaseError(message: String) {
            description("Database error")
            display("Database error: {}", message)
        }
        DatabaseExecuteError(query: String) {
            description("Database error executing query")
            display("Database error executing: {}", query)
        }
        DatabaseConnectionFinishError {
            description("Database connection couldn't finish")
            display("Database connection couldn't finish")
        }
        ExtractError(message: String) {
            description("Extract Error")
            display("Extraction Error: {}", message)
        }
        ProjectError(message: String) {
            description("Project format error")
            display("Project format error: {}", message)
        }
        PublishError(message: String) {
            description("Publish error")
            display("Publish error: {}", message)
        }
        MultipleErrors(errors: Vec<PsqlpackError>) {
            description("Multiple errors")
            display("Multiple errors:\n{}", MultipleErrorFormatter(errors))
        }
    }
}

fn write_err(f: &mut Formatter, error: &ParseError<(), lexer::Token, &'static str>) -> Result {
    match *error {
        ParseError::InvalidToken { .. } => write!(f, "Invalid token"),
        ParseError::UnrecognizedToken {
            ref token,
            ref expected,
        } => {
            writeln!(f, "Unexpected {:?}", token.1)?;
            write!(f, "   Expected one of:\n   {}", expected.join(", "))
        }
        ParseError::UnrecognizedEOF { ref expected, .. } => {
            writeln!(f, "Unexpected end of file")?;
            write!(f, "   Expected one of:\n   {}", expected.join(", "))
        }
        ParseError::ExtraToken { ref token } => write!(f, "Extra token detected: {:?}", token),
        ParseError::User { ref error } => write!(f, "{:?}", error),
    }
}

struct LineFormatter<'fmt>(&'fmt str, usize, usize);
const MAX_LINE_LENGTH: usize = 78;

impl<'fmt> Display for LineFormatter<'fmt> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let mut line = self.0;
        let mut start = self.1;
        let mut end = self.2;
        if line.len() > MAX_LINE_LENGTH {
            if start > 20 {
                let adj = start - 20;
                let (_, l) = line.split_at(adj);
                line = l;
                start -= adj;
                end -= adj;
            }
            if line.len() > MAX_LINE_LENGTH && end < MAX_LINE_LENGTH {
                let (l, _) = line.split_at(MAX_LINE_LENGTH);
                line = l;
            }
        }
        write!(f, "  {}\n  {}{}", line, " ".repeat(start), "^".repeat(end - start))?;
        Ok(())
    }
}

struct ParseErrorsFormatter<'fmt>(&'fmt Vec<ParseError<(), lexer::Token, &'static str>>);

impl<'fmt> Display for ParseErrorsFormatter<'fmt> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        for (i, error) in self.0.iter().enumerate() {
            write!(f, "{}: ", i,)?;
            write_err(f, error)?;
        }
        Ok(())
    }
}

struct ParseErrorFormatter<'fmt>(&'fmt ParseError<(), lexer::Token, &'static str>);

impl<'fmt> Display for ParseErrorFormatter<'fmt> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write_err(f, self.0)
    }
}

struct MultipleErrorFormatter<'fmt>(&'fmt Vec<PsqlpackError>);

impl<'fmt> Display for MultipleErrorFormatter<'fmt> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        for (i, error) in self.0.iter().enumerate() {
            write!(f, "--- Error {} ---\n{}", i, error)?;
        }
        Ok(())
    }
}

struct ValidationErrorFormatter<'fmt>(&'fmt Vec<ValidationKind>);

impl<'fmt> Display for ValidationErrorFormatter<'fmt> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        for error in self.0.iter() {
            writeln!(f, " - {}", error)?;
        }
        Ok(())
    }
}
