14
| Lignes | Section |
|---|---|
| 16-19 | Architecture du moteur |
| 20-27 | 1. Objectif |
| 28-44 | 2. Architecture cible |
| 45-52 | — Flux |
| 53-63 | — Modes de lancement (un seul binaire par jeu) |
| 64-65 | 3. Briques |
| 66-71 | — World 2D |
| 72-89 | — Data-driven (defs RON) |
| 90-100 | — Réseau |
| 101-103 | — UI |
| 104-113 | 4. Décisions (validées 2026-10-15) |

# Architecture du moteur

Statut : **M1 : socle réel ; M3 : réseau et réplication des defs en cours ; M4 : rendu 2D des defs + caméra faits (inputs/UI = cible) ; le reste = cible**. Mise à jour à chaque milestone pour refléter le code réel. Suivi des étapes : milestones M0-M5 sur GitHub.

## 1. Objectif
Un moteur 2D minimal sur lequel un jeu se génère avec un minimum de tokens :
- le jeu déclare ses entités en data (RON) et n'écrit que ses systèmes de gameplay ;
- le multijoueur est le mode par défaut (le solo = un host local) ;
- l'UI se crée en quelques lignes.

Critère de réussite V1 : le jeu exemple tient en peu de code (cible < 300 lignes Rust hors data) et tourne en solo et à 2 clients sans code spécifique au réseau.

## 2. Architecture cible

```
Cargo.toml            workspace, versions pinées dans [workspace.dependencies]
engine/
  common/             engine_common (bevy: std) : Mode, puis world 2D, defs data, protocole
  server/             engine_server (bevy: default_app, std, multi_threaded) : ServerPlugin, headless, ne tire PAS bevy_render
  client/             engine_client (bevy: 2d) : ClientPlugin (caméra 2D), puis rendu, inputs, UI
  src/                engine : façade, EnginePlugin + prelude (bevy: 2d)
games/
  sample/             jeu exemple (bin) : dépend uniquement de `engine` ; assets/*.ron + src/ (gameplay)
```

- `EnginePlugin { title, mode }` : `from_args(title)` parse `std::env::args().skip(1)` (erreur : usage sur stderr, exit 2). `build` insère la resource `Mode` puis ajoute les plugins du mode (ci-dessous).
- `engine::prelude` = `bevy::prelude::*` + `EnginePlugin` + `Mode` ; `engine::bevy` ré-exporté. Un jeu ne dépend que de `engine`.
- `main.rs` d'un jeu : `App::new().add_plugins(EnginePlugin::from_args("Sample")).run();`

### Flux
1. Serveur et client chargent les mêmes defs (`common`).
2. Le serveur spawn une entité depuis une def : `spawn_def("goblin", pos)` (ajoute `Replicated`). Le réseau n'est ouvert qu'à `EngineState::Ready` : les defs sont chargées avant toute réplication.
3. Replicon réplique `DefId`, `Position`, `Rotation` + les composants de jeu enregistrés avec `register_game_component`.
4. Le client reçoit `DefId`, retrouve les composants statiques dans sa copie des defs (observer de `DefsPlugin`, valeur répliquée prioritaire), puis affiche : un observer d'`engine_client` (`DefRenderPlugin`) ajoute le visuel de la def (`Mesh2d` + `ColorMaterial`, ou `Sprite`) et un `Transform` avec `z = layer`. Même code pour une entité répliquée (client) et locale (host).
   Chaque frame, avant la propagation des transforms (`PostUpdate`, avant `TransformSystems::Propagate`), `Position`/`Rotation` sont copiés dans `Transform` (xy / rotation z) et la caméra 2D suit le `LocalPlayer`. Pas de prédiction ni d'interpolation.
5. Le client envoie des inputs (messages client → serveur), le serveur les applique.

### Modes de lancement (un seul binaire par jeu)
| Commande | Plugins |
|---|---|
| `cargo run -p sample -- host [port]` (défaut) | `DefaultPlugins` (fenêtre) + `NetPlugin` + `ServerPlugin` + `ClientPlugin`. Écoute sur `0.0.0.0:<port>`. Solo = host sans autre joueur. |
| `cargo run -p sample -- server [port]` | `MinimalPlugins` (boucle 64 Hz = pas de `FixedUpdate`) + `LogPlugin` + `NetPlugin` + `ServerPlugin`, sans fenêtre. Écoute sur `0.0.0.0:<port>`. |
| `cargo run -p sample -- client <ip[:port]>` | `DefaultPlugins` + `NetPlugin` + `ClientPlugin`. `<ip>` obligatoire (`localhost` accepté, IPv4 uniquement). |

Port par défaut : `DEFAULT_PORT` = 5000 (`engine/common/src/net.rs`), pris quand `[port]` ou `:port` est absent. Port occupé : le `server` quitte avec une erreur, le `host` log l'erreur et reste jouable en solo (sans accueillir de client).

Limite connue : un seul binaire par jeu, donc le mode `server` d'un jeu lie quand même `bevy_render` (seul `engine_server` en est exempt). Serveur sans rendu = features cargo sur `engine` ou binaire serveur séparé : question à enjeu, non traitée en V1.

## 3. Briques

