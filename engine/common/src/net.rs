//! Réseau : réplication (replicon) sur transport QUIC (quinnet).

use bevy::prelude::*;
use bevy_replicon::prelude::RepliconPlugins;
use bevy_replicon_quinnet::RepliconQuinnetPlugins;

/// Port par défaut du serveur.
pub const DEFAULT_PORT: u16 = 5000;

/// Plugins réseau communs aux 3 modes. Ajouté une seule fois par `EnginePlugin`.
pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((RepliconPlugins, RepliconQuinnetPlugins));
    }
}
