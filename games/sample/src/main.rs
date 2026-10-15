use engine::prelude::*;

fn main() {
    App::new()
        .add_plugins(EnginePlugin::from_args("Sample"))
        .run();
}
