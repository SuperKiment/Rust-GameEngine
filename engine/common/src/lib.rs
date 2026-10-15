//! Types partagés client/serveur : world 2D, defs data, protocole.

use std::net::{IpAddr, SocketAddr};

use bevy::prelude::*;

pub mod defs;
pub mod net;
pub mod spawn;
pub mod world;
pub use bevy_quinnet;
pub use bevy_replicon;
pub use bevy_replicon_quinnet;
pub use defs::{DefRegistry, DefsPlugin, EngineState, EntityDef, Visual};
pub use net::{DEFAULT_PORT, GameComponentExt, NetPlugin, ProtocolPlugin};
pub use spawn::{DefId, SpawnDef};
pub use world::{Position, Rotation, Velocity, WorldConfig};

/// Mode de lancement du binaire d'un jeu.
#[derive(Resource, Clone, Debug, PartialEq)]
pub enum Mode {
    /// Serveur + client dans le même process (défaut, solo). Écoute sur `port`.
    Host { port: u16 },
    /// Serveur headless qui écoute sur `port`.
    Server { port: u16 },
    /// Client qui rejoint le serveur `addr`.
    Client { addr: SocketAddr },
}

impl Mode {
    pub const USAGE: &'static str =
        "usage: <jeu> [host [port] | server [port] | client <ip[:port]>]";

    /// Parse les arguments de la ligne de commande (sans le nom du binaire).
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Mode, String> {
        let mut args = args.into_iter();
        let mode = match args.next().as_deref() {
            None => Mode::Host { port: DEFAULT_PORT },
            Some("host") => Mode::Host {
                port: Self::parse_port(args.next())?,
            },
            Some("server") => Mode::Server {
                port: Self::parse_port(args.next())?,
            },
            Some("client") => Mode::Client {
                addr: Self::parse_addr(
                    &args
                        .next()
                        .ok_or_else(|| format!("adresse manquante\n{}", Self::USAGE))?,
                )?,
            },
            Some(other) => return Err(format!("mode inconnu : {other}\n{}", Self::USAGE)),
        };
        match args.next() {
            None => Ok(mode),
            Some(extra) => Err(format!("argument en trop : {extra}\n{}", Self::USAGE)),
        }
    }

    fn parse_port(arg: Option<String>) -> Result<u16, String> {
        match arg {
            None => Ok(DEFAULT_PORT),
            Some(p) => p
                .parse()
                .map_err(|_| format!("port invalide : {p}\n{}", Self::USAGE)),
        }
    }

    /// `ip`, `ip:port`, `[ipv6]:port`, `localhost` ou `localhost:port`. Port par défaut : `DEFAULT_PORT`.
    fn parse_addr(arg: &str) -> Result<SocketAddr, String> {
        let arg = match arg.strip_prefix("localhost") {
            Some(rest) => format!("127.0.0.1{rest}"),
            None => arg.to_string(),
        };
        arg.parse::<SocketAddr>()
            .or_else(|_| {
                arg.parse::<IpAddr>()
                    .map(|ip| SocketAddr::new(ip, DEFAULT_PORT))
            })
            .map_err(|_| format!("adresse invalide : {arg}\n{}", Self::USAGE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Mode, String> {
        Mode::parse(args.iter().map(|s| s.to_string()))
    }

    fn client(addr: &str) -> Mode {
        Mode::Client {
            addr: addr.parse().unwrap(),
        }
    }

    #[test]
    fn modes() {
        let port = DEFAULT_PORT;
        assert_eq!(parse(&[]), Ok(Mode::Host { port }));
        assert_eq!(parse(&["host"]), Ok(Mode::Host { port }));
        assert_eq!(parse(&["host", "6000"]), Ok(Mode::Host { port: 6000 }));
        assert_eq!(parse(&["server"]), Ok(Mode::Server { port }));
        assert_eq!(parse(&["server", "6000"]), Ok(Mode::Server { port: 6000 }));
    }

    #[test]
    fn client_addr() {
        assert_eq!(
            parse(&["client", "127.0.0.1"]),
            Ok(client("127.0.0.1:5000"))
        );
        assert_eq!(
            parse(&["client", "10.0.0.2:6000"]),
            Ok(client("10.0.0.2:6000"))
        );
        assert_eq!(
            parse(&["client", "localhost"]),
            Ok(client("127.0.0.1:5000"))
        );
        assert_eq!(
            parse(&["client", "localhost:6000"]),
            Ok(client("127.0.0.1:6000"))
        );
        assert_eq!(parse(&["client", "[::1]:6000"]), Ok(client("[::1]:6000")));
        assert_eq!(parse(&["client", "::1"]), Ok(client("[::1]:5000")));
    }

    #[test]
    fn errors() {
        assert!(parse(&["client"]).is_err());
        assert!(parse(&["client", "pas-une-adresse"]).is_err());
        assert!(parse(&["client", "127.0.0.1:99999"]).is_err());
        assert!(parse(&["nope"]).is_err());
        assert!(parse(&["server", "x"]).is_err());
        assert!(parse(&["server", "70000"]).is_err());
        assert!(parse(&["server", "6000", "x"]).is_err());
        assert!(parse(&["host", "-1"]).is_err());
    }
}
