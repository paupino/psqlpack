macro_rules! dbtry {
    ($expr:expr) => {
        match $expr {
            Ok(o) => o,
            Err(e) => bail!(crate::PsqlpackErrorKind::DatabaseError(format!("{}", e))),
        }
    };
}

macro_rules! map {
    ($expr:expr) => {{
        $expr.iter().map(|row| row.into()).collect()
    }};
}

mod capabilities;
mod extension;
mod profiles;
mod project;
mod package;
mod delta;
pub mod template;

pub use self::capabilities::{Capabilities, DefinableCatalog};
pub use self::extension::{Extension};
pub use self::profiles::{GenerationOptions, PublishProfile, Toggle};
pub use self::project::{Dependency, Project};
pub use self::package::{MetaInfo, Node, Package, SourceInfo, ValidationKind};
pub use self::delta::Delta;
