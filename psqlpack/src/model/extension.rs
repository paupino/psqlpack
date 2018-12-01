use connection::Connection;
use errors::PsqlpackResult;
use model::{Capabilities, DefinableCatalog, MetaInfo, Package, SourceInfo};
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

        trace!(log, "Connecting to database");
        let db_conn = connection.connect_database()?;
        let meta = MetaInfo::new(SourceInfo::Extension);
        let context = capabilities.with_context(self);
        let schemas = context.schemata(&db_conn, connection.database())?;
        let types = context.types(&db_conn)?;
        let functions = context.functions(&db_conn)?;
        dbtry!(db_conn.finish());

        let mut package = Package {
            meta,
            extensions: Vec::new(),
            functions,
            indexes: Vec::new(), // TODO
            schemas,
            scripts: Vec::new(),
            tables: Vec::new(), // TODO
            types,
        };
        package.promote_primary_keys_to_table_constraints();
        Ok(package)
    }
}
