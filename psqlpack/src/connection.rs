use std::str::FromStr;

use postgres::{Client as PostgresClient, NoTls};

error_chain! {
    types {
        ConnectionError, ConnectionErrorKind, ConnectionResultExt, ConnectionResult;
    }
    foreign_links {
        PostgresConnect(::postgres::error::Error);
    }
    errors {
        MalformedConnectionString {
            description("The connection string was malformed.")
        }
        RequiredPartMissing(part: String) {
            description("Required connection string part missing")
            display("Required connection string part missing: '{}'", part)
        }
        TlsNotSupported {
            description("TLS connections are not currently supported.")
        }
    }
}

#[derive(Debug)]
pub struct Connection {
    database: String,
    uri: String,
}

impl Connection {
    pub fn database(&self) -> &str {
        &self.database
    }

    pub fn connect_host(&self) -> ConnectionResult<PostgresClient> {
        Ok(PostgresClient::connect(&self.uri, NoTls)?)
    }

    pub fn connect_database(&self) -> ConnectionResult<PostgresClient> {
        Ok(PostgresClient::connect(&self.uri_with_database(), NoTls)?)
    }

    fn uri_with_database(&self) -> String {
        format!("{}/{}", self.uri, self.database)
    }
}

impl FromStr for Connection {
    type Err = ConnectionError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        use std::collections::HashMap;

        // First up, parse the connection string
        let mut parts = HashMap::new();
        for section in input.split(';') {
            if section.trim().is_empty() {
                continue;
            }

            let pair: Vec<&str> = section.split('=').collect();
            if pair.len() != 2 {
                bail!(ConnectionErrorKind::MalformedConnectionString);
            }

            parts.insert(pair[0], pair[1]);
        }

        let host = match parts.get(&"host") {
            Some(host) => *host,
            None => bail!(ConnectionErrorKind::RequiredPartMissing("host".to_owned())),
        };
        let database = match parts.get(&"database") {
            Some(database) => *database,
            None => bail!(ConnectionErrorKind::RequiredPartMissing("database".to_owned())),
        };
        let user = match parts.get(&"userid") {
            Some(user) => *user,
            None => bail!(ConnectionErrorKind::RequiredPartMissing("userid".to_owned())),
        };

        let mut builder = ConnectionBuilder::new(database, host, user);

        if let Some(password) = parts.get("password") {
            builder.with_password(*password);
        }

        if let Some(port) = parts.get("port") {
            match u16::from_str(port) {
                Ok(port) => builder.with_port(port),
                Err(_) => bail!(ConnectionErrorKind::MalformedConnectionString),
            };
        }

        if let Some(tls_mode) = parts.get("tlsmode") {
            builder.with_tls_mode(*tls_mode);
        }

        // Make sure we have enough for a connection string
        Ok(builder.build()?)
    }
}

pub struct ConnectionBuilder {
    database: String,
    host: String,
    user: String,
    password: Option<String>,
    port: Option<u16>,
    tls_mode: bool,
}

impl ConnectionBuilder {
    pub fn new<S: Into<String>>(database: S, host: S, user: S) -> ConnectionBuilder {
        ConnectionBuilder {
            database: database.into(),
            host: host.into(),
            user: user.into(),
            password: None,
            port: None,
            tls_mode: false,
        }
    }

    pub fn with_password<S: Into<String>>(&mut self, value: S) -> &mut ConnectionBuilder {
        self.password = Some(value.into());
        self
    }

    pub fn with_port(&mut self, value: u16) -> &mut ConnectionBuilder {
        self.port = Some(value);
        self
    }

    pub fn with_tls_mode(&mut self, value: &str) -> &mut ConnectionBuilder {
        self.tls_mode = value.eq_ignore_ascii_case("true");
        self
    }

    pub fn build(&self) -> ConnectionResult<Connection> {
        if self.tls_mode {
            Err(ConnectionErrorKind::TlsNotSupported.into())
        } else {
            let fq_host = match self.port {
                Some(port) => format!("{}:{}", self.host, port),
                None => self.host.to_owned(),
            };
            let uri = match self.password {
                Some(ref password) => format!("postgres://{}:{}@{}", self.user, password, fq_host),
                None => format!("postgres://{}@{}", self.user, fq_host),
            };
            Ok(Connection {
                database: self.database.clone(),
                uri,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_error_kind {
        ($err:expr, $kind:pat) => {
            match *$err.kind() {
                $kind => assert!(true, "{:?} is of kind {:?}", $err, stringify!($kind)),
                _ => assert!(false, "{:?} is NOT of kind {:?}", $err, stringify!($kind)),
            }
        };
    }

    #[test]
    fn builder_basic_works() {
        let connection = ConnectionBuilder::new("database", "host", "user").build().unwrap();
        assert_eq!("database", connection.database());
        assert_eq!("postgres://user@host", connection.uri);
    }

    #[test]
    fn builder_with_password_works() {
        let connection = ConnectionBuilder::new("database", "host", "user")
            .with_password("password")
            .build()
            .unwrap();
        assert_eq!("database", connection.database());
        assert_eq!("postgres://user:password@host", connection.uri);
    }

    #[test]
    fn builder_with_tls_fails() {
        let error = ConnectionBuilder::new("database", "host", "user")
            .with_tls_mode("true")
            .build()
            .unwrap_err();
        assert_error_kind!(error, ConnectionErrorKind::TlsNotSupported);
    }

    #[test]
    fn parse_basic_works() {
        let connection: Connection = "host=localhost;database=db1;userid=user;".parse().unwrap();
        assert_eq!("db1", connection.database());
        assert_eq!("postgres://user@localhost", connection.uri);
    }

    #[test]
    fn parse_with_password_works() {
        let connection: Connection = "host=localhost;database=db1;userid=user;password=secret;"
            .parse()
            .unwrap();
        assert_eq!("db1", connection.database());
        assert_eq!("postgres://user:secret@localhost", connection.uri);
    }

    #[test]
    fn parse_without_host_fails() {
        let error = "database=db1;userid=user;".parse::<Connection>().unwrap_err();
        assert_error_kind!(error, ConnectionErrorKind::RequiredPartMissing(_));
    }

    #[test]
    fn parse_without_database_fails() {
        let error = "host=localhost;userid=user;".parse::<Connection>().unwrap_err();
        assert_error_kind!(error, ConnectionErrorKind::RequiredPartMissing(_));
    }

    #[test]
    fn parse_without_user_fails() {
        let error = "host=localhost;database=db1".parse::<Connection>().unwrap_err();
        assert_error_kind!(error, ConnectionErrorKind::RequiredPartMissing(_));
    }
}
