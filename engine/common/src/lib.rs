//! Types partagés client/serveur : world 2D, defs data, protocole.

use std::net::{IpAddr, SocketAddr};

use bevy::prelude::*;

pub mod defs;
pub mod net;
pub mod player;
pub mod spawn;
pub mod world;
pub use bevy_quinnet;
pub use bevy_replicon;
pub use bevy_replicon_quinnet;
pub use defs::{DefRegistry, DefsPlugin, EngineState, EntityDef, Visual};
pub use net::{DEFAULT_PORT, GameComponentExt, NetPlugin, ProtocolPlugin};
pub use player::{GameInputExt, LocalPlayer, OwnPlayer, Owner};
pub use spawn::{DefId, SpawnDef};
pub use world::{Position, Rotation, Velocity, WorldConfig};

/// Mode de la partie. Resource insérée par la CLI (`Mode::parse`) ou par le menu (`StartGame`) :
/// absente tant qu'aucune partie n'est choisie.
#[derive(Resource, Clone, Debug, PartialEq)]
pub enum Mode {
    /// Serveur + client dans le même process (solo). Écoute sur `port`.
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
    /// Sans argument : `None`, le joueur choisit dans le menu.
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Option<Mode>, String> {
        let mut args = args.into_iter();
        let mode = match args.next().as_deref() {
            None => return Ok(None),
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
                )
                .map_err(|err| format!("{err}\n{}", Self::USAGE))?,
            },
            Some(other) => return Err(format!("mode inconnu : {other}\n{}", Self::USAGE)),
        };
        match args.next() {
            None => Ok(Some(mode)),
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

    /// `ip`, `ip:port`, `localhost` ou `localhost:port`. Port par défaut : `DEFAULT_PORT`.
    /// Utilisée par la CLI et par le menu de connexion.
    pub fn parse_addr(arg: &str) -> Result<SocketAddr, String> {
        let arg = match arg.strip_prefix("localhost") {
            Some(rest) => format!("127.0.0.1{rest}"),
            None => arg.to_string(),
        };
        let addr = arg
            .parse::<SocketAddr>()
            .or_else(|_| {
                arg.parse::<IpAddr>()
                    .map(|ip| SocketAddr::new(ip, DEFAULT_PORT))
            })
            .map_err(|_| format!("adresse invalide : {arg}"))?;
        if addr.is_ipv6() {
            return Err("IPv6 non supporté : le serveur écoute en IPv4".to_string());
        }
        Ok(addr)
    }
}

/// Message client : lancer une partie dans ce mode (envoyé par le menu). Ignoré hors `EngineState::Menu`.
#[derive(Message, Clone, Debug)]
pub struct StartGame(pub Mode);

/// Run condition : vraie en mode `Host` ou `Server`. À mettre sur tout système serveur d'un jeu :
/// le même plugin tourne aussi en mode `Client` et avant le choix du mode (menu).
pub fn is_server(mode: Option<Res<Mode>>) -> bool {
    matches!(
        mode.as_deref(),
        Some(Mode::Host { .. } | Mode::Server { .. })
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Option<Mode>, String> {
        Mode::parse(args.iter().map(|s| s.to_string()))
    }

    fn client(addr: &str) -> Option<Mode> {
        Some(Mode::Client {
            addr: addr.parse().unwrap(),
        })
    }

    #[test]
    fn modes() {
        let port = DEFAULT_PORT;
        assert_eq!(parse(&[]), Ok(None)); // pas de mode : menu
        assert_eq!(parse(&["host"]), Ok(Some(Mode::Host { port })));
        assert_eq!(
            parse(&["host", "6000"]),
            Ok(Some(Mode::Host { port: 6000 }))
        );
        assert_eq!(parse(&["server"]), Ok(Some(Mode::Server { port })));
        assert_eq!(
            parse(&["server", "6000"]),
            Ok(Some(Mode::Server { port: 6000 }))
        );
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
    }

    #[test]
    fn errors() {
        assert!(parse(&["client"]).is_err());
        assert!(parse(&["client", "pas-une-adresse"]).is_err());
        assert!(parse(&["client", "127.0.0.1:99999"]).is_err());
        assert!(parse(&["client", "[::1]:6000"]).is_err());
        assert!(parse(&["client", "::1"]).is_err());
        assert!(parse(&["nope"]).is_err());
        assert!(parse(&["server", "x"]).is_err());
        assert!(parse(&["server", "70000"]).is_err());
        assert!(parse(&["server", "6000", "x"]).is_err());
        assert!(parse(&["host", "-1"]).is_err());
    }

    #[test]
    fn parse_addr_errors_without_usage() {
        let err = Mode::parse_addr("pas-une-adresse").unwrap_err();
        assert!(!err.contains("usage"), "{err}");
        assert!(Mode::parse_addr("::1").is_err());
    }

    /// Nombre de passages d'un système `run_if(is_server)` sur une app dans ce mode.
    fn runs(mode: Option<Mode>) -> u32 {
        #[derive(Resource, Default)]
        struct Runs(u32);

        let mut app = App::new();
        app.init_resource::<Runs>().add_systems(
            Update,
            (|mut runs: ResMut<Runs>| runs.0 += 1).run_if(is_server),
        );
        if let Some(mode) = mode {
            app.insert_resource(mode);
        }
        app.update();
        app.world().resource::<Runs>().0
    }

    #[test]
    fn is_server_only_for_host_and_server() {
        assert_eq!(runs(Some(Mode::Host { port: 0 })), 1);
        assert_eq!(runs(Some(Mode::Server { port: 0 })), 1);
        assert_eq!(runs(client("127.0.0.1:5000")), 0);
        assert_eq!(runs(None), 0); // menu : pas encore de mode
    }
}
