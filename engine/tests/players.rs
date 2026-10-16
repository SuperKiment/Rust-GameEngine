//! Joueur par client et inputs : 1 serveur + 2 clients réels (localhost), vrai chargement des defs
//! (`tests/assets`), et le host sans réseau.

mod common;

use bevy::prelude::*;
use common::{client_state, game_app, is_ready, run_until};
use engine_client::ClientPlugin;
use engine_common::bevy_quinnet::{client::QuinnetClient, server::QuinnetServer};
use engine_common::bevy_replicon::prelude::{ClientId, ClientState, ServerState};
use engine_common::{DefId, GameInputExt, LocalPlayer, Mode, Owner, Position};
use engine_server::ServerPlugin;
use serde::{Deserialize, Serialize};

/// Input de test : déplacement à appliquer une fois par message reçu.
#[derive(Component, Message, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
struct Move(Vec2);

/// Système serveur de test (le code du jeu) : applique chaque nouveau `Move` à la position.
fn apply_move(mut players: Query<(&Move, &mut Position), Changed<Move>>) {
    for (movement, mut pos) in &mut players {
        pos.0 += movement.0;
    }
}

fn server_app(mode: Mode) -> App {
    let mut app = game_app(mode);
    app.add_plugins(ServerPlugin)
        .register_input::<Move>()
        .add_systems(Update, apply_move);
    app
}

fn client_app(port: u16) -> App {
    let mut app = game_app(Mode::Client {
        addr: ([127, 0, 0, 1], port).into(),
    });
    app.add_plugins(ClientPlugin).register_input::<Move>();
    app.finish();
    app
}

/// Positions des entités joueur (`DefId == "player"`), triées.
fn player_positions(app: &mut App) -> Vec<Vec2> {
    let world = app.world_mut();
    let mut query = world.query::<(&DefId, &Position)>();
    let mut positions: Vec<_> = query
        .iter(world)
        .filter(|(def, _)| def.0 == "player")
        .map(|(_, pos)| pos.0)
        .collect();
    positions.sort_by(|a, b| a.x.total_cmp(&b.x));
    positions
}

/// Position de l'unique entité `LocalPlayer` (None si zéro ou plusieurs).
fn local_position(app: &mut App) -> Option<Vec2> {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&Position, With<LocalPlayer>>();
    let mut iter = query.iter(world);
    let pos = iter.next()?.0;
    iter.next().is_none().then_some(pos)
}

fn send_move(client: &mut App, movement: Vec2) {
    client.world_mut().write_message(Move(movement));
}

#[test]
fn each_client_moves_its_own_player() {
    let mut server = server_app(Mode::Server { port: 0 });
    server.finish();
    // Les defs se chargent, puis `InGame` ouvre l'endpoint.
    run_until(&mut [&mut server], "serveur prêt", |a| {
        is_ready(a[0]) && a[0].world().resource::<QuinnetServer>().is_listening()
    });
    let port = server
        .world()
        .resource::<QuinnetServer>()
        .endpoint()
        .local_addr()
        .port();
    // Serveur dédié : aucun joueur sans client.
    assert!(player_positions(&mut server).is_empty());

    let (mut a, mut b) = (client_app(port), client_app(port));
    run_until(&mut [&mut server, &mut a, &mut b], "joueurs", |apps| {
        local_position(apps[1]).is_some() && local_position(apps[2]).is_some()
    });
    // Un joueur par client, chacun lié à un client différent.
    let world = server.world_mut();
    let owners: Vec<_> = world.query::<&Owner>().iter(world).map(|o| o.0).collect();
    assert_eq!(owners.len(), 2);
    assert!(owners.iter().all(|o| matches!(o, ClientId::Client(_))));
    assert_ne!(owners[0], owners[1]);

    // Chaque client déplace son joueur différemment.
    let (move_a, move_b) = (Vec2::new(3.0, 1.0), Vec2::new(-5.0, 2.0));
    send_move(&mut a, move_a);
    send_move(&mut b, move_b);
    run_until(&mut [&mut server, &mut a, &mut b], "déplacements", |apps| {
        player_positions(apps[0]) == [move_b, move_a]
            && local_position(apps[1]) == Some(move_a)
            && local_position(apps[2]) == Some(move_b)
            && player_positions(apps[1]) == [move_b, move_a]
            && player_positions(apps[2]) == [move_b, move_a]
    });

    // B part : son joueur disparaît chez le serveur et chez A.
    let mut quinnet = b.world_mut().resource_mut::<QuinnetClient>();
    let connection = quinnet.get_default_connection().unwrap();
    quinnet.close_connection(connection).unwrap();
    run_until(
        &mut [&mut server, &mut a, &mut b],
        "déconnexion de B",
        |apps| {
            client_state(apps[2]) == ClientState::Disconnected
                && player_positions(apps[0]) == [move_a]
                && player_positions(apps[1]) == [move_a]
        },
    );
}

#[test]
fn host_has_local_player_and_receives_its_input() {
    // Port déjà pris : l'endpoint ne démarre pas (log d'erreur). La livraison locale ne dépend
    // que de `ClientState::Disconnected` (bevy_replicon src/server/message.rs:184-188).
    let taken = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
    let port = taken.local_addr().unwrap().port();
    let mut host = server_app(Mode::Host { port });
    host.add_plugins(ClientPlugin);
    host.finish();

    run_until(&mut [&mut host], "joueur local du host", |a| {
        local_position(a[0]).is_some()
    });
    let world = host.world_mut();
    let mut query = world.query_filtered::<&Owner, With<LocalPlayer>>();
    assert_eq!(query.single(world).unwrap().0, ClientId::Server);

    // Input écrit localement : livré au joueur sans passer par le réseau.
    let movement = Vec2::new(2.0, -4.0);
    send_move(&mut host, movement);
    run_until(&mut [&mut host], "input du host", |a| {
        local_position(a[0]) == Some(movement)
    });
    // Le port pris a bien empêché le serveur de démarrer.
    assert_ne!(
        *host.world().resource::<State<ServerState>>().get(),
        ServerState::Running
    );
    drop(taken);
}
