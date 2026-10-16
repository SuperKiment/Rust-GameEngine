# UI : écrans et HUD

UI = bevy_egui 0.43 (egui 0.36), réexporté par `engine` : `engine::bevy_egui`, `engine::egui`. Le jeu n'ajoute aucune dépendance.
Le plugin egui (`UiPlugin`, `engine/client/src/ui.rs`) n'existe que dans le binaire avec fenêtre : jamais sur le serveur headless ni dans les tests.

## HUD en une ligne
`HudExt` (dans le prelude) ajoute une ligne en haut à gauche, visible en `EngineState::InGame`. Les lignes suivent l'ordre d'enregistrement.

```rust
app.add_hud::<Health>(|h| format!("PV : {}", h.0))       // composant du joueur local
    .add_hud_resource::<Score>(|s| format!("Score : {}", s.0)); // resource
```
- Ligne vide (non affichée) tant que le composant/la resource est absent : pas de joueur local encore, ou serveur dédié.
- Ces helpers n'utilisent pas egui : à appeler dans le plugin du jeu, sans condition de mode (sans danger sur le serveur headless).
- Composant : celui de l'entité `LocalPlayer`. Pour le voir côté client, il doit être répliqué : `app.register_game_component::<Health>()` (voir `docs/networking.md`).
- Resource : lue dans le monde local, donc par défaut propre au client. bevy_replicon 0.45 sait répliquer une resource (`replicate_resource`, stockée comme composant d'une entité `Replicated`), mais le moteur n'a pas encore de helper `register_game_resource` : un score partagé demande aujourd'hui d'appeler replicon directement (`engine_common::bevy_replicon`), ce que `engine` ne réexporte pas. Pour un score par joueur, préférer un composant sur le joueur.
- Texte brut seulement (une `String` par ligne). Pour du riche (barre, icône), écrire un écran custom.

## Écran custom
Un système egui dans le schedule `EguiPrimaryContextPass`, gardé par l'état voulu. Exemple (`games/sample/src/main.rs`) :

```rust
use engine::bevy_egui::{EguiContexts, EguiPrimaryContextPass};
use engine::egui;

app.add_systems(
    EguiPrimaryContextPass,
    help_window.run_if(in_state(EngineState::InGame)),
);

fn help_window(mut contexts: EguiContexts) -> Result {
    egui::Window::new("Aide").show(contexts.ctx_mut()?, |ui| {
        ui.label("Exemple d'écran custom (fenêtre egui).");
    });
    Ok(())
}
```
- Le système peut être ajouté sur le serveur : sans `EguiPlugin`, ce schedule n'est jamais exécuté.
- Lecture de l'état de jeu : paramètres ECS normaux (`Query`, `Res`). Écriture de l'état de jeu : envoyer un input au serveur (serveur autoritaire), pas de modification directe.
- `ctx_mut()` renvoie un `Result` : le système retourne `Result` et utilise `?`.
- Un panneau plein écran se montre dans un `egui::Ui` créé sur `ctx.viewport_rect()` (voir `menu_ui`) ; fenêtres et `Area` se montrent directement sur le contexte.

## Menu de connexion
`menu_ui` (`engine/client/src/ui.rs`), en `EngineState::Menu` : "Héberger", champ adresse + "Rejoindre", "Quitter". Écrit `StartGame(Mode)`. Pas de thème ; hors périmètre : retour au menu après déconnexion.

## Hors périmètre
UI définie en data (fichiers RON).
