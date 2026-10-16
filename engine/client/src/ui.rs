//! Plugin egui : écran de connexion (`EngineState::Menu`, écrit `StartGame`) et HUD (`EngineState::InGame`).

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use engine_common::{DEFAULT_PORT, EngineState, Mode, StartGame};

use crate::hud::HudLines;

/// Nécessite une fenêtre et le rendu : absent des apps headless (tests).
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default()).add_systems(
            EguiPrimaryContextPass,
            (
                menu_ui.run_if(in_state(EngineState::Menu)),
                hud_ui.run_if(in_state(EngineState::InGame)),
            ),
        );
    }
}

/// Adresse proposée par défaut dans le champ "Rejoindre".
const DEFAULT_ADDR: &str = "127.0.0.1";

fn menu_ui(
    mut contexts: EguiContexts,
    mut start: MessageWriter<StartGame>,
    mut exit: MessageWriter<AppExit>,
    mut addr: Local<Option<String>>,
    mut error: Local<Option<String>>,
) -> Result {
    let addr = addr.get_or_insert_with(|| DEFAULT_ADDR.to_string());
    // egui 0.36 : un panneau s'affiche dans un `Ui`. Celui-ci couvre toute la fenêtre.
    let ctx = contexts.ctx_mut()?;
    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "menu".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    egui::CentralPanel::default().show(&mut viewport_ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.3);
            if ui.button("Héberger").clicked() {
                start.write(StartGame(Mode::Host { port: DEFAULT_PORT }));
            }
            ui.add_space(16.0);
            ui.add(egui::TextEdit::singleline(addr).desired_width(160.0));
            if ui.button("Rejoindre").clicked() {
                match Mode::parse_addr(addr.trim()) {
                    Ok(addr) => {
                        start.write(StartGame(Mode::Client { addr }));
                    }
                    Err(err) => *error = Some(err),
                }
            }
            if let Some(err) = error.as_deref() {
                ui.colored_label(egui::Color32::LIGHT_RED, err);
            }
            ui.add_space(16.0);
            if ui.button("Quitter").clicked() {
                exit.write(AppExit::Success);
            }
        });
    });
    Ok(())
}

/// Lignes non vides de `HudLines`, en haut à gauche. Zone non interactive (ne capte pas la souris).
fn hud_ui(mut contexts: EguiContexts, lines: Res<HudLines>) -> Result {
    let ctx = contexts.ctx_mut()?;
    if lines.0.iter().all(String::is_empty) {
        return Ok(());
    }
    egui::Area::new(egui::Id::new("hud"))
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(8.0, 8.0))
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                for line in lines.0.iter().filter(|l| !l.is_empty()) {
                    ui.label(line);
                }
            });
        });
    Ok(())
}
