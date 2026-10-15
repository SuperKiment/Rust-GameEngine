12
| Lignes | Section |
|---|---|
| 14-17 | Format des données d'un jeu |
| 18-26 | Arborescence |
| 27-35 | `world.ron` |
| 36-56 | Def d'entité |
| 57-66 | Ajouter un composant de jeu |
| 67-81 | Cycle de vie : `EngineState` |
| 82-89 | Erreurs de chargement |
| 90-92 | Hot-reload |
| 93-98 | Lancer hors cargo |

# Format des données d'un jeu

Un jeu = `games/<jeu>/assets/` + des composants/systèmes Rust. Modèle : `games/sample/`.

## Arborescence
```
games/<jeu>/assets/
├── world.ron            # bornes du monde, taille de tuile
└── defs/
    └── <id>.def.ron     # une def par type d'entité ; id = nom du fichier (pas de champ id)
```
Exemple : `defs/goblin.def.ron` -> id `"goblin"`. Les sous-dossiers de `defs/` sont chargés, l'id reste le nom de fichier (doublon = warning, le dernier gagne).

## `world.ron`
```ron
(
  bounds: (min: (-512.0, -512.0), max: (512.0, 512.0)),
  tile_size: 32.0,
)
```
Charge la resource `WorldConfig` (`world_to_tile`, `tile_to_world`, `clamp`).

## Def d'entité
```ron
(
  visual: Circle(radius: 12.0, color: "#33aa33"),
  layer: 1,
  components: {
    "Health": (30),
    "Speed": (80.0),
    "Hostile": (),
  },
)
```
- Tous les champs sont optionnels (`()` est valide). Défauts : `visual: None`, `layer: 0`, `components: {}`. Champ inconnu = erreur.
- `visual` (extension `implicit_some` : écrire la valeur sans `Some(..)`) :
  - `Circle(radius: f32, color: "#rrggbb")`
  - `Rect(size: (w, h), color: "#rrggbb")`
  - `Sprite("chemin/relatif/assets.png")`
- `layer` : profondeur d'affichage (z).
- `components` : map `"NomComposant": valeur`. Nom court (`"Health"`) ou complet (`"mon_jeu::Health"`, obligatoire si deux types ont le même nom court). Valeur = RON du type : struct tuple `(30)`, struct `(hp: 30, max: 40)`, unité `()`.
- `Position` est posée par `spawn_def`, ne la mets pas dans la def.

## Ajouter un composant de jeu
```rust
#[derive(Component, Reflect, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[reflect(Component)]
struct Health(u32);
```
- `#[reflect(Component)]` obligatoire (sinon : "n'est pas un composant").
- `app.register_type::<T>()` **non nécessaire** dans un jeu : la feature Bevy `reflect_auto_register` est active via `engine` (vérifié : un type `#[derive(Reflect)]` est dans `AppTypeRegistry` dès `App::new()`). Nécessaire seulement dans `engine_common` seul (ses tests), où la feature est absente.
- `Serialize/Deserialize` : requis dès que le composant est répliqué (M3), pas pour le chargement des defs.

## Cycle de vie : `EngineState`
`Loading` (chargement de `defs/` + `world.ron`) puis `Ready`. Spawner uniquement une fois `Ready`, côté serveur :
```rust
use engine::prelude::*;

fn spawn_world(mut commands: Commands) {
    commands.spawn_def("goblin", Vec2::new(64.0, 0.0));
}

app.add_systems(OnEnter(EngineState::Ready), spawn_world);
```
- `spawn_def(id, pos)` crée `(DefId(id), Position(pos))` + une copie des composants de la def, et renvoie les `EntityCommands` (ajoute d'autres composants avec `.insert(..)`).
- Id inconnu ou def non chargée : `error!` avec l'id, l'entité reste avec `DefId` + `Position`.
- Serveur uniquement (serveur autoritaire) ; le client reçoit `DefId` + l'état dynamique.

## Erreurs de chargement
Le moteur reste en `Loading` et logue (`error!`) le fichier fautif et la cause, par exemple :
- `composant `Nope` inconnu : type non enregistré ...`
- `composant `Health` : ...` (valeur RON invalide pour ce type)
- `` `X` n'est pas un composant : ajoute `#[reflect(Component)]` ``
- `composant `X` ambigu ... utilise le nom complet`
- `chargement échoué, reste en Loading : defs/ : ...` / `world.ron : ...`

## Hot-reload
En dev, modifier un `.def.ron` ou `world.ron` recharge la def / `WorldConfig` à chaud (feature `file_watcher`). Les entités déjà spawnées gardent leurs anciennes valeurs ; seuls les prochains `spawn_def` utilisent la nouvelle def.

## Lancer hors cargo
Hors `cargo run`, Bevy cherche `assets/` à côté de l'exécutable. Pour pointer ailleurs :
```sh
BEVY_ASSET_ROOT=/chemin/ABSOLU/vers/games/<jeu> ./<jeu> host
```
Le chemin doit être **absolu** : relatif + `file_watcher` = panic.
