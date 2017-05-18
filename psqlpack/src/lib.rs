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
mod model;
mod psqlpack;
mod graph;
mod lexer;
#[cfg_attr(feature = "cargo-clippy", allow(clippy))]
#[cfg_attr(rustfmt, rustfmt_skip)]
mod sql;

pub use errors::*;
pub use psqlpack::Psqlpack;
