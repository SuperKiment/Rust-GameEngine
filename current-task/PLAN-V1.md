19
| Lignes | Section |
|---|---|
| 21-24 | Plan V1 — socle du moteur |
| 25-32 | 1. Objectif |
| 33-45 | 2. Architecture cible |
| 46-52 | — Flux |
| 53-57 | — Modes de lancement (un seul binaire par jeu) |
| 58-59 | 3. Briques |
| 60-64 | — World 2D |
| 65-81 | — Data-driven (defs RON) |
| 82-87 | — Réseau |
| 88-90 | — UI |
| 91-103 | 4. Étapes (= milestones) |
| 104-105 | 5. Décisions à trancher |
| 106-115 | — D1. Version Bevy + transport réseau |
| 116-121 | — D2. UI |
| 122-127 | — D3. Composants dans les defs |
| 128-129 | — D4. Collisions / physique |

# Plan V1 — socle du moteur

Statut : **à valider**. Après validation : 1 milestone par étape, 1 issue par tâche, PR + review Claude par issue.

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
- Transport : voir décision D1.
- Connexion : un client connecté → le serveur spawn l'entité joueur (def `player` du jeu) et la lie au client.
- Hors V1 : prédiction client / interpolation (le jeu sera net en LAN, latence visible en distant).

### UI
Voir décision D2. Le moteur fournit : écran de connexion (host / rejoindre), et des helpers pour afficher un composant ou une resource en une ligne (HUD).

## 4. Étapes (= milestones)

| # | Milestone | Contenu | Doc produite |
|---|---|---|---|
| M0 | Outillage repo | CLAUDE.md, INDEX.md, mdx/reindex, CI (fmt, clippy, test), workflow review Claude sur PR, labels, templates d'issue | ce plan |
| M1 | Socle workspace | workspace cargo, crates vides qui compilent, modes host/server/client (fenêtre vide / headless), CI verte | `docs/architecture.md` |
| M2 | World + data | composants world 2D, `WorldConfig`, defs RON par réflexion, `DefRegistry`, `spawn_def`, hot-reload, tests headless | `docs/data-format.md` |
| M3 | Réseau | replicon + transport, `register_game_component`, réplication `DefId`, inputs client → serveur, entité joueur par client, test 2 clients | `docs/networking.md` |
| M4 | Rendu + UI | rendu 2D depuis la def (formes, sprites), caméra qui suit le joueur, UI (D2), écran de connexion, helpers HUD | `docs/ui.md` |
| M5 | Jeu exemple + guide | `games/sample` (top-down : déplacement, ramassage d'objets, score multi en HUD), mesure de sa taille | `docs/game-guide.md` (la doc que l'IA lit pour générer un jeu) |

Chaque milestone : arrêt, je montre, tu testes, tu valides.

## 5. Décisions à trancher

### D1. Version Bevy + transport réseau
Bevy 0.20 est sorti le 2026-10-08. État de l'écosystème au 2026-10-15 :

| Option | Compatibilité | Conséquences |
|---|---|---|
| **A. Bevy 0.20 + replicon 0.45 + bevy_replicon_quinnet 0.21 (QUIC)** (recommandé) | tout publié, + bevy_egui 0.43 | dernière version, pas de migration immédiate. QUIC : fiable, chiffré, un peu plus lourd que UDP brut |
| B. Bevy 0.19 + replicon 0.44 + bevy_replicon_renet 0.20 (UDP) | tout publié | renet plus éprouvé ; migration 0.20 à faire tôt ou tard |

Le transport est derrière replicon : en changer plus tard = remplacer un plugin, pas le code de jeu.

### D2. UI
| Option | Conséquences |
|---|---|
| **A. bevy_egui** (recommandé) | UI immédiate, très peu de code par écran (le moins de tokens), look "outil" stylable via thème. Nouvelle dépendance |
| B. bevy_ui natif + helpers moteur | look jeu, pas de dépendance, mais verbeux : le moteur doit fournir beaucoup de helpers pour que la génération reste courte |

### D3. Composants dans les defs
| Option | Conséquences |
|---|---|
| **A. Réflexion Bevy, nom court** (recommandé) | un composant de jeu déclaré une fois est utilisable dans les data et répliqué. Erreur de nom détectée au chargement, pas à la compilation |
| B. Enum fermé de composants moteur + map libre `String → valeur` | typé à la compilation, mais chaque composant de jeu demande du code de mapping : plus de tokens par jeu |

### D4. Collisions / physique
Pas dans ta liste V1. Proposition : hors moteur en V1 (le jeu exemple fait un test de distance pour le ramassage). Une physique (avian2d) = question à enjeu pour une V2.
