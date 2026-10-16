//! Rendu 2D, inputs et UI. `ClientPlugin` : jeu (headless-compatible) ; `MenuPlugin` : écran de connexion (egui, fenêtre requise).

mod menu;
mod render;

pub use menu::MenuPlugin;

use std::net::Ipv4Addr;

use bevy::prelude::*;
use engine_common::bevy_quinnet::client::{
    ClientConnectionConfiguration, ClientConnectionConfigurationDefaultables, QuinnetClient,
    certificate::CertificateVerificationMode, connection::ClientAddrConfiguration,
};
use engine_common::bevy_replicon::prelude::{ClientState, RepliconChannels};
use engine_common::bevy_replicon_quinnet::ChannelsConfigurationExt;
use engine_common::{EngineState, LocalPlayer, Mode, OwnPlayer};

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(render::DefRenderPlugin)
            .add_systems(Startup, spawn_camera)
            .add_systems(Update, mark_local_player)
            .add_systems(OnEnter(EngineState::InGame), connect)
            .add_systems(OnEnter(ClientState::Connected), || {
                info!("connecté au serveur");
            })
            .add_systems(
                OnTransition {
                    exited: ClientState::Connecting,
                    entered: ClientState::Disconnected,
                },
                || error!("connexion au serveur impossible"),
            )
            .add_systems(
                OnTransition {
                    exited: ClientState::Connected,
                    entered: ClientState::Disconnected,
                },
                || warn!("déconnecté du serveur"),
            );
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Le serveur dit quelle entité est notre joueur. Le message n'arrive qu'une fois l'entité répliquée.
/// En host, il est livré localement (`ClientId::Server`) : même code que pour un client distant.
fn mark_local_player(mut own_player: MessageReader<OwnPlayer>, mut commands: Commands) {
    for OwnPlayer(player) in own_player.read() {
        commands.entity(*player).try_insert(LocalPlayer);
    }
}

/// Ouvre la connexion vers le serveur (mode client uniquement) au début de la partie :
/// les defs sont prêtes (sinon les entités arrivent sans leurs defs).
/// Le host n'ouvre rien : son joueur local est `ClientId::Server`.
/// Certificat serveur non vérifié (LAN/dev, voir docs/architecture.md).
fn connect(mode: Res<Mode>, channels: Res<RepliconChannels>, mut client: ResMut<QuinnetClient>) {
    let Mode::Client { addr } = *mode else {
        return;
    };
    info!("connexion à {addr}");
    let config = ClientConnectionConfiguration {
        addr_config: ClientAddrConfiguration::from_ips(
            addr.ip(),
            addr.port(),
            Ipv4Addr::UNSPECIFIED,
            0,
        ),
        cert_mode: CertificateVerificationMode::SkipVerification,
        defaultables: ClientConnectionConfigurationDefaultables {
            send_channels_cfg: channels.client_configs(),
        },
    };
    if let Err(err) = client.open_connection(config) {
        error!("ouverture de la connexion impossible : {err}");
    }
}
