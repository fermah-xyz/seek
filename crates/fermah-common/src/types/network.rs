use std::{
    fmt::Display,
    net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs},
};

use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use strum::Display;
use thiserror::Error;
use url::{ParseError, Url};

#[derive(Serialize, Deserialize, Display, ValueEnum, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Network {
    Local,
    Dev,
    Main,
}

#[derive(
    Serialize, Deserialize, Display, ValueEnum, Default, Debug, Copy, Clone, PartialEq, Eq, Hash,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ConnectionProtocol {
    #[default]
    Ws,
    Wss,
    Http,
    Https,
    File,
}

#[derive(Error, Debug)]
pub enum ConnectionParseError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("address does not resolve to a host: {0}")]
    Resolution(String),

    #[error("url parse error: {0}")]
    Url(#[from] ParseError),
}

/// Represents a parsed remote connection using a protocol.
#[derive(Serialize, Deserialize, Parser, Copy, Clone, Debug)]
pub struct Connection {
    pub proto: Option<ConnectionProtocol>,
    pub host: IpAddr,
    pub port: u16,
}

impl Default for Connection {
    fn default() -> Self {
        Self {
            proto: None,
            host: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 8080,
        }
    }
}

impl Connection {
    pub fn try_from_str(value: &str) -> Result<Self, ConnectionParseError> {
        let addresses = value.to_socket_addrs()?;
        let addr = addresses
            .last()
            .ok_or(ConnectionParseError::Resolution(value.to_string()))?;

        let proto = Url::parse(value)
            .ok()
            .and_then(|url| ConnectionProtocol::from_str(url.scheme(), true).ok());

        Ok(Connection {
            proto,
            host: addr.ip(),
            port: addr.port(),
        })
    }
}

impl TryFrom<&str> for Connection {
    type Error = ConnectionParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Connection::try_from_str(value)
    }
}

impl From<Connection> for Url {
    fn from(conn: Connection) -> Self {
        Url::parse(&format!(
            "{}://{}:{}",
            conn.proto.unwrap_or_default(),
            conn.host,
            conn.port
        ))
        .unwrap()
    }
}

impl From<Connection> for SocketAddr {
    fn from(value: Connection) -> Self {
        SocketAddr::new(value.host, value.port)
    }
}

impl From<SocketAddr> for Connection {
    fn from(value: SocketAddr) -> Self {
        Connection {
            proto: None,
            host: value.ip(),
            port: value.port(),
        }
    }
}

impl Display for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}://{}:{}",
            self.proto.unwrap_or_default(),
            self.host,
            self.port
        )
    }
}
