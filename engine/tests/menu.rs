//! Menu -> partie : sans `Mode`, l'app attend en `Menu` ; `StartGame` lance un host (joueur local).

mod common;

use bevy::prelude::*;
use common::{count, menu_app, run_until};
use engine_client::ClientPlugin;
use engine_common::bevy_quinnet::server::QuinnetServer;
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
