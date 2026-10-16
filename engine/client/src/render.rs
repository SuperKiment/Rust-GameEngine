//! Rendu 2D des entités issues d'une def, sync `Position`/`Rotation` -> `Transform`, caméra.

use bevy::prelude::*;
use engine_common::{DefId, DefRegistry, EntityDef, LocalPlayer, Position, Rotation, Visual};

pub struct DefRenderPlugin;

impl Plugin for DefRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(add_visual).add_systems(
            PostUpdate,
            (sync_transforms, follow_local_player).before(TransformSystems::Propagate),
        );
    }
}

/// Une entité reçoit un `DefId` (répliquée côté client, locale en host) : on lui ajoute le visuel
/// de sa def. Z = `layer`. Def sans `visual` : rien.
#[allow(clippy::too_many_arguments)] // params de système Bevy
fn add_visual(
    add: On<Add<DefId>>,
    ids: Query<&DefId>,
    registry: Res<DefRegistry>,
    defs: Res<Assets<EntityDef>>,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
) {
    let Ok(DefId(id)) = ids.get(add.entity) else {
        return;
    };
    // Def inconnue : déjà signalée par `spawn_def` / `rebuild_from_def`.
    let Some(def) = registry.def(id, &defs) else {
        return;
    };
    let Some(visual) = &def.visual else {
        return;
    };
    let transform = Transform::from_xyz(0.0, 0.0, def.layer as f32);
    let color = Color::from(visual.color().unwrap_or(Srgba::WHITE));
    let mut entity = commands.entity(add.entity);
    match visual {
        Visual::Circle { radius, .. } => entity.try_insert((
            Mesh2d(meshes.add(Circle::new(*radius))),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(color))),
            transform,
        )),
        Visual::Rect { size, .. } => entity.try_insert((
            Mesh2d(meshes.add(Rectangle::from_size(*size))),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(color))),
            transform,
        )),
        Visual::Sprite(path) => {
            entity.try_insert((Sprite::from_image(server.load(path)), transform))
        }
    };
}

type PositionOrRotationChanged = Or<(Changed<Position>, Changed<Rotation>, Added<Transform>)>;

/// `Position` -> xy, `Rotation` -> rotation z. Le z (layer) ne bouge pas.
/// Sans prédiction ni interpolation. N'affecte que les entités qui ont déjà un `Transform`.
fn sync_transforms(
    mut query: Query<(&Position, Option<&Rotation>, &mut Transform), PositionOrRotationChanged>,
) {
    for (position, rotation, mut transform) in &mut query {
        transform.translation.x = position.0.x;
        transform.translation.y = position.0.y;
        if let Some(rotation) = rotation {
            transform.rotation = Quat::from_rotation_z(rotation.0);
        }
    }
}

/// La caméra suit le joueur local (xy), sans lissage.
fn follow_local_player(
    player: Single<&Position, With<LocalPlayer>>,
    mut camera: Single<&mut Transform, With<Camera2d>>,
) {
    camera.translation.x = player.0.x;
    camera.translation.y = player.0.y;
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::AssetPlugin;

    /// App headless (sans GPU) avec des defs `(id, visual, layer)` déjà chargées.
    fn app_with_defs(defs: Vec<(&str, Option<Visual>, i32)>) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<EntityDef>()
            .init_asset::<Mesh>()
            .init_asset::<ColorMaterial>()
            .init_asset::<Image>()
            .init_resource::<DefRegistry>()
            .add_plugins(DefRenderPlugin);
        for (id, visual, layer) in defs {
            let def = EntityDef {
                visual,
                layer,
                components: Vec::new(),
            };
            let handle = app.world_mut().resource_mut::<Assets<EntityDef>>().add(def);
            app.world_mut()
                .resource_mut::<DefRegistry>()
                .insert(id, handle);
        }
        app
    }

    fn circle() -> Visual {
        Visual::Circle {
            radius: 5.0,
            color: "#ff0000".into(),
        }
    }

    #[test]
    fn shape_gets_mesh_and_layer_z() {
        let mut app = app_with_defs(vec![("c", Some(circle()), 3)]);
        let e = app
            .world_mut()
            .spawn((
                DefId("c".into()),
                Position(Vec2::new(1.0, 2.0)),
                Rotation(0.5),
            ))
            .id();
        app.update();
        let entity = app.world().entity(e);
        assert!(entity.contains::<Mesh2d>());
        assert!(entity.contains::<MeshMaterial2d<ColorMaterial>>());
        let t = entity.get::<Transform>().unwrap();
        assert_eq!(t.translation, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(t.rotation, Quat::from_rotation_z(0.5));

        // Position changée : xy suit, z (layer) reste.
        app.world_mut().get_mut::<Position>(e).unwrap().0 = Vec2::new(-4.0, 8.0);
        app.update();
        let t = app.world().get::<Transform>(e).unwrap();
        assert_eq!(t.translation, Vec3::new(-4.0, 8.0, 3.0));
    }

    #[test]
    fn rect_and_sprite() {
        let rect = Visual::Rect {
            size: Vec2::new(4.0, 2.0),
            color: "#00ff00".into(),
        };
        let mut app = app_with_defs(vec![
            ("r", Some(rect), 0),
            ("s", Some(Visual::Sprite("a.png".into())), -1),
        ]);
        let r = app
            .world_mut()
            .spawn((DefId("r".into()), Position::default()))
            .id();
        let s = app
            .world_mut()
            .spawn((DefId("s".into()), Position::default()))
            .id();
        app.update();
        assert!(app.world().entity(r).contains::<Mesh2d>());
        assert!(app.world().entity(s).contains::<Sprite>());
        assert_eq!(app.world().get::<Transform>(s).unwrap().translation.z, -1.0);
    }

    #[test]
    fn no_visual_or_unknown_def_gets_nothing() {
        let mut app = app_with_defs(vec![("none", None, 2)]);
        let none = app
            .world_mut()
            .spawn((DefId("none".into()), Position::default()))
            .id();
        let unknown = app
            .world_mut()
            .spawn((DefId("nope".into()), Position::default()))
            .id();
        app.update();
        assert!(!app.world().entity(none).contains::<Transform>());
        assert!(!app.world().entity(unknown).contains::<Transform>());
    }

    #[test]
    fn transform_without_position_untouched() {
        let mut app = app_with_defs(vec![]);
        let e = app
            .world_mut()
            .spawn(Transform::from_xyz(7.0, 8.0, 9.0))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<Transform>(e).unwrap().translation,
            Vec3::new(7.0, 8.0, 9.0)
        );
    }

    #[test]
    fn camera_follows_local_player() {
        let mut app = app_with_defs(vec![]);
        let camera = app.world_mut().spawn(Camera2d).id();
        let player = app
            .world_mut()
            .spawn((Position(Vec2::new(3.0, 4.0)), LocalPlayer))
            .id();
        app.update();
        let t = app.world().get::<Transform>(camera).unwrap();
        assert_eq!((t.translation.x, t.translation.y), (3.0, 4.0));
        app.world_mut().get_mut::<Position>(player).unwrap().0 = Vec2::new(-1.0, 0.5);
        app.update();
        let t = app.world().get::<Transform>(camera).unwrap();
        assert_eq!((t.translation.x, t.translation.y), (-1.0, 0.5));
    }
}
