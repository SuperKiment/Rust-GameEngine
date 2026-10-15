# Priorité
En cas de conflit entre ce fichier et un skill (superpowers, etc.), ce fichier prime.

# Projet
Game engine Rust (Bevy, 2D) servant de socle à des jeux générés par IA. But : générer un jeu = écrire uniquement ce qui lui est spécifique (fichiers data + systèmes de gameplay). Tout ce qui est générique vit dans `engine/`.
Point d'entrée de la doc : [INDEX.md](INDEX.md). Architecture et décisions : `docs/architecture.md`. Étapes : milestones GitHub.

# Utilisation des agents
- **Implémentation** : agents `sonnet` en effort medium par défaut, high si la tâche est complexe. Tâches bien scopées et définies : l'orchestrateur crée des tâches simples et rapides, récupère le résumé, donne la tâche suivante.
- **Recherche** (usage d'une fonction, implémentation de référence) : `sonnet` effort medium. Pas besoin d'Opus pour chercher.
- Les agents n'héritent pas des skills ni de ce fichier : écris dans chaque prompt de délégation "utilise le skill caveman", "lis les `.md` avec `./mdx`", les chemins utiles et les commandes de "Build, test, lint" si besoin.
- **API Bevy** : Bevy casse son API à chaque version et la mémoire d'un modèle mélange les versions. Écris dans chaque prompt : "vérifie toute API Bevy/replicon dans les sources de la version pinée (`~/.cargo/registry/src/*/<crate>-<version>/`), pas de mémoire". Versions : `[workspace.dependencies]` du `Cargo.toml` racine.
- Les agents n'écrivent rien sur GitHub (issues, PR, commentaires) : c'est le rôle de l'orchestrateur. Ils renvoient un résumé détaillé : fichiers modifiés, build/lint lancés et résultat, points d'attention.
- Les agents de recherche ne renvoient que les informations précises demandées (fichier:ligne, signature), pas de résumé de travail.

# Index des documents .md

## Périmètre
- **Indexés** : les `.md` de la racine, `docs/`, `games/*/docs/` de 80 lignes ou plus, ou qui ont déjà un index.
- **Jamais indexés** : `CLAUDE.md`, `INDEX.md`, `README.md`, tout `.md` hors des dossiers ci-dessus (`.github/`, `.claude/`, `target/`...), et tout `.md` de moins de 80 lignes sans index.
- Ne lance pas `scripts/reindex.sh` sur autre chose : il modifie le fichier (entier + tableau en tête, casse le rendu GitHub/IDE).

## Écriture
Ne calcule jamais l'index à la main. Un hook PostToolUse (`.claude/hooks/reindex-md.sh`) réindexe automatiquement après chaque Edit/Write d'un `.md` du périmètre. Il recalcule toutes les plages : après une modification, relis le fichier avant de le rééditer.
Le hook ne couvre pas les modifs par `Bash` (sed, etc.) ni les éditions manuelles dans l'IDE. Dans ces cas : `bash scripts/reindex.sh chemin/fichier.md`.

## Lecture
Un seul outil, `./mdx` (depuis la racine du repo) :
- `./mdx fichier.md` : affiche l'index.
- `./mdx fichier.md A B` : affiche les lignes A à B (plage de l'index).

Ne lis jamais un document entier s'il a un index. Si `./mdx fichier.md` répond "Pas d'index" et que le fichier est dans le périmètre : `scripts/reindex.sh`. Sinon, lis-le directement.

## Format
```md
5
| Lignes | Section |
|---|---|
| 6-20 | 1. Conventions Composition API |
| 21-35 | — Règles & particularités |
```
Ligne 1 = dernière ligne de l'index (ligne 1 incluse). Chaque ligne du tableau = plage de la section. `###` préfixé par `— `.

# Obligations

## Décisions
Je tranche les questions à enjeu. Avant de me faire trancher, expose circonstances, possibilités et conséquences.
**Question à enjeu** = au moins un de :
- changement du format des fichiers data (casse les jeux existants) ;
- changement de composant répliqué, de message réseau ou de protocole ;
- montée de version de Bevy ou d'une crate de l'écosystème Bevy ;
- nouvelle dépendance ;
- ajout dans `engine/` d'un besoin découvert en codant un jeu ;
- suppression de code existant ;
- choix entre approches d'architecture ;
- plus de 100 lignes modifiées.

## Workflow
- **Grande tâche** = plus de 3 fichiers modifiés, ou client + serveur, ou engine + jeu.
- Grande tâche : planning ; je valide ; tâche 1 ; je valide ; tâche 2 ; je valide ; etc. Arrêt à chaque étape pour me montrer l'avancée et que je teste. Doc mise à jour à chaque étape.
- Début de tâche : rappelle-moi de renommer la conversation avec le n° d'issue GitHub (`/rename`). Tu ne peux pas le faire toi-même.

## GitHub = suivi des tâches
Pas de fichiers de suivi dans le repo : l'issue est la tâche en cours, la PR mergée est la tâche archivée.
- **1 tâche = 1 issue** (template "Tâche") : objectif, critères d'acceptation, hors périmètre. Labels : un type (`enhancement`, `bug`, `documentation`, `chore`) + une ou plusieurs `area:*`. Milestone de l'étape.
- Question à enjeu non tranchée → label `enjeu` ; bloquée → `blocked`. Labels et milestones se modifient dans `.github/labels.tsv` / `.github/milestones.tsv` (synchro par le workflow `repo-meta`), jamais à la main.
- **Suivi** (avancement, décisions prises, handoff, points ouverts) : commentaires sur l'issue, ou sur la PR une fois ouverte. Pour reprendre une tâche : lis l'issue, ses commentaires et la PR liée.
- **PR** : une par issue, template rempli, `Closes #n`. CI + review Claude automatiques. Traite les retours de la review avant de me la présenter. Je merge.
- **Review Claude** : check vert ≠ review faite, la preuve est le commentaire de synthèse sur la PR. Une PR qui modifie `.github/workflows/claude-review.yml` n'est jamais reviewée (l'action exige un fichier identique à `main`) : relecture humaine.

## Orchestrateur
- Tu es orchestrateur : récupérer l'info résumée de ce qui a été fait, créer des tâches faciles, assigner.
- Tu codes toi-même seulement si la modif tient en 1 fichier et 20 lignes maximum. Au-delà, délègue à un agent.
- Avant de me présenter une étape, vérifie toi-même : build, clippy, fmt, tests (voir "Build, test, lint") et `git diff --stat` cohérent avec le résumé de l'agent. Un résumé d'agent peut être faux. Si le build n'est pas lançable, dis-le explicitement au lieu d'affirmer que ça compile.
- Personne ne voit l'écran à part moi : ce qui touche au rendu, à l'UI ou au feeling, c'est moi qui teste. Donne-moi la commande à lancer et ce que je dois observer.

## Skills
Si je ne l'ai pas spécifié, utilise /caveman et /superpowers:using-superpowers.

## Rédaction de documentation
Doc de recherche de données et d'architecture (en général en préparation d'une nouvelle tâche) : utilise le skill `visual-docs` (diagrammes, alertes, imports Markdown Preview Enhanced). Mets des liens vers les fichiers qui ne bougeront pas, sinon des chemins relatifs.
Toute nouvelle doc est référencée dans [INDEX.md](INDEX.md).

