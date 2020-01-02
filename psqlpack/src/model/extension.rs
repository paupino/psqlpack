use crate::connection::Connection;
use crate::errors::PsqlpackResult;
use crate::model::{Capabilities, DefinableCatalog, MetaInfo, Package, SourceInfo};
use crate::semver::Semver;

use slog::Logger;

#[derive(Clone, Debug, PartialEq)]
pub struct Extension {
    pub name: String,
    pub version: Semver,
    pub installed: bool,
}

impl Extension {
    pub fn build_package_from_connection(
        &self,
        log: &Logger,
        connection: &Connection,
        capabilities: &Capabilities,
    ) -> PsqlpackResult<Package> {
        trace!(log, "Connecting to database");
        let mut client = connection.connect_database()?;
        let meta = MetaInfo::new(SourceInfo::Extension(self.name.to_owned()));
        let context = capabilities.with_context(self);
        let schemas = context.schemata(&mut client, connection.database())?;
        let types = context.types(&mut client)?;
        let functions = context.functions(&mut client)?;
        let tables = context.tables(&mut client)?;
        let indexes = context.indexes(&mut client)?;

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
        Ok(package)
    }
}
