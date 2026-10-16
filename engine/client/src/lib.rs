//! Rendu 2D, inputs et UI. `ClientPlugin` : jeu (headless-compatible) ; `UiPlugin` : menu de connexion et HUD (egui, fenêtre requise).

mod hud;
mod render;
mod ui;

pub use bevy_egui;
pub use hud::{HudExt, HudLines};
pub use ui::UiPlugin;

/// Échec de connexion à afficher dans le menu (posé au retour en `Menu`, effacé au prochain `StartGame`).
#[derive(Resource, Debug, PartialEq)]
pub struct ConnectionError(pub String);

use std::net::Ipv4Addr;

use bevy::prelude::*;
use engine_common::bevy_quinnet::client::{
    ClientConnectionConfiguration, ClientConnectionConfigurationDefaultables, QuinnetClient,
    certificate::CertificateVerificationMode, connection::ClientAddrConfiguration,
};
use engine_common::bevy_replicon::prelude::{ClientState, RepliconChannels};
use engine_common::bevy_replicon_quinnet::ChannelsConfigurationExt;
use engine_common::{EngineState, LocalPlayer, Mode, OwnPlayer, StartGame};

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudLines>()
            .add_plugins(render::DefRenderPlugin)
            .add_systems(Startup, spawn_camera)
            .add_systems(Update, (mark_local_player, clear_connection_error))
            .add_systems(OnEnter(EngineState::InGame), connect)
            .add_systems(OnEnter(ClientState::Connected), || {
                info!("connecté au serveur");
            })
            .add_systems(
                OnTransition {
                    exited: ClientState::Connecting,
                    entered: ClientState::Disconnected,
                },
                connection_failed,
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

/// Échec de connexion : retour au menu avec le message. Sans `Mode`, l'écran de jeu n'a plus de raison d'être.
fn connection_failed(
    mode: Option<Res<Mode>>,
    mut next: ResMut<NextState<EngineState>>,
    mut commands: Commands,
) {
    let msg = match mode.as_deref() {
        Some(Mode::Client { addr }) => format!("connexion au serveur impossible : {addr}"),
        _ => "connexion au serveur impossible".to_string(),
    };
    error!("{msg}");
    commands.insert_resource(ConnectionError(msg));
    commands.remove_resource::<Mode>();
    next.set(EngineState::Menu);
}

/// Un nouveau `StartGame` efface l'erreur précédente.
fn clear_connection_error(mut start: MessageReader<StartGame>, mut commands: Commands) {
    if start.read().next().is_some() {
        commands.remove_resource::<ConnectionError>();
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
