macro_rules! dbtry {
    ($expr:expr) => {
        match $expr {
            Ok(o) => o,
            Err(e) => bail!(DatabaseError(format!("{}", e))),
        }
    };
}

mod profiles;
mod project;
mod package;
mod delta;

pub use self::profiles::PublishProfile;
pub use self::project::Project;
pub use self::package::{Node, Package, ValidationKind};
pub use self::delta::Delta;