### World 2D
- Composants répliqués `Position(Vec2)`, `Rotation(f32)`, `Velocity(Vec2)` dans `common` ; le client synchronise `Position`/`Rotation` → `Transform` des entités qui ont un visuel (le `Transform` Bevy n'est pas répliqué : trop lourd). Les entités sans visuel n'ont pas de `Transform`.
- `WorldConfig` en RON : bornes du monde, taille de tuile (helpers grille ↔ monde).
- Profondeur d'affichage : `layer` dans la def → `z` du `Transform` (posé à l'ajout du visuel, la sync ne touche que xy). Le rendu vit uniquement dans `engine_client` (le serveur dédié ne tire pas `bevy_render`).
- Caméra : `Camera2d` spawnée au démarrage, copie le xy de la `Position` du `LocalPlayer` chaque frame, sans lissage.

### Data-driven (defs RON)
Def d'entité = prototype `assets/defs/<id>.def.ron` ; **l'id est le nom du fichier** (pas de champ `id`). Les composants sont résolus par **réflexion Bevy** (nom court ou complet) :
```ron
// games/sample/assets/defs/goblin.def.ron  ->  id "goblin"
(
  visual: Circle(radius: 12.0, color: "#3a3"),
  layer: 1,
  components: {
    "Health": (30),
    "Speed": (80.0),
  },
)
```
- Côté jeu : `#[derive(Component, Reflect, Serialize, Deserialize)] #[reflect(Component)] struct Health(u32);` : chargeable depuis les data sans autre ligne (`reflect_auto_register`) ; la réplication (M3) aura son enregistrement dédié.
- `DefRegistry` (resource) : id -> handle, via l'asset server de Bevy, hot-reload en dev. `EngineState::Loading -> Ready` quand tout est chargé.
- Serveur : `commands.spawn_def("goblin", pos)` spawn `DefId` + `Position` + copie des composants de la def.
- Format complet, erreurs, lancement : [data-format.md](data-format.md).

### Réseau
- `bevy_replicon` : réplication serveur-autoritaire des composants enregistrés.
- Transport : bevy_replicon_quinnet (QUIC) (D1).
- Réel aujourd'hui : `NetPlugin` (`engine/common/src/net.rs` = `RepliconPlugins` + `RepliconQuinnetPlugins`), ajouté une seule fois par `EnginePlugin` dans les 3 modes. `ServerPlugin` ouvre l'endpoint (host/server), `ClientPlugin` ouvre la connexion (client), tous deux à `OnEnter(EngineState::Ready)`. Le host n'ouvre aucune connexion : son joueur local est `ClientId::Server`. Connexions et déconnexions loggées côté serveur. `engine_common` ré-exporte `bevy_replicon`, `bevy_replicon_quinnet` et `bevy_quinnet`.
- Réplication : `app.register_game_component::<T>()` (trait `GameComponentExt`, dans le prelude) = `register_type` + `replicate`. `ProtocolPlugin` (inclus dans `NetPlugin`, séparable pour tester sans quinnet) enregistre `DefId`, `Position`, `Rotation`. `Velocity` n'est pas répliqué (pas de prédiction en V1). Côté client, les composants de def absents d'une entité `Remote` sont insérés depuis la copie locale des defs.
- Sécurité : certificat serveur auto-signé, jamais vérifié par le client (`SkipVerification`). LAN/dev uniquement : pas d'authentification du serveur (attaque de l'homme du milieu possible), à revoir avant tout jeu en ligne public.
- Tests : `engine/tests/net.rs` (serveur + client réels sur localhost, port choisi par l'OS) ; `engine/common/tests/replication.rs` (réplication des defs via le backend de test replicon, sans réseau).
- Joueur par client (réel) : un client autorisé → le serveur spawn l'entité joueur (def `player` du jeu, `Owner` = client) et lui envoie `OwnPlayer` ; le client pose `LocalPlayer`. Despawn à la déconnexion. Le host a son propre joueur (`ClientId::Server`), un serveur dédié aucun.
- Inputs (réel) : `app.register_input::<I>()` (trait `GameInputExt`) ; le client écrit `MessageWriter<I>`, le serveur lit `I` comme composant du joueur. Détails, exemple et limites : [networking.md](networking.md).
- Hors V1 : prédiction client / interpolation (le jeu sera net en LAN, latence visible en distant).

### UI
bevy_egui (D2). Le moteur fournit : écran de connexion (host / rejoindre), et des helpers pour afficher un composant ou une resource en une ligne (HUD).

## 4. Décisions (validées 2026-10-15)

| # | Sujet | Choix | Écarté | Raison / conséquence |
|---|---|---|---|---|
| D1 | Version + transport | Bevy 0.20, bevy_replicon 0.45, bevy_replicon_quinnet 0.21 (QUIC) | Bevy 0.19 + renet (UDP) | tout l'écosystème utile publié en 0.20 ; transport remplaçable derrière replicon sans toucher au code de jeu |
| D2 | UI | bevy_egui 0.43 | bevy_ui natif + helpers | le moins de code par écran ; look "outil", stylable par thème |
| D3 | Composants dans les defs | réflexion Bevy, nom court (`"Health": (30)`) | enum fermé + map libre | un composant de jeu = 1 ligne d'enregistrement, utilisable en data et répliqué ; erreur de nom détectée au chargement, pas à la compilation |
| D4 | Collisions / physique | hors moteur en V1 (test de distance dans le jeu) | avian2d | une physique = question à enjeu pour une V2 |

Hors V1 : prédiction client / interpolation.
