mod audio;
mod boot;
mod content;
#[cfg(feature = "dev")]
mod devtools;
mod events;
mod rng;
mod ship;
mod states;
mod ui;
mod voyage;
mod weather;

use bevy::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Gitche Gumee: The Last Voyage of the Edmund Fitzgerald".into(),
                    resolution: (960, 600).into(),
                    ..default()
                }),
                ..default()
            }),
    )
    .add_plugins((
        states::StatesPlugin,
        rng::RngPlugin,
        ui::UiPlugin,
        boot::BootPlugin,
    ));
    #[cfg(feature = "dev")]
    app.add_plugins(devtools::DevToolsPlugin);
    app.run();
}
