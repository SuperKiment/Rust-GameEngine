//! Defs data-driven : `assets/defs/<id>.def.ron` (prototypes d'entités) et `assets/world.ron`.
//!
//! Les composants d'une def sont résolus par nom (court ou complet) dans `AppTypeRegistry`
//! et désérialisés par réflexion : un composant de jeu = `#[derive(Component, Reflect)]`
//! + `#[reflect(Component)]` + `app.register_type::<T>()`.

use std::{fmt, sync::Arc};

use crate::world::WorldConfig;
use bevy::{
    asset::{AssetLoader, LoadContext, LoadState, LoadedFolder, io::Reader},
    ecs::{error::BevyError, reflect::ReflectComponent},
    platform::collections::HashMap,
    prelude::*,
    reflect::{
        ReflectFromReflect, TypeRegistration, TypeRegistry, TypeRegistryArc,
        serde::TypedReflectDeserializer,
    },
};
use ron::extensions::Extensions;
use serde::{
    Deserialize,
    de::{DeserializeSeed, Error as _, MapAccess, Visitor},
};

/// Rendu d'une def (le rendu arrive en M4).
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub enum Visual {
    Circle { radius: f32, color: String },
    Rect { size: Vec2, color: String },
    Sprite(String),
}

/// Prototype d'entité chargé depuis `<id>.def.ron`. L'id est le nom du fichier.
#[derive(Asset, TypePath)]
pub struct EntityDef {
    pub visual: Option<Visual>,
    pub layer: i32,
    /// Valeurs des composants (concrètes si le type a `ReflectFromReflect`).
    pub components: Vec<Box<dyn PartialReflect>>,
}

/// `Loading` tant que `defs/` et `world.ron` ne sont pas chargés, puis `Ready`.
#[derive(States, Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EngineState {
    #[default]
    Loading,
    Ready,
}

/// Id de def -> handle. Reconstruit quand le dossier `defs/` change.
#[derive(Resource, Default)]
pub struct DefRegistry(HashMap<String, Handle<EntityDef>>);

impl DefRegistry {
    pub fn get(&self, id: &str) -> Option<&Handle<EntityDef>> {
        self.0.get(id)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

// --- Parsing RON -----------------------------------------------------------------------------

fn ron_options() -> ron::Options {
    ron::Options::default().with_default_extension(Extensions::IMPLICIT_SOME)
}

/// Nom court ou complet -> registration d'un composant.
fn resolve<'a>(registry: &'a TypeRegistry, name: &str) -> Result<&'a TypeRegistration, String> {
    let registration = registry
        .get_with_type_path(name)
        .or_else(|| registry.get_with_short_type_path(name));
    match registration {
        None if registry.is_ambiguous(name) => Err(format!(
            "composant `{name}` ambigu : plusieurs types portent ce nom, utilise le nom complet"
        )),
        None => Err(format!(
            "composant `{name}` inconnu : type non enregistré (`app.register_type`)"
        )),
        Some(r) if r.data::<ReflectComponent>().is_none() => Err(format!(
            "`{name}` n'est pas un composant : ajoute `#[reflect(Component)]`"
        )),
        Some(r) => Ok(r),
    }
}

struct ComponentsSeed<'a>(&'a TypeRegistry);

impl<'de> DeserializeSeed<'de> for ComponentsSeed<'_> {
    type Value = Vec<Box<dyn PartialReflect>>;

    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for ComponentsSeed<'_> {
    type Value = Vec<Box<dyn PartialReflect>>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("map { \"NomComposant\": valeur }")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut components = Vec::new();
        while let Some(name) = map.next_key::<String>()? {
            let registration = resolve(self.0, &name).map_err(A::Error::custom)?;
            let value = map
                .next_value_seed(TypedReflectDeserializer::new(registration, self.0))
                .map_err(|e| A::Error::custom(format!("composant `{name}` : {e}")))?;
            // Valeur concrète si possible (sinon `DynamicStruct` & co).
            let value = registration
                .data::<ReflectFromReflect>()
                .and_then(|fr| fr.from_reflect(value.as_partial_reflect()))
                .map(PartialReflect::into_partial_reflect)
                .unwrap_or(value);
            components.push(value);
        }
        Ok(components)
    }
}

