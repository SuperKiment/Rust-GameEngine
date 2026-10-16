//! Simulation autoritaire, headless.

use std::net::Ipv4Addr;

use bevy::prelude::*;
use engine_common::bevy_quinnet::server::{
    EndpointAddrConfiguration, QuinnetServer, ServerEndpointConfiguration,
    ServerEndpointConfigurationDefaultables, certificate::CertificateRetrievalMode,
};
use engine_common::bevy_replicon::prelude::{
    AuthorizedClient, ClientId, ConnectedClient, RepliconChannels, SendTargets, ToClients,
};
use engine_common::bevy_replicon::shared::backend::connected_client::NetworkId;
use engine_common::bevy_replicon_quinnet::ChannelsConfigurationExt;
use engine_common::{
    EngineState, Mode, OwnPlayer, Owner, Position, SpawnDef, Velocity, WorldConfig,
};

/// Def de l'entité joueur : `assets/defs/player.def.ron` du jeu.
const PLAYER_DEF: &str = "player";

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(EngineState::Ready),
            (start_listening, spawn_host_player),
        )
        .add_observer(log_client_connected)
        .add_observer(log_client_disconnected)
        .add_observer(spawn_client_player)
        .add_observer(despawn_client_player)
        .add_systems(FixedUpdate, integrate_velocity);
    }
}

/// Ouvre l'endpoint réseau sur `0.0.0.0:<port>` (modes host et server), une fois les defs prêtes :
/// le réseau s'ouvre quand les données sont prêtes (sinon les entités arrivent sans leurs defs).
/// Certificat auto-signé : le client ne le vérifie pas (LAN/dev, voir docs/architecture.md).
fn start_listening(
    mode: Res<Mode>,
    channels: Res<RepliconChannels>,
    mut server: ResMut<QuinnetServer>,
    mut exit: MessageWriter<AppExit>,
) {
    let (Mode::Host { port } | Mode::Server { port }) = *mode else {
        return;
    };
    let config = ServerEndpointConfiguration {
        addr_config: EndpointAddrConfiguration::from_ip(Ipv4Addr::UNSPECIFIED, port),
        cert_mode: CertificateRetrievalMode::GenerateSelfSigned {
            server_hostname: Ipv4Addr::LOCALHOST.to_string(),
        },
        defaultables: ServerEndpointConfigurationDefaultables {
            send_channels_cfg: channels.server_configs(),
        },
    };
    match server.start_endpoint(config) {
        Ok(_) => info!("serveur en écoute sur {}", server.endpoint().local_addr()),
        Err(err) => {
            error!("impossible d'écouter sur le port {port} (port déjà utilisé ?) : {err:?}");
            // Un serveur dédié sans réseau ne sert à rien : on quitte. Le host reste jouable en solo.
            if matches!(*mode, Mode::Server { .. }) {
                exit.write(AppExit::error());
            }
        }
    }
}

/// Spawn l'entité joueur de `owner` et lui dit laquelle c'est (`OwnPlayer`).
/// Même chemin pour un client distant et pour le joueur local du host (`ClientId::Server`).
fn spawn_player(
    owner: ClientId,
    commands: &mut Commands,
    own_player: &mut MessageWriter<ToClients<OwnPlayer>>,
) {
    let player = commands
        .spawn_def(PLAYER_DEF, Vec2::ZERO)
        .insert(Owner(owner))
        .id();
    own_player.write(ToClients {
        targets: SendTargets::Single(owner),
        message: OwnPlayer(player),
    });
}

/// Host : joueur du serveur lui-même. Serveur dédié : pas de joueur local.
fn spawn_host_player(
    mode: Res<Mode>,
    mut commands: Commands,
    mut own_player: MessageWriter<ToClients<OwnPlayer>>,
) {
    if matches!(*mode, Mode::Host { .. }) {
        spawn_player(ClientId::Server, &mut commands, &mut own_player);
    }
}

fn spawn_client_player(
    add: On<Add<AuthorizedClient>>,
    mut commands: Commands,
    mut own_player: MessageWriter<ToClients<OwnPlayer>>,
) {
    spawn_player(ClientId::Client(add.entity), &mut commands, &mut own_player);
}

fn despawn_client_player(
    remove: On<Remove<ConnectedClient>>,
    players: Query<(Entity, &Owner)>,
    mut commands: Commands,
) {
    for (player, owner) in &players {
        if owner.0 == ClientId::Client(remove.entity) {
            commands.entity(player).despawn();
        }
    }
}

fn log_client_connected(add: On<Add<AuthorizedClient>>, ids: Query<&NetworkId>) {
    match ids.get(add.entity) {
        Ok(id) => info!("client connecté : {} (id réseau {})", add.entity, id.get()),
        Err(_) => info!("client connecté : {}", add.entity),
    }
}

fn log_client_disconnected(remove: On<Remove<ConnectedClient>>, ids: Query<&NetworkId>) {
    match ids.get(remove.entity) {
        Ok(id) => info!(
            "client déconnecté : {} (id réseau {})",
            remove.entity,
            id.get()
        ),
        Err(_) => info!("client déconnecté : {}", remove.entity),
    }
}

/// `Position += Velocity * dt`, puis clamp aux bornes si `WorldConfig` existe.
fn integrate_velocity(
    time: Res<Time<Fixed>>,
    config: Option<Res<WorldConfig>>,
    mut query: Query<(&mut Position, &Velocity)>,
) {
    let dt = time.delta_secs();
    for (mut pos, vel) in &mut query {
        pos.0 += vel.0 * dt;
        if let Some(config) = &config {
            pos.0 = config.clamp(pos.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;
    use bevy::time::TimeUpdateStrategy;
    use engine_common::NetPlugin;

    #[test]
    fn velocity_moves_position_within_bounds() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, NetPlugin, ServerPlugin))
            .insert_resource(Mode::Server { port: 0 })
            .insert_resource(WorldConfig {
                bounds: Rect::new(-10.0, -10.0, 10.0, 10.0),
                tile_size: 1.0,
            })
            .insert_resource(TimeUpdateStrategy::ManualDuration(
                Time::<Fixed>::default().timestep(),
            ));
        app.finish(); // replicon prépare ses canaux dans `finish`
        let e = app
            .world_mut()
            .spawn((Position::default(), Velocity(Vec2::new(64.0, 0.0))))
            .id();

        // Fixed = 64 Hz : 64 u/s -> 1 u par tick. 1re update : le temps s'initialise, aucun tick.
        app.update();
        app.update();
        let x = app.world().get::<Position>(e).unwrap().0.x;
        assert!((x - 1.0).abs() < 1e-4, "x = {x}");

        for _ in 0..20 {
            app.update();
        }
        assert_eq!(app.world().get::<Position>(e).unwrap().0.x, 10.0);
    }
}
