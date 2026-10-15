//! Réseau : réplication (replicon) sur transport QUIC (quinnet).

use bevy::{prelude::*, reflect::GetTypeRegistration};
use bevy_replicon::prelude::{AppRuleExt, RepliconPlugins};
use bevy_replicon::shared::replication::registry::receive_fns::MutWrite;
use bevy_replicon_quinnet::RepliconQuinnetPlugins;
use serde::{Serialize, de::DeserializeOwned};

use crate::{DefId, Position, Rotation};

/// Port par défaut du serveur.
pub const DEFAULT_PORT: u16 = 5000;

/// Plugins réseau communs aux 3 modes. Ajouté une seule fois par `EnginePlugin`.
pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((RepliconPlugins, RepliconQuinnetPlugins, ProtocolPlugin));
    }
}

/// Composants répliqués par le moteur. Sans transport : s'ajoute après `RepliconPlugins`
/// (déjà inclus dans `NetPlugin`). Séparé pour tester la réplication sans quinnet.
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // `Velocity` n'est pas répliqué (pas de prédiction en V1).
        app.register_game_component::<DefId>()
            .register_game_component::<Position>()
            .register_game_component::<Rotation>();
    }
}

pub trait GameComponentExt {
    /// Enregistre un composant de jeu : réflexion (`register_type`) + réplication serveur → client.
    /// Même ordre d'enregistrement sur le serveur et le client (chaque plugin de jeu est ajouté dans tous les modes).
    fn register_game_component<T>(&mut self) -> &mut Self
    where
        T: Component<Mutability: MutWrite<T>> + GetTypeRegistration + Serialize + DeserializeOwned;
}

impl GameComponentExt for App {
    fn register_game_component<T>(&mut self) -> &mut Self
    where
        T: Component<Mutability: MutWrite<T>> + GetTypeRegistration + Serialize + DeserializeOwned,
    {
        self.register_type::<T>().replicate::<T>()
    }
}
