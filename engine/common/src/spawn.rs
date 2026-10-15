//! Spawn d'une entité depuis une def : `commands.spawn_def("goblin", pos)`.

use crate::{DefRegistry, EntityDef, Position};
use bevy::{ecs::reflect::ReflectComponent, prelude::*};
use serde::{Deserialize, Serialize};

/// Id de la def dont l'entité est issue (nom du fichier `.def.ron`). Répliqué.
#[derive(Component, Reflect, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[reflect(Component)]
pub struct DefId(pub String);

pub trait SpawnDef {
    /// Spawn `(DefId, Position)` puis insère les composants de la def `id`.
    /// Id inconnu ou def non chargée : `error!` et l'entité garde `DefId` + `Position`.
    /// À appeler côté serveur uniquement.
    fn spawn_def(&mut self, id: &str, pos: Vec2) -> EntityCommands<'_>;
}

impl SpawnDef for Commands<'_, '_> {
    fn spawn_def(&mut self, id: &str, pos: Vec2) -> EntityCommands<'_> {
        let mut entity = self.spawn((DefId(id.to_owned()), Position(pos)));
        let (e, id) = (entity.id(), id.to_owned());
        entity.commands().queue(move |world: &mut World| {
            let handle = world.resource::<DefRegistry>().get(&id).cloned();
            let registry = world.resource::<AppTypeRegistry>().clone();
            world.resource_scope(|world, defs: Mut<Assets<EntityDef>>| {
                let Some(def) = handle.and_then(|h| defs.get(&h)) else {
                    error!("spawn_def : def `{id}` inconnue ou non chargée");
                    return;
                };
                let registry = registry.read();
                let mut entity = world.entity_mut(e);
                for value in &def.components {
                    let reflect = value
                        .get_represented_type_info()
                        .and_then(|info| registry.get(info.type_id()))
                        .and_then(|r| r.data::<ReflectComponent>());
                    match reflect {
                        Some(reflect) => reflect.insert(&mut entity, value.as_ref(), &registry),
                        None => error!("spawn_def `{id}` : composant non enregistré"),
                    }
                }
            });
        });
        entity
    }
}
