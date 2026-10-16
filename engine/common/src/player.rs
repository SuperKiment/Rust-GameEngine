//! Joueur par client : lien serveur (`Owner`), marqueur client (`LocalPlayer`) et inputs.

use bevy::{ecs::entity::MapEntities, prelude::*};
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// Serveur uniquement (non répliqué) : client propriétaire de l'entité joueur.
/// `ClientId::Server` = le joueur local du host.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Owner(pub ClientId);

/// Marqueur sur l'entité joueur de ce client (côté client, et côté host pour son joueur local).
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct LocalPlayer;

/// Message serveur -> client : "cette entité est ton joueur". Le client en déduit `LocalPlayer`.
/// L'entité est convertie en entité client par replicon.
#[derive(Message, Serialize, Deserialize, MapEntities, Clone, Copy, Debug)]
pub struct OwnPlayer(#[entities] pub Entity);

pub trait GameInputExt {
    /// Enregistre un input de jeu (client -> serveur). Le client écrit `MessageWriter<I>` ; le serveur
    /// reçoit la dernière valeur comme composant `I` sur l'entité joueur de l'expéditeur.
    /// Même ordre d'enregistrement sur le serveur et le client.
    fn register_input<I>(&mut self) -> &mut Self
    where
        I: Component + Message + Clone + Serialize + DeserializeOwned;
}

impl GameInputExt for App {
    fn register_input<I>(&mut self) -> &mut Self
    where
        I: Component + Message + Clone + Serialize + DeserializeOwned,
    {
        self.add_client_message::<I>(Channel::Ordered)
            .add_systems(PreUpdate, apply_input::<I>.after(ServerSystems::Receive))
    }
}

/// Insère l'input reçu sur le joueur de l'expéditeur (le dernier de la frame l'emporte).
fn apply_input<I: Component + Message + Clone>(
    mut inputs: MessageReader<FromClient<I>>,
    players: Query<(Entity, &Owner)>,
    mut commands: Commands,
) {
    for input in inputs.read() {
        if let Some((player, _)) = players.iter().find(|(_, owner)| owner.0 == input.client_id) {
            commands.entity(player).insert(input.message.clone());
        }
    }
}
