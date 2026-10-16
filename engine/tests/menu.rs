//! Menu -> partie : sans `Mode`, l'app attend en `Menu` ; `StartGame` lance un host (joueur local).

mod common;

use bevy::prelude::*;
use common::{client_state, count, menu_app, run_until};
use engine_client::{ClientPlugin, ConnectionError};
use engine_common::bevy_quinnet::client::QuinnetClient;
use engine_common::bevy_quinnet::server::QuinnetServer;
use engine_common::bevy_replicon::prelude::ClientState;
use engine_common::{EngineState, LocalPlayer, Mode, StartGame};
use engine_server::ServerPlugin;

fn state(app: &App) -> EngineState {
    **app.world().resource::<State<EngineState>>()
}

#[test]
fn menu_then_start_host() {
    // Comme le binaire avec fenêtre : serveur et client présents, mode choisi plus tard.
    let mut app = menu_app();
    app.add_plugins((ServerPlugin, ClientPlugin));
    app.finish();

    run_until(&mut [&mut app], "menu", |a| {
        state(a[0]) == EngineState::Menu
    });
    assert!(app.world().get_resource::<Mode>().is_none());
    // Rien n'a démarré avant le choix du mode.
    assert!(!app.world().resource::<QuinnetServer>().is_listening());
    assert_eq!(count::<LocalPlayer>(&mut app), 0);

    // Port 0 : l'OS choisit un port libre.
    app.world_mut()
        .write_message(StartGame(Mode::Host { port: 0 }));
    run_until(&mut [&mut app], "partie lancée", |a| {
        state(a[0]) == EngineState::InGame && count::<LocalPlayer>(a[0]) == 1
    });
    assert_eq!(app.world().resource::<Mode>(), &Mode::Host { port: 0 });
    assert!(app.world().resource::<QuinnetServer>().is_listening());
}

#[test]
fn failed_connection_returns_to_menu() {
    // Port localhost fermé : on réserve un port UDP libre puis on le libère.
    let addr = {
        let socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        socket.local_addr().unwrap()
    };
    let mut app = menu_app();
    app.add_plugins(ClientPlugin);
    app.finish();
    run_until(&mut [&mut app], "menu", |a| {
        state(a[0]) == EngineState::Menu
    });

    app.world_mut()
        .write_message(StartGame(Mode::Client { addr }));
    run_until(&mut [&mut app], "partie lancée", |a| {
        state(a[0]) == EngineState::InGame
    });
    // Un vrai port fermé met plus de 10 s à échouer (timeout QUIC) : on ferme la connexion
    // pendant `Connecting`, ce que quinnet voit comme un échec (-> `Disconnected`).
    run_until(&mut [&mut app], "connexion en cours", |a| {
        client_state(a[0]) == ClientState::Connecting
    });
    app.world_mut()
        .resource_mut::<QuinnetClient>()
        .close_all_connections();
    run_until(&mut [&mut app], "retour au menu", |a| {
        a[0].world().get_resource::<ConnectionError>().is_some() && state(a[0]) == EngineState::Menu
    });
    assert!(app.world().get_resource::<Mode>().is_none());
    assert_eq!(
        app.world().resource::<ConnectionError>(),
        &ConnectionError(format!("connexion au serveur impossible : {addr}"))
    );

    // Un nouveau StartGame efface l'erreur et relance bien une connexion.
    app.world_mut()
        .write_message(StartGame(Mode::Client { addr }));
    run_until(&mut [&mut app], "relance", |a| {
        state(a[0]) == EngineState::InGame
    });
    assert!(app.world().get_resource::<ConnectionError>().is_none());
}