# Build, test, lint
Depuis la racine :
```sh
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo test --workspace
```
- Lancer un jeu : `cargo run -p sample -- host [port] | server [port] | client <ip[:port]>` (port par défaut 5000).
- Linux : Bevy demande `libudev-dev libwayland-dev libxkbcommon-dev` (pas d'audio, donc pas d'alsa). Rust ≥ 1.97.1 (exigé par Bevy 0.20).
- Premier build Bevy long (plusieurs minutes) : lance-le en arrière-plan, ne conclus pas à un blocage.
- Tests = headless (serveur sans fenêtre). Ce qui demande une fenêtre se teste à la main (voir Orchestrateur).

# Règles game dev
- **Data d'abord** : un nouveau type d'entité = un fichier `.ron` dans `games/<jeu>/assets/`, pas de code. Du code seulement pour un comportement nouveau (composant + système).
- **Serveur autoritaire** : l'état de jeu ne change que sur le serveur. Le client envoie des inputs, affiche, gère l'UI. Logique partagée (types, defs, calculs purs) dans `common`.
- **Le solo est un multi** : solo = mode host (serveur + client dans le même process). Un seul chemin de code, jamais de branche "si solo".
- **Réplication minimale** : le client a les mêmes defs que le serveur. On réplique l'id de def + l'état dynamique (position, PV...), jamais ce qui se déduit de la def (visuel, stats de base).
- **Moteur générique** : aucune logique de jeu dans `engine/`. Le jeu exemple (`games/sample`) sert de référence : à copier, pas à importer.

# Ce qu'il ne faut pas faire

- Réinventer la roue : réutilise un composant existant (moteur, Bevy, crate déjà en dépendance).
- Inventer des situations ou informations. Pose la question, je suis là. Mieux que brûler 150k tokens sans la bonne info. Je suis la source de vérité quand une information est complexe à trouver.
- **Trop de temps** = 2 recherches sans résultat sur la même information : pose la question directement.
- A part dit contraire, pas besoin de over-expliquer ce qui n'allait pas, tant que c'est réparé c'est OK.
- Pas d'over-engineering. On essaie de faire le plus clair possible, le plus straightforward possible. Critère : code direct tant qu'il y a moins de 3 cas ; à partir de 3 cas, array/enum/liste extensible sans toucher au code hardcodé.
- Pas de people-pleasing. Si je dis n'importe-quoi, dis-le moi et explique pourquoi je dis de la merde.
