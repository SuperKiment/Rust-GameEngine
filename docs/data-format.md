12
| Lignes | Section |
|---|---|
| 14-17 | Format des données d'un jeu |
| 18-26 | Arborescence |
| 27-35 | `world.ron` |
| 36-58 | Def d'entité |
| 59-70 | Ajouter un composant de jeu |
| 71-89 | Cycle de vie : `EngineState` |
| 90-98 | Erreurs de chargement |
| 99-101 | Hot-reload |
| 102-107 | Lancer hors cargo |

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
Charge la resource `WorldConfig` (`world_to_tile`, `tile_to_world`, `clamp`). `min > max` sur un axe : erreur de chargement.

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
  - Rendu côté client (`engine_client`) : `Circle`/`Rect` = mesh 2D + `ColorMaterial`, `Sprite` = image chargée par l'asset server (chemin relatif à `assets/`). Aucun code de rendu côté jeu.
  - `color` est validée au chargement de la def (syntaxe `Srgba::hex` de Bevy : `#rgb`, `#rrggbb`, `#rrggbbaa`) : une couleur invalide empêche le chargement.
- `layer` : profondeur d'affichage, devient le `z` du `Transform` de l'entité (plus grand = devant).
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
- Composant répliqué (état dynamique : PV, état d'IA...) : `app.register_game_component::<T>()` = `register_type` + réplication replicon. `Serialize/Deserialize` requis. À appeler dans le plugin du jeu, donc sur le serveur et le client, dans le même ordre.
- Composant purement statique (déduit de la def) : pas répliqué, le client le reconstruit depuis sa copie des defs. Rien à enregistrer (voir `reflect_auto_register` ci-dessus), pas de `Serialize/Deserialize`.
- Moteur : `DefId`, `Position`, `Rotation` sont déjà répliqués. `Velocity` ne l'est pas (pas de prédiction en V1).

## Cycle de vie : `EngineState`
`Loading` (chargement de `defs/` + `world.ron`), puis `Menu` (écran de connexion, tant qu'aucun `Mode` n'est choisi) ou directement `InGame` si le mode est donné en ligne de commande. Le mode choisi au menu est inséré (resource `Mode`) puis l'état passe à `InGame`. Spawner à l'entrée dans `InGame`, côté serveur seulement :
```rust
use engine::prelude::*;

fn spawn_world(mut commands: Commands) {
    commands.spawn_def("goblin", Vec2::new(64.0, 0.0));
}

// Le plugin du jeu est aussi ajouté en mode client : `is_server` (Host ou Server) garde les systèmes serveur.
app.add_systems(OnEnter(EngineState::InGame), spawn_world.run_if(is_server));
```
- `spawn_def(id, pos)` crée `(DefId(id), Position(pos), Replicated)` + une copie des composants de la def, et renvoie les `EntityCommands` (ajoute d'autres composants avec `.insert(..)`).
- Id inconnu ou def non chargée : `error!` avec l'id, l'entité reste avec `DefId` + `Position`.
- Un `"Position"` dans la def est écrasé par `pos`.
- Serveur uniquement (serveur autoritaire) : sans `.run_if(is_server)`, un client spawnerait lui aussi l'entité. Toute entité issue d'une def est répliquée (`Replicated`).
- Client : à l'arrivée d'une entité répliquée avec un `DefId`, `DefsPlugin` insère les composants de la def **absents** de l'entité. Une valeur répliquée prime toujours sur celle de la def, quel que soit l'ordre d'arrivée. Def inconnue : `error!` avec l'id. Le host n'est pas concerné (entités locales).
- Le réseau (écoute serveur, connexion client) s'ouvre à l'entrée dans `InGame` : les defs sont chargées avant l'arrivée des entités.

## Erreurs de chargement
Le moteur reste en `Loading` et logue (`error!`) le fichier fautif et la cause, par exemple :
- `composant `Nope` inconnu : type non enregistré ...`
- `composant `Health` : ...` (valeur RON invalide pour ce type)
- `` `X` n'est pas un composant : ajoute `#[reflect(Component)]` ``
- `composant `X` ambigu ... utilise le nom complet`
- `visual : couleur `rouge` invalide (...), attendu "#rrggbb"`
- `chargement échoué, reste en Loading : defs/ : ...` / `world.ron : ...`

## Hot-reload
En dev, modifier un `.def.ron` ou `world.ron` recharge la def / `WorldConfig` à chaud (feature `file_watcher`). Les entités déjà spawnées gardent leurs anciennes valeurs ; seuls les prochains `spawn_def` utilisent la nouvelle def.

## Lancer hors cargo
Hors `cargo run`, Bevy cherche `assets/` à côté de l'exécutable. Pour pointer ailleurs :
```sh
BEVY_ASSET_ROOT=/chemin/ABSOLU/vers/games/<jeu> ./<jeu> host
```
Le chemin doit être **absolu** : relatif + `file_watcher` = panic.
