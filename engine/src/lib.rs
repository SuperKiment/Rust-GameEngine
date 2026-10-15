//! Façade : un jeu ne dépend que de `engine`.

use bevy::{
    app::ScheduleRunnerPlugin, asset::AssetPlugin, log::LogPlugin, prelude::*,
    state::app::StatesPlugin, window::WindowPlugin,
};
use engine_client::ClientPlugin;
use engine_common::{DefsPlugin, Mode, NetPlugin};
use engine_server::ServerPlugin;

pub use bevy;

pub mod prelude {
    pub use super::EnginePlugin;
    pub use bevy::prelude::*;
    pub use engine_common::{
        DefId, DefRegistry, EngineState, EntityDef, Mode, Position, Rotation, SpawnDef, Velocity,
        Visual, WorldConfig,
    };
}

pub struct EnginePlugin {
    pub title: String,
    pub mode: Mode,
}

impl EnginePlugin {
    /// Lit le mode depuis les arguments du process. Usage sur stderr + exit 2 si invalide.
    pub fn from_args(title: impl Into<String>) -> Self {
        let mode = Mode::parse(std::env::args().skip(1)).unwrap_or_else(|err| {
            eprintln!("{err}");
            std::process::exit(2);
        });
        Self {
            title: title.into(),
            mode,
        }
    }

    fn window_plugins(&self) -> impl PluginGroup {
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: self.title.clone(),
                ..default()
            }),
            ..default()
        })
    }
}

impl Plugin for EnginePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.mode.clone());
        match &self.mode {
            Mode::Server { .. } => {
                app.add_plugins((
                    // Boucle calée sur le pas de FixedUpdate (64 Hz par défaut) : un tick par frame.
                    MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
                        Time::<Fixed>::default().timestep(),
                    )),
                    LogPlugin::default(),
                    AssetPlugin::default(),
                    StatesPlugin,
                    NetPlugin,
                    ServerPlugin,
                ));
            }
            Mode::Client { .. } => {
                app.add_plugins((self.window_plugins(), NetPlugin, ClientPlugin));
            }
            Mode::Host { .. } => {
                app.add_plugins((self.window_plugins(), NetPlugin, ServerPlugin, ClientPlugin));
            }
        }
        app.add_plugins(DefsPlugin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_runs_headless() {
        let mut app = App::new();
        app.add_plugins(EnginePlugin {
            title: "test".into(),
            mode: Mode::Server { port: 0 },
        });
        app.finish(); // replicon prépare ses canaux dans `finish`
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(app.world().resource::<Mode>(), &Mode::Server { port: 0 });
    }
}
