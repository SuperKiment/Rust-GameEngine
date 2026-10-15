//! World 2D : position, rotation, vitesse, bornes et grille.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Position dans le monde (unités monde).
#[derive(Component, Reflect, Serialize, Deserialize, Default, Clone, Copy, Debug, PartialEq)]
#[reflect(Component)]
pub struct Position(pub Vec2);

/// Rotation en radians.
#[derive(Component, Reflect, Serialize, Deserialize, Default, Clone, Copy, Debug, PartialEq)]
#[reflect(Component)]
pub struct Rotation(pub f32);

/// Vitesse en unités monde par seconde.
#[derive(Component, Reflect, Serialize, Deserialize, Default, Clone, Copy, Debug, PartialEq)]
#[reflect(Component)]
pub struct Velocity(pub Vec2);

/// Bornes du monde et taille de tuile. Chargé depuis `assets/world.ron` (asset + resource).
#[derive(Asset, Resource, Reflect, Serialize, Deserialize, Clone, Debug)]
pub struct WorldConfig {
    pub bounds: Rect,
    pub tile_size: f32,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            bounds: Rect::from_center_size(Vec2::ZERO, Vec2::splat(1024.0)),
            tile_size: 32.0,
        }
    }
}

impl WorldConfig {
    /// Tuile contenant le point monde.
    pub fn world_to_tile(&self, pos: Vec2) -> IVec2 {
        (pos / self.tile_size).floor().as_ivec2()
    }

    /// Centre de la tuile.
    pub fn tile_to_world(&self, tile: IVec2) -> Vec2 {
        (tile.as_vec2() + Vec2::splat(0.5)) * self.tile_size
    }

    /// Ramène le point dans les bornes.
    pub fn clamp(&self, pos: Vec2) -> Vec2 {
        pos.clamp(self.bounds.min, self.bounds.max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> WorldConfig {
        WorldConfig {
            bounds: Rect::new(-100.0, -50.0, 100.0, 50.0),
            tile_size: 10.0,
        }
    }

    #[test]
    fn world_to_tile_floors_negatives() {
        let c = config();
        assert_eq!(c.world_to_tile(Vec2::new(0.0, 9.9)), IVec2::new(0, 0));
        assert_eq!(c.world_to_tile(Vec2::new(10.0, -0.1)), IVec2::new(1, -1));
        assert_eq!(c.world_to_tile(Vec2::new(-10.1, -20.0)), IVec2::new(-2, -2));
    }

    #[test]
    fn tile_to_world_is_center() {
        let c = config();
        assert_eq!(c.tile_to_world(IVec2::new(0, 0)), Vec2::new(5.0, 5.0));
        assert_eq!(c.tile_to_world(IVec2::new(-1, 2)), Vec2::new(-5.0, 25.0));
    }

    #[test]
    fn tile_roundtrip() {
        let c = config();
        let t = IVec2::new(-3, 4);
        assert_eq!(c.world_to_tile(c.tile_to_world(t)), t);
    }

    #[test]
    fn clamp_to_bounds() {
        let c = config();
        assert_eq!(c.clamp(Vec2::new(500.0, -500.0)), Vec2::new(100.0, -50.0));
        assert_eq!(c.clamp(Vec2::new(1.0, 2.0)), Vec2::new(1.0, 2.0));
    }
}
