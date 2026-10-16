//! Spawn d'une entité depuis une def : `commands.spawn_def("goblin", pos)`.

use crate::{DefRegistry, EntityDef, Position};
use bevy::{ecs::reflect::ReflectComponent, prelude::*};
use bevy_replicon::prelude::{Remote, Replicated};
use serde::{Deserialize, Serialize};

/// Id de la def dont l'entité est issue (nom du fichier `.def.ron`). Répliqué.
#[derive(Component, Reflect, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[reflect(Component)]
pub struct DefId(pub String);

pub trait SpawnDef {
    /// Spawn `(DefId, Position, Replicated)` puis insère les composants de la def `id`.
    /// Toute entité issue d'une def est répliquée. Id inconnu ou def non chargée :
    /// `error!` et l'entité garde `DefId` + `Position`.
    /// À appeler côté serveur uniquement.
    fn spawn_def(&mut self, id: &str, pos: Vec2) -> EntityCommands<'_>;
}

impl SpawnDef for Commands<'_, '_> {
    fn spawn_def(&mut self, id: &str, pos: Vec2) -> EntityCommands<'_> {
        let mut entity = self.spawn((DefId(id.to_owned()), Position(pos), Replicated));
        let (e, id) = (entity.id(), id.to_owned());
        entity.commands().queue(move |world: &mut World| {
            insert_def_components(world, e, &id, true);
            // La position passée à spawn_def prime sur un éventuel "Position" de la def.
            world.entity_mut(e).insert(Position(pos));
        });
        entity
    }
}

/// Insère sur `entity` les composants de la def `id` (copie par réflexion).
/// `overwrite = false` : garde les composants déjà présents (valeur répliquée > valeur de la def).
fn insert_def_components(world: &mut World, entity: Entity, id: &str, overwrite: bool) {
    let handle = world.resource::<DefRegistry>().get(id).cloned();
    let registry = world.resource::<AppTypeRegistry>().clone();
    world.resource_scope(|world, defs: Mut<Assets<EntityDef>>| {
        let Some(def) = handle.and_then(|h| defs.get(&h)) else {
            error!("def `{id}` inconnue ou non chargée");
            return;
        };
        let registry = registry.read();
        let mut entity = world.entity_mut(entity);
        for value in &def.components {
            let reflect = value
                .get_represented_type_info()
                .and_then(|info| registry.get(info.type_id()))
                .and_then(|r| r.data::<ReflectComponent>());
            match reflect {
                Some(reflect) if !overwrite && reflect.contains(&entity) => {}
                Some(reflect) => reflect.insert(&mut entity, value.as_ref(), &registry),
                None => error!("def `{id}` : composant non enregistré"),
            }
        }
    });
}

/// Client : une entité répliquée (`Remote`) qui reçoit un `DefId` retrouve ses composants
/// statiques dans sa copie des defs. En host, les entités sont locales : rien à faire.
pub(crate) fn rebuild_from_def(
    add: On<Add<DefId>>,
    remote: Query<&DefId, With<Remote>>,
    mut commands: Commands,
) {
    let Ok(DefId(id)) = remote.get(add.entity) else {
        return;
    };
    let (entity, id) = (add.entity, id.clone());
    commands.queue(move |world: &mut World| insert_def_components(world, entity, &id, false));
}
