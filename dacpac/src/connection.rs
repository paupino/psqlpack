use std::ascii::AsciiExt;

use postgres::TlsMode;

use errors::*;

pub struct ConnectionString {
    pub database : Option<String>,
    host : Option<String>,
    user : Option<String>,
    password : Option<String>,
    tls_mode : bool,
}

macro_rules! assert_existance {
    ($s:ident, $field:ident, $errors:ident) => {{
        if $s.$field.is_none() {
            let text = stringify!($field);
            $errors.push(DacpacErrorKind::InvalidConnectionString(format!("No {} defined", text)));
        }
    }};
}

impl ConnectionString {
    pub fn new() -> Self {
        ConnectionString {
            database: None,
            host: None,
            user: None,
            password: None,
            tls_mode: false
        }
    }

    pub fn set_database(&mut self, value: &str) {
        self.database = Some(value.to_owned());
    }

    pub fn set_host(&mut self, value: &str) {
        self.host = Some(value.to_owned());
    }

    pub fn set_user(&mut self, value: &str) {
        self.user = Some(value.to_owned());
    }

    pub fn set_password(&mut self, value: &str) {
        self.password = Some(value.to_owned());
    }

    pub fn set_tls_mode(&mut self, value: &str) {
        self.tls_mode = value.eq_ignore_ascii_case("true");
    }

    pub fn validate(&self) -> DacpacResult<()> {
        let mut errors = Vec::new();
        assert_existance!(self, database, errors);
        assert_existance!(self, host, errors);
        assert_existance!(self, user, errors);
        if self.tls_mode {
            errors.push(DacpacErrorKind::InvalidConnectionString("TLS not supported".to_owned()));
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(DacpacErrorKind::MultipleErrors(errors).into())
        }
    }

    pub fn uri(&self, with_database: bool) -> String {
        // Assumes validate has been called
        if self.password.is_none() {
            if with_database {
                format!("postgres://{}@{}/{}", self.user.clone().unwrap(), self.host.clone().unwrap(), self.database.clone().unwrap())
            } else {
                format!("postgres://{}@{}", self.user.clone().unwrap(), self.host.clone().unwrap())
            }
        } else {
            if with_database {
                format!("postgres://{}:{}@{}/{}", self.user.clone().unwrap(), self.password.clone().unwrap(), self.host.clone().unwrap(), self.database.clone().unwrap())
            } else {
                format!("postgres://{}:{}@{}", self.user.clone().unwrap(), self.password.clone().unwrap(), self.host.clone().unwrap())
            }
        }
    }

    pub fn tls_mode(&self) -> TlsMode {
        TlsMode::None
    }
}
