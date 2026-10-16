//! Façade : un jeu ne dépend que de `engine`.

use bevy::{
    app::ScheduleRunnerPlugin, asset::AssetPlugin, log::LogPlugin, prelude::*,
    state::app::StatesPlugin, window::WindowPlugin,
};
use engine_client::{ClientPlugin, UiPlugin};
use engine_common::{DefsPlugin, Mode, NetPlugin};
use engine_server::ServerPlugin;

pub use bevy;
pub use engine_client::bevy_egui;
pub use engine_client::bevy_egui::egui;

pub mod prelude {
    pub use super::EnginePlugin;
    pub use bevy::prelude::*;
    pub use engine_client::HudExt;
    pub use engine_common::{
        DefId, DefRegistry, EngineState, EntityDef, GameComponentExt, GameInputExt, LocalPlayer,
        Mode, Owner, Position, Rotation, SpawnDef, Velocity, Visual, WorldConfig, is_server,
    };
}

pub struct EnginePlugin {
    pub title: String,
    /// `None` : le mode est choisi au menu (binaire avec fenêtre).
    pub mode: Option<Mode>,
}

impl EnginePlugin {
    /// Lit le mode depuis les arguments du process (sans argument : menu).
    /// Usage sur stderr + exit 2 si invalide.
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
        if let Some(mode) = &self.mode {
            app.insert_resource(mode.clone());
        }
        match &self.mode {
            Some(Mode::Server { .. }) => {
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
            // Binaire avec fenêtre (menu, host, client) : le mode peut être choisi au runtime,
            // le serveur est donc toujours présent (ses systèmes sont gardés par `is_server`).
            _ => {
                app.add_plugins((
                    self.window_plugins(),
                    NetPlugin,
                    ServerPlugin,
                    ClientPlugin,
                    UiPlugin,
                ));
            }
        }
        app.add_plugins(DefsPlugin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn server_runs_headless() {
        let mut app = App::new();
        app.add_plugins(EnginePlugin {
            title: "test".into(),
            mode: Some(Mode::Server { port: 0 }),
        });
        app.finish(); // replicon prépare ses canaux dans `finish`
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(app.world().resource::<Mode>(), &Mode::Server { port: 0 });
    }

    #[test]
    fn server_runs_with_hud_and_egui_system() {
        let mut app = App::new();
        app.add_plugins(EnginePlugin {
            title: "test".into(),
            mode: Some(Mode::Server { port: 0 }),
        });
        // Comme un jeu : HUD + écran egui ajoutés aussi sur le serveur headless.
        app.add_hud::<Position>(|p| format!("{p:?}")).add_systems(
            bevy_egui::EguiPrimaryContextPass,
            (|| ()).run_if(in_state(EngineState::InGame)),
        );
        app.finish();
        for _ in 0..3 {
            app.update();
        }
    }
}
