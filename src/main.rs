// Bevy systems declare what they use as parameters, so these two lints fire on
// ordinary, readable systems.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

mod audio;
mod boot;
mod content;
#[cfg(feature = "dev")]
mod devtools;
mod events;
mod helm;
mod outcome;
mod rng;
mod ship;
mod states;
mod title;
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
        content::ContentPlugin,
        boot::BootPlugin,
        title::TitlePlugin,
        weather::WeatherPlugin,
        ship::ShipPlugin,
        voyage::VoyagePlugin,
        events::EventsPlugin,
        audio::AudioGamePlugin,
        helm::HelmPlugin,
        outcome::OutcomePlugin,
    ));
    // Four times the default, so motion stays smooth at the game's quarter-speed pace.
    app.insert_resource(Time::<Fixed>::from_hz(256.0));
    #[cfg(feature = "dev")]
    app.add_plugins(devtools::DevToolsPlugin);
    app.run();
}
