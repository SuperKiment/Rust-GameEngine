//! Types partagés client/serveur : world 2D, defs data, protocole.

use bevy::prelude::*;

pub mod world;
pub use world::{Position, Rotation, Velocity, WorldConfig};

/// Mode de lancement du binaire d'un jeu.
#[derive(Resource, Clone, Debug, PartialEq)]
pub enum Mode {
    /// Serveur + client dans le même process (défaut, solo).
    Host,
    /// Serveur headless.
    Server,
    /// Client qui rejoint le serveur `addr`.
    Client { addr: String },
}

impl Mode {
    pub const USAGE: &'static str = "usage: <jeu> [host | server | client <addr>]";

    /// Parse les arguments de la ligne de commande (sans le nom du binaire).
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Mode, String> {
        let mut args = args.into_iter();
        let mode = match args.next().as_deref() {
            None | Some("host") => Mode::Host,
            Some("server") => Mode::Server,
            Some("client") => Mode::Client {
                addr: args
                    .next()
                    .ok_or_else(|| format!("adresse manquante\n{}", Self::USAGE))?,
            },
            Some(other) => return Err(format!("mode inconnu : {other}\n{}", Self::USAGE)),
        };
        match args.next() {
            None => Ok(mode),
            Some(extra) => Err(format!("argument en trop : {extra}\n{}", Self::USAGE)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Mode, String> {
        Mode::parse(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn modes() {
        assert_eq!(parse(&[]), Ok(Mode::Host));
        assert_eq!(parse(&["host"]), Ok(Mode::Host));
        assert_eq!(parse(&["server"]), Ok(Mode::Server));
        assert_eq!(
            parse(&["client", "127.0.0.1:5000"]),
            Ok(Mode::Client {
                addr: "127.0.0.1:5000".into()
            })
        );
    }

    #[test]
    fn errors() {
        assert!(parse(&["client"]).is_err());
        assert!(parse(&["nope"]).is_err());
        assert!(parse(&["server", "x"]).is_err());
    }
}
