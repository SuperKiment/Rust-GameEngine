//! Chargement headless des defs : assets sous `tests/assets/{ok,bad}`.

use std::{thread::sleep, time::Duration};

use bevy::{
    asset::{AssetPlugin, LoadState},
    prelude::*,
    state::app::StatesPlugin,
};
use engine_common::{DefRegistry, DefsPlugin, EngineState, EntityDef, Visual, WorldConfig};

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
fn valid_defs_reach_ready() {
    let mut app = app("ok");
    let ready = run_until(&mut app, |a| {
        *a.world().resource::<State<EngineState>>().get() == EngineState::Ready
    });
    assert!(ready, "jamais Ready");

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
