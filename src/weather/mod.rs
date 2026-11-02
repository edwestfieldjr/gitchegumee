//! Wind, seas and visibility, interpolated from the storm timeline.

use bevy::prelude::*;

use crate::content::{Content, WeatherKey};
use crate::states::{SimSet, sim_running};
use crate::voyage::Voyage;

/// Wave height at which the storm counts as being at full strength.
const FULL_STORM_WAVE_FT: f32 = 25.0;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct Weather {
    pub wind_kn: f32,
    pub wind_from: f32,
    pub wave_ft: f32,
    pub visibility: f32,
}

impl Weather {
    /// Storm strength from 0 to 1. It drives pounding and the music layers.
    pub fn storm(&self) -> f32 {
        (self.wave_ft / FULL_STORM_WAVE_FT).clamp(0.0, 1.0)
    }
}

/// Interpolate a compass bearing the short way round.
fn lerp_bearing(from: f32, to: f32, t: f32) -> f32 {
    let delta = (to - from + 540.0).rem_euclid(360.0) - 180.0;
    (from + delta * t).rem_euclid(360.0)
}

/// The weather at a voyage minute. Times beyond the table hold the end values.
pub fn sample(keys: &[WeatherKey], minute: f32) -> Weather {
    let from_key = |key: &WeatherKey| Weather {
        wind_kn: key.wind_kn,
        wind_from: key.wind_from,
        wave_ft: key.wave_ft,
        visibility: key.visibility,
    };
    let (Some(first), Some(last)) = (keys.first(), keys.last()) else {
        return Weather::default();
    };
    if minute <= first.at.minute() {
        return from_key(first);
    }
    for pair in keys.windows(2) {
        let (start, end) = (pair[0].at.minute(), pair[1].at.minute());
        if minute <= end {
            let t = (minute - start) / (end - start);
            let mix = |a: f32, b: f32| a + (b - a) * t;
            return Weather {
                wind_kn: mix(pair[0].wind_kn, pair[1].wind_kn),
                wind_from: lerp_bearing(pair[0].wind_from, pair[1].wind_from, t),
                wave_ft: mix(pair[0].wave_ft, pair[1].wave_ft),
                visibility: mix(pair[0].visibility, pair[1].visibility),
            };
        }
    }
    from_key(last)
}

/// A compass point for a bearing, for display.
pub fn compass_point(bearing: f32) -> &'static str {
    const POINTS: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
    let index = ((bearing.rem_euclid(360.0) + 22.5) / 45.0) as usize % POINTS.len();
    POINTS[index]
}

fn update_weather(voyage: Res<Voyage>, content: Res<Content>, mut weather: ResMut<Weather>) {
    *weather = sample(&content.weather, voyage.minute);
}

pub struct WeatherPlugin;

impl Plugin for WeatherPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Weather>().add_systems(
            FixedUpdate,
            update_weather.in_set(SimSet::Weather).run_if(sim_running),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::At;
    use crate::content::tests::real_content;

    fn key(at: At, wind_from: f32, wave_ft: f32) -> WeatherKey {
        WeatherKey {
            at,
            wind_kn: 10.0,
            wind_from,
            wave_ft,
            visibility: 1.0,
        }
    }

    #[test]
    fn interpolates_between_keys() {
        let keys = [key(At(9, 12, 0), 0.0, 0.0), key(At(9, 14, 0), 0.0, 10.0)];
        assert_eq!(sample(&keys, At(9, 13, 0).minute()).wave_ft, 5.0);
    }

    #[test]
    fn holds_outside_the_table() {
        let keys = [key(At(9, 12, 0), 0.0, 1.0), key(At(9, 14, 0), 0.0, 10.0)];
        assert_eq!(sample(&keys, 0.0).wave_ft, 1.0);
        assert_eq!(sample(&keys, 99_999.0).wave_ft, 10.0);
    }

    #[test]
    fn wind_takes_the_short_way_round() {
        let keys = [key(At(9, 12, 0), 350.0, 0.0), key(At(9, 14, 0), 10.0, 0.0)];
        assert_eq!(sample(&keys, At(9, 13, 0).minute()).wind_from, 0.0);
    }

    #[test]
    fn storm_matches_the_reference_timeline() {
        let content = real_content();
        let at_one_am = sample(&content.weather, At(10, 1, 0).minute());
        assert_eq!((at_one_am.wind_kn, at_one_am.wave_ft), (52.0, 10.0));
        let before_snow = sample(&content.weather, At(10, 14, 0).minute());
        let in_snow = sample(&content.weather, At(10, 14, 45).minute());
        assert!(in_snow.visibility < before_snow.visibility);
        let peak = sample(&content.weather, At(10, 18, 0).minute());
        assert_eq!(peak.wave_ft, 25.0);
        assert_eq!(peak.storm(), 1.0);
    }

    #[test]
    fn wind_shifts_northeast_south_northwest() {
        let content = real_content();
        let from = |at: At| compass_point(sample(&content.weather, at.minute()).wind_from);
        assert_eq!(from(At(10, 1, 0)), "NE");
        assert_eq!(from(At(10, 13, 30)), "S");
        assert_eq!(from(At(10, 16, 0)), "NW");
    }
}
