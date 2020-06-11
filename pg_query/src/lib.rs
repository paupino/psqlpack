extern crate libc;

pub mod pg_types {
    include!(concat!(env!("OUT_DIR"), "/types.rs"));
}

use libc::{c_char, c_int};

use std::ffi::{CString, CStr};
use std::fmt;

use pg_types::Node;

#[repr(C)]
#[derive(Copy, Clone)]
struct PgQueryError {
    pub message: *mut c_char,
    pub filename: *mut c_char,
    pub lineno: c_int,
    pub cursorpos: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct PgQueryParseResult {
    pub parse_tree: *mut c_char,
    pub stderr_buffer: *mut c_char,
    pub error: *mut PgQueryError,
}

#[derive(Clone)]
pub struct ParseError {
    pub message: String,
    pub file: String,
    pub line: u32,
    pub index: usize,
    _p: (),
}

impl fmt::Debug for ParseError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Error")
            .field("message", &self.message)
            .field("file", &self.file)
            .field("line", &self.line)
            .field("index", &self.index)
            .finish()
    }
}

impl ParseError {
    unsafe fn from_raw(raw: *mut PgQueryError) -> ParseError {
        ParseError {
            message: std::str::from_utf8(CStr::from_ptr((*raw).message).to_bytes()).unwrap().to_owned(),
            file: std::str::from_utf8(CStr::from_ptr((*raw).filename).to_bytes()).unwrap().to_owned(),
            line: (*raw).lineno as u32,
            index: (*raw).cursorpos as usize,
            _p: (),
        }
    }
}

extern "C" {
    fn pg_query_parse(input: *const c_char) -> PgQueryParseResult;
    fn pg_query_free_parse_result(result: PgQueryParseResult);
}

fn parse_internal(query: &str) -> Result<String, ParseError> {
    let query = CString::new(query).expect("interior null");
    unsafe {
        let raw_result = pg_query_parse(query.as_ptr() as *mut _);
        let result = if raw_result.error.is_null() {
            Ok(std::str::from_utf8(CStr::from_ptr(raw_result.parse_tree).to_bytes()).unwrap().to_owned())
        } else {
            Err(ParseError::from_raw(raw_result.error))
        };
        pg_query_free_parse_result(raw_result);
        result
    }
}

pub fn parse(query: &str) -> Result<Node, ParseError> {
    let json = parse_internal(query)?;
    let root_node = serde_json::from_str(&json).unwrap_or_else(|_| return Err(ParseError {
        message: "Failed to deserialize root node".into(),
        file: String::new(),
        line: 0,
        index: 0,
        _p: ()
    }))?;
    root_node
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        println!("{:?}", super::parse("CREATE INDEX ix_test ON contacts.person (id, ssn) WHERE ssn IS NOT NULL;").unwrap());
        assert!(false);
    }
}
