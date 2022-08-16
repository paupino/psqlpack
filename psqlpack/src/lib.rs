#![recursion_limit = "1024"]

extern crate chrono;
#[macro_use]
extern crate error_chain;
extern crate lalrpop_util;
#[macro_use]
extern crate lazy_static;
extern crate petgraph;
extern crate postgres;
extern crate regex;
extern crate rust_decimal;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate slog;
extern crate glob;
extern crate slog_stdlog;
extern crate zip;

mod errors;
pub use crate::errors::*;
mod connection;
mod model;
mod semver;
mod sql;

pub mod ast {
    pub use crate::sql::ast::*;
}
pub use crate::connection::ConnectionBuilder;
pub use crate::errors::{PsqlpackErrorKind, PsqlpackResult};
pub use crate::model::{
    template, Capabilities, Delta, Dependency, GenerationOptions, Package, Project, PublishProfile, Toggle,
};
pub use crate::semver::Semver;

/// Allows usage of no logging, std `log`, or slog.
pub enum LogConfig {
    NoLogging,
    StdLog,
}
pub use crate::LogConfig::*;

impl From<LogConfig> for slog::Logger {
    fn from(config: LogConfig) -> Self {
        use slog::{Discard, Drain, Logger};
        match config {
            NoLogging => Logger::root(Discard.fuse(), o!()),
            StdLog => Logger::root(slog_stdlog::StdLog.fuse(), o!()),
        }
    }
}
