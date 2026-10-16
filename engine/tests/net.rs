//! Connexion réseau réelle (localhost) : un serveur et un client dans le même process.

mod common;

use common::{client_state, count, game_app, run_until};
use engine_client::ClientPlugin;
use engine_common::Mode;
use engine_common::bevy_quinnet::{client::QuinnetClient, server::QuinnetServer};
use engine_common::bevy_replicon::prelude::*;
use engine_server::ServerPlugin;

#[test]
fn client_connects_then_disconnects() {
    // Port 0 : l'OS choisit un port libre, lu ensuite sur l'endpoint.
    let mut server = game_app(Mode::Server { port: 0 });
    server.add_plugins(ServerPlugin);
    server.finish();
    // Les defs se chargent, puis `Ready` ouvre l'endpoint.
    run_until(&mut [&mut server], "serveur prêt", |a| {
        a[0].world().resource::<QuinnetServer>().is_listening()
    });
    let port = server
        .world()
        .resource::<QuinnetServer>()
        .endpoint()
        .local_addr()
        .port();

    let mut client = game_app(Mode::Client {
        addr: ([127, 0, 0, 1], port).into(),
    });
    client.add_plugins(ClientPlugin);
    client.finish();

    run_until(&mut [&mut server, &mut client], "connexion", |a| {
        client_state(a[1]) == ClientState::Connected && count::<AuthorizedClient>(a[0]) == 1
    });

    // Fermeture côté client : le serveur voit partir le client.
    let mut quinnet = client.world_mut().resource_mut::<QuinnetClient>();
    let connection = quinnet.get_default_connection().unwrap();
    quinnet.close_connection(connection).unwrap();
    run_until(&mut [&mut server, &mut client], "déconnexion", |a| {
        client_state(a[1]) == ClientState::Disconnected && count::<ConnectedClient>(a[0]) == 0
    });
}
