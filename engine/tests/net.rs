//! Connexion réseau réelle (localhost) : un serveur et un client dans le même process.

use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use bevy::{prelude::*, state::app::StatesPlugin};
use engine_client::ClientPlugin;
use engine_common::bevy_quinnet::{client::QuinnetClient, server::QuinnetServer};
use engine_common::bevy_replicon::prelude::*;
use engine_common::{EngineState, Mode, NetPlugin};
use engine_server::ServerPlugin;

/// App headless avec le réseau, sans fenêtre ni chargement de defs (état `Ready` forcé).
fn net_app(mode: Mode) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin, NetPlugin))
        .insert_state(EngineState::Ready)
        .insert_resource(mode);
    app
}

/// Fait tourner les deux apps jusqu'à `done` (timeout 5 s).
fn run_until(
    server: &mut App,
    client: &mut App,
    what: &str,
    done: impl Fn(&mut App, &mut App) -> bool,
) {
    let start = Instant::now();
    while !done(server, client) {
        assert!(start.elapsed() < Duration::from_secs(5), "timeout : {what}");
        server.update();
        client.update();
        sleep(Duration::from_millis(10));
    }
}

fn count<C: Component>(app: &mut App) -> usize {
    let world = app.world_mut();
    world.query::<&C>().iter(world).count()
}

fn client_state(client: &App) -> ClientState {
    **client.world().resource::<State<ClientState>>()
}

#[test]
fn client_connects_then_disconnects() {
    // Port 0 : l'OS choisit un port libre, lu ensuite sur l'endpoint.
    let mut server = net_app(Mode::Server { port: 0 });
    server.add_plugins(ServerPlugin);
    server.finish();
    server.update(); // OnEnter(Ready) : ouvre l'endpoint
    let port = server
        .world()
        .resource::<QuinnetServer>()
        .endpoint()
        .local_addr()
        .port();

    let mut client = net_app(Mode::Client {
        addr: ([127, 0, 0, 1], port).into(),
    });
    client.add_plugins(ClientPlugin);
    client.finish();

    run_until(&mut server, &mut client, "connexion", |s, c| {
        client_state(c) == ClientState::Connected && count::<AuthorizedClient>(s) == 1
    });

    // Fermeture côté client : le serveur voit partir le client.
    let mut quinnet = client.world_mut().resource_mut::<QuinnetClient>();
    let connection = quinnet.get_default_connection().unwrap();
    quinnet.close_connection(connection).unwrap();
    run_until(&mut server, &mut client, "déconnexion", |s, c| {
        client_state(c) == ClientState::Disconnected && count::<ConnectedClient>(s) == 0
    });
}
