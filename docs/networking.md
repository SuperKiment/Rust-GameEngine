10
| Lignes | Section |
|---|---|
| 12-15 | Réseau : réplication, joueur par client, inputs |
| 16-28 | Modes et connexion |
| 29-34 | Réplication |
| 35-51 | Un joueur par client |
| 52-97 | Inputs |
| 98-105 | — Limites |
| 106-110 | Tests |

# Réseau : réplication, joueur par client, inputs

Statut : **M3** (#11). Complète la section Réseau de [architecture.md](architecture.md). Les entités et leurs composants (defs) sont décrits dans [data-format.md](data-format.md).

## Modes et connexion
Le serveur est autoritaire : l'état de jeu ne change que chez lui. Le client envoie des inputs et affiche.

| Mode | Réseau | Joueur local |
|---|---|---|
| `host` (défaut, solo) | serveur + client dans le même process, endpoint ouvert | `ClientId::Server` : un joueur est spawn à `OnEnter(EngineState::Ready)` |
| `server` | serveur headless | aucun |
| `client <ip[:port]>` | connexion au serveur | celui que le serveur lui assigne |

- L'endpoint (serveur) et la connexion (client) s'ouvrent à `OnEnter(EngineState::Ready)`, une fois les defs chargées : sinon des entités arriveraient sans leurs defs.
- Le host n'ouvre aucune connexion : replicon livre localement ses messages (`ClientId::Server`). Le code de jeu est identique en solo, host et client : aucune branche "si solo".
- Les plugins, composants et messages sont enregistrés dans le même ordre sur le serveur et le client (replicon vérifie un hash du protocole à la connexion). Tout plugin de jeu est ajouté dans les 3 modes.

## Réplication
- `app.register_game_component::<T>()` = `register_type` + `replicate` (serveur -> client). Le moteur réplique `DefId`, `Position`, `Rotation`.
- Toute entité créée par `commands.spawn_def(id, pos)` est répliquée (marqueur `Replicated`). Le client reçoit l'id de def et l'état dynamique, puis reconstruit les composants statiques depuis sa copie des defs.
- Pas répliqué : `Velocity`, `Owner`, les inputs (voir plus bas).
- Le tick de réplication tourne en `FixedPostUpdate` (64 Hz par défaut).

## Un joueur par client
```
client se connecte
  -> serveur : On<Add<AuthorizedClient>>
       spawn_def("player", Vec2::ZERO) + Owner(ClientId::Client(entité))
       envoie OwnPlayer(entité joueur) à ce seul client
  -> client : reçoit OwnPlayer une fois l'entité répliquée
       insère LocalPlayer sur son entité joueur
client se déconnecte
  -> serveur : On<Remove<ConnectedClient>> -> despawn du joueur dont Owner correspond
  -> les autres clients voient l'entité disparaître (réplication)
```
- Le joueur est la def `player` (`games/<jeu>/assets/defs/player.def.ron`) : le jeu l'écrit en data (visuel, composants). Sans cette def, le spawn logue une erreur et le joueur n'a que `DefId` + `Position`.
- `Owner(ClientId)` (serveur uniquement, non répliqué) lie l'entité à son client. `ClientId::Server` = joueur local du host.
- `LocalPlayer` (marqueur) désigne l'entité joueur de **ce** client : c'est le moyen pour le code client de savoir quelle entité afficher/suivre. Exactement une par client ; en host, sur le joueur du host. Requête typique : `Query<&Position, With<LocalPlayer>>`.
- `OwnPlayer` est un message serveur -> client avec conversion d'entité (`add_mapped_server_message`). Replicon ne le livre qu'une fois l'entité répliquée (messages non indépendants, mis en file jusqu'au tick), donc `LocalPlayer` est posé sur une entité qui existe déjà. En host, le même message est livré localement (`SendTargets::Single(ClientId::Server)`) : un seul chemin de code.

## Inputs
Un input = un type du jeu, envoyé du client au serveur sans code réseau côté jeu.

```rust
use engine::prelude::*;
use serde::{Deserialize, Serialize};

// Composant + message : "dernière direction demandée".
#[derive(Component, Message, Serialize, Deserialize, Clone, Copy)]
struct MoveInput(Vec2);

pub struct SamplePlugin;

impl Plugin for SamplePlugin {
    fn build(&self, app: &mut App) {
        // Dans les 3 modes, même ordre partout.
        app.register_input::<MoveInput>()
            // Client : lit le clavier, écrit l'input (pas de clavier sur un serveur dédié).
            .add_systems(Update, send_move.run_if(resource_exists::<ButtonInput<KeyCode>>))
            // Serveur : applique l'input à son joueur.
            .add_systems(FixedUpdate, apply_move);
    }
}

fn send_move(keys: Res<ButtonInput<KeyCode>>, mut input: MessageWriter<MoveInput>) {
    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyD) { dir.x += 1.0; }
    if keys.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    input.write(MoveInput(dir));
}

// Seuls les joueurs ont un `MoveInput` : pas besoin de savoir à quel client ils sont.
fn apply_move(mut players: Query<(&MoveInput, &mut Velocity)>) {
    for (input, mut vel) in &mut players {
        vel.0 = input.0 * 100.0;
    }
}
```

Fonctionnement de `register_input::<I>()` (`engine/common/src/player.rs`) :
1. `add_client_message::<I>(Channel::Ordered)` : tout `MessageWriter<I>` du client est envoyé au serveur.
2. Un système serveur lit les `FromClient<I>`, retrouve l'entité dont `Owner` = expéditeur et y insère `I` comme composant. Le jeu ne lit que `Query<(&I, ...)>`.
3. En host, replicon renvoie le message localement avec `ClientId::Server` : le joueur du host reçoit ses inputs par le même chemin.

Bornes de `I` : `Component + Message + Clone + Serialize + DeserializeOwned`. Le crate du jeu a besoin de `serde` en dépendance pour le derive.

### Limites
- **Dernière valeur** : `I` reste sur l'entité jusqu'au prochain message. Pour un état continu (touches maintenues), le client écrit l'état à chaque frame ou à chaque changement (le serveur n'est pas prévenu qu'une touche est relâchée si le client n'écrit rien). Pour une action ponctuelle (clic), détecter le changement côté serveur avec `Changed<I>`.
- **Plusieurs messages dans la même frame serveur** : seul le dernier est conservé (les intermédiaires sont perdus).
- **Pas de prédiction ni d'interpolation** (hors V1) : le client voit son joueur bouger après l'aller-retour réseau.
- **Pas de validation** : l'input est pris tel quel. Le système de jeu serveur borne ce qu'il applique (vitesse max, etc.).
- **Ordre d'enregistrement** identique sur le serveur et le client (`register_input`, `register_game_component`), sinon replicon refuse la connexion.
- L'input n'est pas répliqué aux autres clients : si les autres doivent le voir, le système serveur écrit son effet dans un composant répliqué (`Position`, ...).

## Tests
- `engine/tests/net.rs` : connexion/déconnexion réelles (localhost, port choisi par l'OS).
- `engine/tests/players.rs` : 1 serveur + 2 clients réels avec chargement des defs (`engine/tests/assets/`) : un `LocalPlayer` par client, chaque client déplace son joueur via un input de test, un client part et son joueur disparaît chez les autres. Test host sans réseau : joueur local + input local.
- `engine/common/tests/replication.rs` : réplication des defs avec le backend de test replicon, sans réseau.
- Helpers partagés : `engine/tests/common/mod.rs`.
