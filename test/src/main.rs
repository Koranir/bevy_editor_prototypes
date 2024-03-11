use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy_world_sync::WorldSyncPlugin)
        .add_systems(Startup, startup)
        .run()
}

fn startup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn(NodeBundle::default());
}
