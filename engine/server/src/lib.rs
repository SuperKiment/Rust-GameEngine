//! Simulation autoritaire, headless.

use bevy::prelude::*;

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, || info!("server started"));
    }
}
