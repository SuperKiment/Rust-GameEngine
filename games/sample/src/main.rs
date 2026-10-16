use engine::prelude::*;

fn main() -> AppExit {
    App::new()
        .add_plugins(EnginePlugin::from_args("Sample"))
        .run()
}
