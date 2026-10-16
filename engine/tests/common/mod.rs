//! Helpers communs aux tests d'intégration réseau (localhost, un process).
#![allow(dead_code)] // chaque fichier de test n'utilise qu'une partie

use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use bevy::{asset::AssetPlugin, prelude::*, state::app::StatesPlugin};
use engine_common::bevy_replicon::prelude::*;
use engine_common::{DefsPlugin, EngineState, Mode, NetPlugin};

/// App headless avec le réseau et le vrai chargement des defs (`tests/assets`), sans `Mode` :
/// `Menu` quand `world.ron` et `defs/` sont chargés.
pub fn menu_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: format!("{}/tests/assets", env!("CARGO_MANIFEST_DIR")),
            ..default()
        },
        StatesPlugin,
        NetPlugin,
        DefsPlugin,
    ))
    // Assets que le rendu du client remplit : pas de GPU ici, mais les stores doivent exister.
    .init_asset::<Mesh>()
    .init_asset::<ColorMaterial>()
    .init_asset::<Image>();
    app
}

/// Comme `menu_app` avec un mode fourni (CLI) : `InGame` quand les defs sont chargées.
pub fn game_app(mode: Mode) -> App {
    let mut app = menu_app();
    app.insert_resource(mode);
    app
}

/// Fait tourner les apps ensemble jusqu'à `done` (timeout 10 s).
pub fn run_until(apps: &mut [&mut App], what: &str, done: impl Fn(&mut [&mut App]) -> bool) {
    let start = Instant::now();
    while !done(apps) {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "timeout : {what}"
        );
        for app in apps.iter_mut() {
            app.update();
        }
        sleep(Duration::from_millis(10));
    }
}

pub fn count<C: Component>(app: &mut App) -> usize {
    let world = app.world_mut();
    world.query::<&C>().iter(world).count()
}

pub fn client_state(client: &App) -> ClientState {
    **client.world().resource::<State<ClientState>>()
}

pub fn is_ready(app: &App) -> bool {
    **app.world().resource::<State<EngineState>>() == EngineState::InGame
}
