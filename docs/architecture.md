14
| Lignes | Section |
|---|---|
| 16-19 | Architecture du moteur |
| 20-27 | 1. Objectif |
| 28-40 | 2. Architecture cible |
| 41-47 | — Flux |
| 48-52 | — Modes de lancement (un seul binaire par jeu) |
| 53-54 | 3. Briques |
| 55-59 | — World 2D |
| 60-76 | — Data-driven (defs RON) |
| 77-82 | — Réseau |
| 83-85 | — UI |
| 86-95 | 4. Décisions (validées 2026-10-15) |

# Architecture du moteur

Statut : **cible V1** (rien n'est encore codé). Mise à jour à chaque milestone pour refléter le code réel. Suivi des étapes : milestones M0-M5 sur GitHub.

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
  common/             engine_common : world 2D, defs data, registre de composants, protocole (inputs)
  server/             engine_server : transport serveur, spawn depuis les defs, simulation autoritaire, headless
  client/             engine_client : transport client, rendu des entités, caméra, inputs, UI
  (engine/)           engine : crate façade, GamePlugin + modes host / server / client
games/
  sample/             jeu exemple : assets/*.ron + src/ (gameplay uniquement)
```

### Flux
1. Serveur et client chargent les mêmes defs (`common`).
2. Le serveur spawn une entité depuis une def : `spawn_def("goblin", pos)`.
3. Replicon réplique `DefId` + composants dynamiques marqués.
4. Le client reçoit `DefId`, retrouve visuel et données statiques dans sa copie des defs, affiche.
5. Le client envoie des inputs (messages client → serveur), le serveur les applique.

### Modes de lancement (un seul binaire par jeu)
- `host` (défaut) : serveur + client dans le même process. Solo = host sans autre joueur.
- `server` : headless, sans fenêtre.
- `client <addr>` : rejoint un serveur.

## 3. Briques

### World 2D
- Composants répliqués `Position(Vec2)`, `Rotation(f32)`, `Velocity(Vec2)` dans `common` ; le client synchronise `Position` → `Transform` (le `Transform` Bevy n'est pas répliqué : trop lourd).
- `WorldConfig` en RON : bornes du monde, taille de tuile (helpers grille ↔ monde).
- Profondeur d'affichage : `layer` dans la def → `z` du `Transform`.

### Data-driven (defs RON)
Def d'entité = prototype. Les composants sont résolus par **réflexion Bevy** via leur nom court :
```ron
// games/sample/assets/defs/goblin.def.ron
(
  id: "goblin",
  visual: Shape(circle: 12.0, color: "#3a3"),
  layer: 1,
  components: {
    "Health": (30),
    "Speed": (80.0),
  },
)
```
- Côté jeu : `#[derive(Component, Reflect, Serialize, Deserialize)] #[reflect(Component)] struct Health(u32);` puis **une ligne** `app.register_game_component::<Health>()` = chargeable depuis les data + répliqué.
- `DefRegistry` (resource) chargé via l'asset server de Bevy, hot-reload en dev.

### Réseau
- `bevy_replicon` : réplication serveur-autoritaire des composants enregistrés.
- Transport : bevy_replicon_quinnet (QUIC) (D1).
- Connexion : un client connecté → le serveur spawn l'entité joueur (def `player` du jeu) et la lie au client.
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
