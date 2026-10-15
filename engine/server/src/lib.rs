//! Simulation autoritaire, headless.

use bevy::prelude::*;
use engine_common::{Position, Velocity, WorldConfig};

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, || info!("server started"))
            .add_systems(FixedUpdate, integrate_velocity);
    }
}

/// `Position += Velocity * dt`, puis clamp aux bornes si `WorldConfig` existe.
fn integrate_velocity(
    time: Res<Time<Fixed>>,
    config: Option<Res<WorldConfig>>,
    mut query: Query<(&mut Position, &Velocity)>,
) {
    let dt = time.delta_secs();
    for (mut pos, vel) in &mut query {
        pos.0 += vel.0 * dt;
        if let Some(config) = &config {
            pos.0 = config.clamp(pos.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::time::TimeUpdateStrategy;

    #[test]
    fn velocity_moves_position_within_bounds() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, ServerPlugin))
            .insert_resource(WorldConfig {
                bounds: Rect::new(-10.0, -10.0, 10.0, 10.0),
                tile_size: 1.0,
            })
            .insert_resource(TimeUpdateStrategy::ManualDuration(
                Time::<Fixed>::default().timestep(),
            ));
        let e = app
            .world_mut()
            .spawn((Position::default(), Velocity(Vec2::new(64.0, 0.0))))
            .id();

        // Fixed = 64 Hz : 64 u/s -> 1 u par tick. 1re update : le temps s'initialise, aucun tick.
        app.update();
        app.update();
        let x = app.world().get::<Position>(e).unwrap().0.x;
        assert!((x - 1.0).abs() < 1e-4, "x = {x}");

        for _ in 0..20 {
            app.update();
        }
        assert_eq!(app.world().get::<Position>(e).unwrap().0.x, 10.0);
    }
}