const FIELDS: &[&str] = &["visual", "layer", "components"];

struct DefSeed<'a>(&'a TypeRegistry);

impl<'de> DeserializeSeed<'de> for DefSeed<'_> {
    type Value = EntityDef;

    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<EntityDef, D::Error> {
        d.deserialize_struct("EntityDef", FIELDS, self)
    }
}

impl<'de> Visitor<'de> for DefSeed<'_> {
    type Value = EntityDef;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("(visual: .., layer: .., components: { .. })")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<EntityDef, A::Error> {
        let mut def = EntityDef {
            visual: None,
            layer: 0,
            components: Vec::new(),
        };
        while let Some(field) = map.next_key::<String>()? {
            match field.as_str() {
                "visual" => def.visual = map.next_value()?,
                "layer" => def.layer = map.next_value()?,
                "components" => def.components = map.next_value_seed(ComponentsSeed(self.0))?,
                _ => return Err(A::Error::unknown_field(&field, FIELDS)),
            }
        }
        Ok(def)
    }
}

fn parse_def(bytes: &[u8], registry: &TypeRegistry) -> Result<EntityDef, ron::error::SpannedError> {
    ron_options().from_bytes_seed(bytes, DefSeed(registry))
}

// --- Loaders ---------------------------------------------------------------------------------

/// Loader de `*.def.ron`. Tient le registre de types partagé (comme le loader de scène).
#[derive(TypePath)]
pub struct EntityDefLoader {
    registry: TypeRegistryArc,
}

impl FromWorld for EntityDefLoader {
    fn from_world(world: &mut World) -> Self {
        Self {
            registry: world.resource::<AppTypeRegistry>().0.clone(),
        }
    }
}

impl AssetLoader for EntityDefLoader {
    type Asset = EntityDef;
    type Settings = ();
    type Error = BevyError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<EntityDef, BevyError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(parse_def(&bytes, &self.registry.read())?)
    }

    fn extensions(&self) -> &[&str] {
        &["def.ron"]
    }
}

/// Loader de `world.ron` -> `WorldConfig`.
#[derive(TypePath, Default)]
struct WorldConfigLoader;

impl AssetLoader for WorldConfigLoader {
    type Asset = WorldConfig;
    type Settings = ();
    type Error = BevyError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<WorldConfig, BevyError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(ron_options().from_bytes(&bytes)?)
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

// --- Plugin ----------------------------------------------------------------------------------

#[derive(Resource)]
struct DefHandles {
    folder: Handle<LoadedFolder>,
    world: Handle<WorldConfig>,
}

/// Charge `defs/` + `world.ron`, maintient `DefRegistry` et `WorldConfig`, pilote `EngineState`.
/// Demande `AssetPlugin` et `StatesPlugin` déjà ajoutés.
pub struct DefsPlugin;

impl Plugin for DefsPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<EntityDef>()
            .init_asset::<WorldConfig>()
            .init_asset_loader::<EntityDefLoader>()
            .init_asset_loader::<WorldConfigLoader>()
            .init_state::<EngineState>()
            .init_resource::<DefRegistry>()
            .add_systems(Startup, start_loading)
            .add_systems(
                Update,
                (
                    sync_world_config,
                    sync_registry,
                    check_ready.run_if(in_state(EngineState::Loading)),
                )
                    .chain(),
            )
            .add_systems(OnEnter(EngineState::Ready), |defs: Res<DefRegistry>| {
                info!("defs ready: {}", defs.len());
            });
    }
}

fn start_loading(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(DefHandles {
        folder: server.load_folder("defs"),
        world: server.load("world.ron"),
    });
}

/// `WorldConfig` (resource) suit l'asset `world.ron`, hot-reload compris.
fn sync_world_config(
    mut commands: Commands,
    mut events: MessageReader<AssetEvent<WorldConfig>>,
    handles: Res<DefHandles>,
    configs: Res<Assets<WorldConfig>>,
) {
    let changed = events
        .read()
        .filter(|e| matches!(e, AssetEvent::Added { .. } | AssetEvent::Modified { .. }))
        .count()
        > 0;
    if changed && let Some(config) = configs.get(&handles.world) {
        commands.insert_resource(config.clone());
    }
}

