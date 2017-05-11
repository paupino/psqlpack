#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate lazy_static;
extern crate lalrpop_util;
#[macro_use]
extern crate log;
extern crate postgres;
extern crate regex;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate walkdir;
extern crate zip;

mod ast;
mod errors;
mod connection;
mod profiles;
mod project;
mod package;
mod psqlpack;
mod graph;
mod lexer;
mod sql;

pub use errors::*;
pub use psqlpack::Psqlpack;
