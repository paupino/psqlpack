macro_rules! dbtry {
    ($expr:expr) => {
        match $expr {
            Ok(o) => o,
            Err(e) => bail!(crate::PsqlpackErrorKind::DatabaseError(format!("{}", e))),
        }
    };
}

mod capabilities;
mod delta;
mod extension;
mod package;
mod profiles;
mod project;
pub mod template;

pub use self::capabilities::{Capabilities, DefinableCatalog};
pub use self::delta::Delta;
pub use self::extension::Extension;
pub use self::package::{MetaInfo, Node, Package, SourceInfo, ValidationKind};
pub use self::profiles::{GenerationOptions, PublishProfile, Toggle};
pub use self::project::{Dependency, Project};
