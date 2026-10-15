# Index de la documentation

Point de départ. Lis l'index d'un doc avec `./mdx fichier.md`, puis seulement la plage utile (`./mdx fichier.md A B`).

## Règles de travail
| Doc | Contenu | Quand le lire |
|---|---|---|
| [CLAUDE.md](CLAUDE.md) | workflow, décisions, agents, build, règles game dev | chargé automatiquement |

## Suivi des tâches (GitHub)
- Étapes : milestones `M0`..`M5`. Tâche en cours : l'issue (+ ses commentaires). Tâches terminées : PR mergées.
- Labels et milestones : `.github/labels.tsv`, `.github/milestones.tsv`.

## Moteur (`docs/`)
| Doc | Contenu | Quand le lire | Statut |
|---|---|---|---|
| [docs/architecture.md](docs/architecture.md) | crates, modes host/server/client, flux, décisions D1-D4 | avant de toucher `engine/` | M1 réel, reste = cible |
| [docs/data-format.md](docs/data-format.md) | format RON des defs, `WorldConfig`, enregistrer un composant | avant d'ajouter une entité ou un composant | M2 |
| `docs/networking.md` | réplication, inputs, cycle de vie d'un joueur | avant de toucher au réseau ou d'ajouter un input | à créer (M3) |
| `docs/ui.md` | créer un écran, helpers HUD | avant de créer de l'UI | à créer (M4) |
| `docs/game-guide.md` | créer un jeu de A à Z sur le moteur | **avant de générer un jeu** | à créer (M5) |

## Jeux (`games/`)
| Jeu | Contenu | Statut |
|---|---|---|
| `games/sample` | jeu de référence : déplacement, ramassage, score multi | squelette (M1), jeu en M5 |

## Outillage
| Fichier | Rôle |
|---|---|
| `mdx` | lecture des docs indexés |
| `scripts/reindex.sh` | recalcul de l'index d'un `.md` |
| `.github/workflows/repo-meta.yml` | synchro labels + milestones |
| `.github/workflows/claude-review.yml` | review Claude automatique des PR |
| `.claude/hooks/reindex-md.sh` | réindexation auto après Edit/Write (hook PostToolUse, `.claude/settings.json`) |
