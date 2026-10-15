# Index de la documentation

Point de départ. Lis l'index d'un doc avec `./mdx fichier.md`, puis seulement la plage utile (`./mdx fichier.md A B`).

## Règles de travail
| Doc | Contenu | Quand le lire |
|---|---|---|
| [CLAUDE.md](CLAUDE.md) | workflow, décisions, agents, build, règles game dev | chargé automatiquement |

## Tâche en cours
| Doc | Contenu | Quand le lire |
|---|---|---|
| [current-task/PLAN-V1.md](current-task/PLAN-V1.md) | architecture cible, milestones M0-M5, décisions D1-D4 | avant toute tâche V1 |

## Moteur (`docs/`)
| Doc | Contenu | Quand le lire | Statut |
|---|---|---|---|
| `docs/architecture.md` | crates, modes host/server/client, flux de données | avant de toucher `engine/` | à créer (M1) |
| `docs/data-format.md` | format RON des defs, `WorldConfig`, enregistrer un composant | avant d'ajouter une entité ou un composant | à créer (M2) |
| `docs/networking.md` | réplication, inputs, cycle de vie d'un joueur | avant de toucher au réseau ou d'ajouter un input | à créer (M3) |
| `docs/ui.md` | créer un écran, helpers HUD | avant de créer de l'UI | à créer (M4) |
| `docs/game-guide.md` | créer un jeu de A à Z sur le moteur | **avant de générer un jeu** | à créer (M5) |

## Jeux (`games/`)
| Jeu | Contenu | Statut |
|---|---|---|
| `games/sample` | jeu de référence : déplacement, ramassage, score multi | à créer (M5) |

## Historique
`old-tasks/<n°issue>-<slug>/` : plans et suivis des tâches terminées. Ne lire que si une décision passée est en question.

## Outillage
| Fichier | Rôle |
|---|---|
| `mdx` | lecture des docs indexés |
| `scripts/reindex.sh` | recalcul de l'index d'un `.md` |
| `.claude/hooks/reindex-md.sh` | réindexation auto après Edit/Write (hook PostToolUse, `.claude/settings.json`) |
