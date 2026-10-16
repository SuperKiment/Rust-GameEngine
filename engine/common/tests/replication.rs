//! Réplication serveur -> client sans réseau (backend de test replicon) : assets sous `tests/assets/ok`.

use bevy::{asset::AssetPlugin, prelude::*, state::app::StatesPlugin};
use bevy_replicon::{prelude::*, test_app::ServerTestAppExt};
use engine_common::{
    DefId, DefsPlugin, EngineState, GameComponentExt, Position, ProtocolPlugin, Rotation, SpawnDef,
    Velocity,
};
use serde::{Deserialize, Serialize};
use std::{thread::sleep, time::Duration};

#[derive(Component, Reflect, Serialize, Deserialize, Debug, PartialEq)]
#[reflect(Component)]
struct TestHealth(u32);

#[derive(Component, Reflect, Debug, PartialEq)]
#[reflect(Component)]
struct TestTag;

/// Même app pour serveur et client (mêmes plugins, même ordre d'enregistrement).
fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: format!("{}/tests/assets/ok", env!("CARGO_MANIFEST_DIR")),
            ..default()
        },
        StatesPlugin,
        RepliconPlugins.set(ServerPlugin::new(PostUpdate)), // un tick par update
        ProtocolPlugin,
        DefsPlugin,
    ))
    .register_game_component::<TestHealth>()
    .register_type::<TestTag>();
    app.finish();
    app
}

fn ready(app: &App) -> bool {
    *app.world().resource::<State<EngineState>>().get() == EngineState::Menu
}

/// Serveur et client prêts (`Menu`, pas de mode) et connectés.
fn connected() -> (App, App) {
    let (mut server, mut client) = (app(), app());
    for _ in 0..100 {
        server.update();
        client.update();
        if ready(&server) && ready(&client) {
            server.connect_client(&mut client);
            return (server, client);
        }
        sleep(Duration::from_millis(20));
    }
    panic!("jamais Menu");
}

/// Un tick serveur, envoi au client, mise à jour du client.
fn sync(server: &mut App, client: &mut App) {
    server.update();
    server.exchange_with_client(client);
    client.update();
}

fn remote_entity(client: &mut App) -> Entity {
    let world = client.world_mut();
    let mut query = world.query_filtered::<Entity, With<Remote>>();
    query.single(world).expect("une entité répliquée")
}

#[test]
fn def_entity_is_replicated_and_rebuilt() {
    let (mut server, mut client) = connected();

    let pos = Vec2::new(3.0, -4.0);
    server.world_mut().commands().spawn_def("hero", pos);
    sync(&mut server, &mut client);

    let e = remote_entity(&mut client);
    let world = client.world();
    // Répliqués par le moteur.
    assert_eq!(world.get::<DefId>(e), Some(&DefId("hero".into())));
    assert_eq!(world.get::<Position>(e), Some(&Position(pos)));
    // Reconstruit depuis la copie locale des defs (TestHealth: valeur de la def, TestTag: non répliqué).
    assert_eq!(world.get::<TestHealth>(e), Some(&TestHealth(30)));
    assert!(world.get::<TestTag>(e).is_some());
}

#[test]
fn replicated_value_wins_over_def() {
    let (mut server, mut client) = connected();

    // La def dit 30, le serveur force 99 : le client doit voir 99.
    server
        .world_mut()
        .commands()
        .spawn_def("hero", Vec2::ZERO)
        .insert((TestHealth(99), Rotation(1.5), Velocity(Vec2::ONE)));
    sync(&mut server, &mut client);

    let e = remote_entity(&mut client);
    let world = client.world();
    assert_eq!(world.get::<TestHealth>(e), Some(&TestHealth(99)));
    assert_eq!(world.get::<Rotation>(e), Some(&Rotation(1.5)));
    // Pas répliqué en V1.
    assert!(world.get::<Velocity>(e).is_none());

    // Changement ultérieur côté serveur : répliqué aussi.
    let se = {
        let w = server.world_mut();
        w.query_filtered::<Entity, With<Replicated>>()
            .iter(w)
            .next()
            .unwrap()
    };
    server.world_mut().entity_mut(se).insert(TestHealth(7));
    sync(&mut server, &mut client);
    assert_eq!(client.world().get::<TestHealth>(e), Some(&TestHealth(7)));
}

#[test]
fn unknown_def_keeps_replicated_components() {
    let (mut server, mut client) = connected();

    server.world_mut().commands().spawn_def("nope", Vec2::ONE);
    sync(&mut server, &mut client);

    let e = remote_entity(&mut client);
    let world = client.world();
    assert_eq!(world.get::<DefId>(e), Some(&DefId("nope".into())));
    assert_eq!(world.get::<Position>(e), Some(&Position(Vec2::ONE)));
    assert!(world.get::<TestHealth>(e).is_none());
}
