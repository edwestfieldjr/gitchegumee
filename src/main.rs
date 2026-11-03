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

use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            // A web server answers a check for missing .meta files with errors.
            .set(AssetPlugin {
                meta_check: AssetMetaCheck::Never,
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Gitche Gumee: The Last Voyage of the Edmund Fitzgerald".into(),
                    resolution: (960, 600).into(),
                    // In a browser, fill the page's canvas.
                    canvas: Some("#game".into()),
                    fit_canvas_to_parent: true,
                    prevent_default_event_handling: true,
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
    // Four times the default, so motion stays smooth when the game runs slower than real time.
    app.insert_resource(Time::<Fixed>::from_hz(256.0));
    #[cfg(feature = "dev")]
    app.add_plugins(devtools::DevToolsPlugin);
    app.run();
}