/// `DefRegistry` reconstruit à chaque (re)chargement du dossier `defs/`.
fn sync_registry(
    mut events: MessageReader<AssetEvent<LoadedFolder>>,
    handles: Res<DefHandles>,
    folders: Res<Assets<LoadedFolder>>,
    mut registry: ResMut<DefRegistry>,
) {
    let changed = events
        .read()
        .filter(|e| matches!(e, AssetEvent::Added { .. } | AssetEvent::Modified { .. }))
        .count()
        > 0;
    let Some(folder) = folders.get(&handles.folder).filter(|_| changed) else {
        return;
    };
    registry.0.clear();
    for handle in &folder.handles {
        let Ok(handle) = handle.clone().try_typed::<EntityDef>() else {
            continue;
        };
        let id = handle
            .path()
            .and_then(|p| p.path().file_name()?.to_str()?.strip_suffix(".def.ron"));
        if let Some(id) = id
            && registry.0.insert(id.to_owned(), handle.clone()).is_some()
        {
            warn!("def `{id}` définie plusieurs fois, la dernière gagne");
        }
    }
}

/// `Loading` -> `Ready` quand tout est chargé ; en cas d'échec, log l'erreur et reste en `Loading`.
fn check_ready(
    handles: Res<DefHandles>,
    server: Res<AssetServer>,
    config: Option<Res<WorldConfig>>,
    folders: Res<Assets<LoadedFolder>>,
    mut next: ResMut<NextState<EngineState>>,
    mut failed: Local<Option<Arc<str>>>,
) {
    let states = [
        ("defs/", server.load_state(&handles.folder)),
        ("world.ron", server.load_state(&handles.world)),
    ];
    for (what, load_state) in states {
        if let LoadState::Failed(err) = load_state {
            let msg: Arc<str> = format!("{what} : {err}").into();
            if failed.as_ref() != Some(&msg) {
                error!("chargement échoué, reste en Loading : {msg}");
                *failed = Some(msg);
            }
            return;
        }
    }
    let loaded = server.is_loaded_with_dependencies(&handles.folder)
        && server.is_loaded_with_dependencies(&handles.world);
    // Le registre se reconstruit sur l'event du dossier : on attend qu'il l'ait fait.
    if loaded && config.is_some() && folders.contains(&handles.folder) {
        next.set(EngineState::Ready);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Component, Reflect, Debug, PartialEq)]
    #[reflect(Component)]
    struct Health(u32);

    #[derive(Reflect, Debug)]
    struct NotAComponent(u32);

    fn registry() -> TypeRegistry {
        let mut r = TypeRegistry::new();
        r.register::<Health>();
        r.register::<NotAComponent>();
        r.register::<i32>();
        r
    }

    fn err(src: &str) -> String {
        match parse_def(src.as_bytes(), &registry()) {
            Ok(_) => panic!("devrait échouer"),
            Err(e) => e.to_string(),
        }
    }

    #[test]
    fn parses_full_def() {
        let def = parse_def(
            br##"(visual: Sprite("a.png"), layer: 3, components: { "Health": (7) })"##,
            &registry(),
        )
        .unwrap();
        assert_eq!(def.visual, Some(Visual::Sprite("a.png".into())));
        assert_eq!(def.layer, 3);
        assert_eq!(
            def.components[0].try_downcast_ref::<Health>(),
            Some(&Health(7))
        );
    }

    #[test]
    fn defaults() {
        let def = parse_def(b"()", &registry()).unwrap();
        assert_eq!(def.visual, None);
        assert_eq!(def.layer, 0);
        assert!(def.components.is_empty());
    }

    #[test]
    fn full_type_path_accepted() {
        let path = <Health as TypePath>::type_path();
        let src = format!("(components: {{ \"{path}\": (1) }})");
        assert_eq!(
            parse_def(src.as_bytes(), &registry())
                .unwrap()
                .components
                .len(),
            1
        );
    }

    #[test]
    fn errors_name_the_component() {
        assert!(err(r#"(components: { "Nope": (1) })"#).contains("`Nope` inconnu"));
        assert!(err(r#"(components: { "NotAComponent": (1) })"#).contains("`NotAComponent`"));
        assert!(err(r#"(components: { "Health": ("x") })"#).contains("composant `Health`"));
    }
}
