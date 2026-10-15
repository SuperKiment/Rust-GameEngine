//! Façade : un jeu ne dépend que de `engine`.

use bevy::{app::ScheduleRunnerPlugin, log::LogPlugin, prelude::*, window::WindowPlugin};
use engine_client::ClientPlugin;
use engine_common::Mode;
use engine_server::ServerPlugin;

pub use bevy;

pub mod prelude {
    pub use super::EnginePlugin;
    pub use bevy::prelude::*;
    pub use engine_common::{Mode, Position, Rotation, Velocity, WorldConfig};
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
            Mode::Server => {
                app.add_plugins((
                    // Boucle calée sur le pas de FixedUpdate (64 Hz par défaut) : un tick par frame.
                    MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
                        Time::<Fixed>::default().timestep(),
                    )),
                    LogPlugin::default(),
                    ServerPlugin,
                ));
            }
            Mode::Client { .. } => {
                app.add_plugins((self.window_plugins(), ClientPlugin));
            }
            Mode::Host => {
                app.add_plugins((self.window_plugins(), ServerPlugin, ClientPlugin));
            }
        }
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
            mode: Mode::Server,
        });
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(app.world().resource::<Mode>(), &Mode::Server);
    }
}
