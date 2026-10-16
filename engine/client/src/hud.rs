//! HUD en une ligne : `app.add_hud::<C>(..)` / `app.add_hud_resource::<R>(..)`.
//! N'utilise pas egui (le dessin est dans `UiPlugin`) : utilisable dans tous les modes.

use bevy::prelude::*;
use engine_common::LocalPlayer;

/// Lignes du HUD, dans l'ordre d'enregistrement. Ligne vide = rien à afficher.
#[derive(Resource, Default, Debug)]
pub struct HudLines(pub Vec<String>);

impl HudLines {
    /// Réserve une ligne vide et renvoie son index.
    fn reserve(world: &mut World) -> usize {
        let mut lines = world.get_resource_or_init::<HudLines>();
        lines.0.push(String::new());
        lines.0.len() - 1
    }

    /// N'écrit que si la ligne change (évite de marquer la resource modifiée à chaque frame).
    fn set(&mut self, index: usize, text: String) {
        if self.0[index] != text {
            self.0[index] = text;
        }
    }
}

pub trait HudExt {
    /// Une ligne de HUD pour le composant `C` du joueur local. Vide tant que le joueur local
    /// n'existe pas (ou sur un serveur dédié). `C` doit être répliqué pour être vu côté client.
    fn add_hud<C: Component>(
        &mut self,
        format: impl Fn(&C) -> String + Send + Sync + 'static,
    ) -> &mut Self;

    /// Une ligne de HUD pour la resource `R`. Vide tant que la resource n'existe pas.
    fn add_hud_resource<R: Resource>(
        &mut self,
        format: impl Fn(&R) -> String + Send + Sync + 'static,
    ) -> &mut Self;
}

impl HudExt for App {
    fn add_hud<C: Component>(
        &mut self,
        format: impl Fn(&C) -> String + Send + Sync + 'static,
    ) -> &mut Self {
        let index = HudLines::reserve(self.world_mut());
        self.add_systems(
            Update,
            move |query: Query<&C, With<LocalPlayer>>, mut lines: ResMut<HudLines>| {
                let text = query.single().map(&format).unwrap_or_default();
                lines.set(index, text);
            },
        )
    }

    fn add_hud_resource<R: Resource>(
        &mut self,
        format: impl Fn(&R) -> String + Send + Sync + 'static,
    ) -> &mut Self {
        let index = HudLines::reserve(self.world_mut());
        self.add_systems(
            Update,
            move |res: Option<Res<R>>, mut lines: ResMut<HudLines>| {
                lines.set(index, res.map(|r| format(&r)).unwrap_or_default());
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Component)]
    struct Health(u32);

    #[derive(Resource)]
    struct Score(u32);

    fn lines(app: &App) -> Vec<String> {
        app.world().resource::<HudLines>().0.clone()
    }

    #[test]
    fn component_only_for_local_player() {
        let mut app = App::new();
        app.add_hud::<Health>(|h| format!("PV : {}", h.0));
        app.world_mut().spawn(Health(5)); // autre joueur
        app.update();
        assert_eq!(lines(&app), [""]);
        app.world_mut().spawn((Health(30), LocalPlayer));
        app.update();
        assert_eq!(lines(&app), ["PV : 30"]);
    }

    #[test]
    fn resource_line() {
        let mut app = App::new();
        app.add_hud_resource::<Score>(|s| format!("Score : {}", s.0));
        app.update();
        assert_eq!(lines(&app), [""]);
        app.insert_resource(Score(7));
        app.update();
        assert_eq!(lines(&app), ["Score : 7"]);
    }

    #[test]
    fn lines_follow_registration_order() {
        let mut app = App::new();
        app.add_hud_resource::<Score>(|s| format!("S{}", s.0))
            .add_hud::<Health>(|h| format!("H{}", h.0));
        app.insert_resource(Score(1));
        app.world_mut().spawn((Health(2), LocalPlayer));
        app.update();
        assert_eq!(lines(&app), ["S1", "H2"]);
    }

    #[test]
    fn server_without_local_player_does_not_panic() {
        let mut app = App::new();
        app.add_hud::<Health>(|h| h.0.to_string());
        app.world_mut().spawn(Health(1));
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(lines(&app), [""]);
    }
}
