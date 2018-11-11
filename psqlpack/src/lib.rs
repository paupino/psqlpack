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
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate slog;
extern crate slog_stdlog;
#[cfg(test)]
#[macro_use]
extern crate spectral;
extern crate glob;
extern crate zip;

mod errors;
pub use errors::*;
mod connection;
mod sql;
mod model;
mod semver;

pub mod ast {
    pub use sql::ast::*;
}
pub use connection::ConnectionBuilder;
pub use errors::{PsqlpackErrorKind, PsqlpackResult};
pub use model::{
    Capabilities,
    Delta,
    Dependency,
    GenerationOptions,
    Package,
    Project,
    PublishProfile,
    Toggle,
    template,
};
pub use semver::Semver;

/// Allows usage of no logging, std `log`, or slog.
pub enum LogConfig {
    NoLogging,
    StdLog,
}
pub use LogConfig::*;

impl From<LogConfig> for slog::Logger {
    fn from(config: LogConfig) -> Self {
        use slog::{Discard, Drain, Logger};
        match config {
            NoLogging => Logger::root(Discard.fuse(), o!()),
            StdLog => Logger::root(slog_stdlog::StdLog.fuse(), o!()),
        }
    }
}
