//! Chargement headless des defs : assets sous `tests/assets/{ok,bad}`.

use std::{thread::sleep, time::Duration};

use bevy::{
    asset::{AssetPlugin, LoadState},
    prelude::*,
    state::app::StatesPlugin,
};
use engine_common::{
    DefId, DefRegistry, DefsPlugin, EngineState, EntityDef, Mode, Position, SpawnDef, StartGame,
    Visual, WorldConfig,
};

#[derive(Component, Reflect, Debug, PartialEq)]
#[reflect(Component)]
struct TestHealth(u32);

#[derive(Component, Reflect, Debug, PartialEq)]
#[reflect(Component)]
struct TestTag;

fn app(root: &str) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: format!("{}/tests/assets/{root}", env!("CARGO_MANIFEST_DIR")),
            ..default()
        },
        StatesPlugin,
        DefsPlugin,
    ))
    .register_type::<TestHealth>()
    .register_type::<TestTag>();
    app
}

/// Met à jour l'app (max 100 fois) jusqu'à ce que `done` soit vrai.
fn run_until(app: &mut App, done: impl Fn(&App) -> bool) -> bool {
    for _ in 0..100 {
        app.update();
        if done(app) {
            return true;
        }
        sleep(Duration::from_millis(20));
    }
    false
}

#[test]
fn valid_defs_reach_menu_without_mode() {
    let mut app = app("ok");
    let ready = run_until(&mut app, |a| {
        *a.world().resource::<State<EngineState>>().get() == EngineState::Menu
    });
    assert!(ready, "jamais Menu");

    let world = app.world();
    assert_eq!(world.resource::<WorldConfig>().tile_size, 32.0);
    let handle = world.resource::<DefRegistry>().get("hero").expect("hero");
    let def = world.resource::<Assets<EntityDef>>().get(handle).unwrap();
    assert_eq!(def.layer, 2);
    assert!(matches!(def.visual, Some(Visual::Rect { .. })));
    assert_eq!(
        def.components[0].try_downcast_ref::<TestHealth>(),
        Some(&TestHealth(30))
    );
    assert!(def.components[1].try_downcast_ref::<TestTag>().is_some());
}

#[test]
fn valid_defs_reach_in_game_with_mode() {
    let mut app = app("ok");
    app.insert_resource(Mode::Server { port: 0 });
    let in_game = run_until(&mut app, |a| {
        *a.world().resource::<State<EngineState>>().get() == EngineState::InGame
    });
    assert!(in_game, "jamais InGame");
}

#[test]
fn start_game_sets_mode_and_enters_in_game() {
    let mut app = app("ok");
    assert!(run_until(&mut app, |a| {
        *a.world().resource::<State<EngineState>>().get() == EngineState::Menu
    }));
    assert!(app.world().get_resource::<Mode>().is_none());

    app.world_mut()
        .write_message(StartGame(Mode::Server { port: 1234 }));
    assert!(run_until(&mut app, |a| {
        *a.world().resource::<State<EngineState>>().get() == EngineState::InGame
    }));
    assert_eq!(app.world().resource::<Mode>(), &Mode::Server { port: 1234 });
}

#[test]
fn unknown_component_fails_and_stays_loading() {
    let mut app = app("bad");
    let handle: Handle<EntityDef> = app
        .world()
        .resource::<AssetServer>()
        .load("defs/bad.def.ron");
    let failed = run_until(&mut app, |a| {
        a.world()
            .resource::<AssetServer>()
            .load_state(&handle)
            .is_failed()
    });
    assert!(failed, "le chargement aurait dû échouer");

    let state = app.world().resource::<AssetServer>().load_state(&handle);
    let LoadState::Failed(err) = state else {
        unreachable!()
    };
    let msg = err.to_string();
    assert!(msg.contains("NopeComponent"), "{msg}");
    assert!(msg.contains("bad.def.ron"), "{msg}");

    app.update();
    assert_eq!(
        *app.world().resource::<State<EngineState>>().get(),
        EngineState::Loading
    );
}

#[test]
fn spawn_def_inserts_components() {
    let mut app = app("ok");
    assert!(run_until(&mut app, |a| {
        *a.world().resource::<State<EngineState>>().get() == EngineState::Menu
    }));

    let pos = Vec2::new(3.0, -4.0);
    let mut commands = app.world_mut().commands();
    let hero = commands.spawn_def("hero", pos).id();
    let ghost = commands.spawn_def("nope", pos).id();
    app.update();

    let world = app.world();
    assert_eq!(world.get::<DefId>(hero), Some(&DefId("hero".into())));
    assert_eq!(world.get::<Position>(hero), Some(&Position(pos)));
    assert_eq!(world.get::<TestHealth>(hero), Some(&TestHealth(30)));
    assert!(world.get::<TestTag>(hero).is_some());
    assert_eq!(world.get::<DefId>(ghost), Some(&DefId("nope".into())));
    assert!(world.get::<TestHealth>(ghost).is_none());
}
