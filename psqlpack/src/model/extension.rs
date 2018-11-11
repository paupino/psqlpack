use connection::Connection;
use errors::{PsqlpackResult, PsqlpackErrorKind};
use model::{Capabilities, Package};
use semver::Semver;

use slog::Logger;

#[derive(Clone, Debug, PartialEq)]
pub struct Extension {
    pub name: String,
    pub version: Semver,
    pub installed: bool,
}

impl Extension {
    pub fn build_package_from_connection(&self,
                                         log: &Logger,
                                         connection: &Connection,
                                         capabilities: &Capabilities) -> PsqlpackResult<Package> {
        /*
        let mut package = Package {
            meta,
            extensions: Vec::new(),
            functions,
            indexes,
            schemas,
            scripts: Vec::new(),
            tables,
            types,
        };
        package.promote_primary_keys_to_table_constraints();
        Ok(package)*/
        panic!("")
    }
}
