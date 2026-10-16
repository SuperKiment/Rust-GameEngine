use engine::bevy_egui::{EguiContexts, EguiPrimaryContextPass};
use engine::egui;
use engine::prelude::*;

fn main() -> AppExit {
    App::new()
        .add_plugins(EnginePlugin::from_args("Sample"))
        // HUD en une ligne : composant du joueur local.
        .add_hud::<Position>(|p| format!("Position : {:.0}, {:.0}", p.0.x, p.0.y))
        // Écran custom : un système egui dans son schedule, gardé par l'état de jeu.
        // Sur le serveur headless ce schedule n'est jamais exécuté.
        .add_systems(
            EguiPrimaryContextPass,
            help_window.run_if(in_state(EngineState::InGame)),
        )
        .run()
}

fn help_window(mut contexts: EguiContexts) -> Result {
    egui::Window::new("Aide").show(contexts.ctx_mut()?, |ui| {
        ui.label("Exemple d'écran custom (fenêtre egui).");
        ui.label("Le gameplay du sample arrive avec l'issue #15.");
    });
    Ok(())
}
