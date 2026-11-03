//! Game content: RON files under `assets/data/`, parsed and checked at boot.

use std::collections::HashSet;

use bevy::asset::AssetLoader;
use bevy::asset::io::Reader;
use bevy::prelude::*;
use serde::Deserialize;

/// A file's raw text. Parsing waits until every file has loaded, so all the
/// problems in the content can be reported at once.
#[derive(Asset, TypePath, Debug)]
pub struct RonText(pub String);

#[derive(TypePath, Default)]
struct RonTextLoader;

impl AssetLoader for RonTextLoader {
    type Asset = RonText;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _context: &mut bevy::asset::LoadContext<'_>,
    ) -> Result<RonText, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        String::from_utf8(bytes)
            .map(RonText)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

/// (day of November 1975, hour, minute).
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub struct At(pub u8, pub u8, pub u8);

impl At {
    /// Minutes since midnight at the start of November 9.
    pub fn minute(self) -> f32 {
        let day = f32::from(self.0) - 9.0;
        day * 1440.0 + f32::from(self.1) * 60.0 + f32::from(self.2)
    }
}

/// Clock text for a voyage minute, such as `NOV 10  7:20 PM`.
pub fn clock_text(minute: f32) -> String {
    let total = minute.max(0.0) as u32;
    let day = 9 + total / 1440;
    let hour = (total / 60) % 24;
    let mins = total % 60;
    let (display, suffix) = match hour {
        0 => (12, "AM"),
        1..=11 => (hour, "AM"),
        12 => (12, "PM"),
        _ => (hour - 12, "PM"),
    };
    format!("NOV {day} {display:>2}:{mins:02} {suffix}")
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct WeatherKey {
    pub at: At,
    pub wind_kn: f32,
    pub wind_from: f32,
    pub wave_ft: f32,
    pub visibility: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum RouteId {
    Standard,
    North,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Waypoint {
    pub name: String,
    pub lat: f32,
    pub lon: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RouteDef {
    pub id: RouteId,
    pub name: String,
    pub points: Vec<Waypoint>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Chart {
    pub lake: Vec<(f32, f32)>,
    pub islands: Vec<Vec<(f32, f32)>>,
    /// Waterways too narrow to outline, as lines of (lat, lon) points.
    pub rivers: Vec<Vec<(f32, f32)>>,
    pub routes: Vec<RouteDef>,
}

impl Chart {
    pub fn route(&self, id: RouteId) -> Option<&RouteDef> {
        self.routes.iter().find(|route| route.id == id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
pub enum SpeedOrder {
    Slow,
    #[default]
    Cruise,
    Full,
}

/// Something an event does to the ship or the voyage.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum Effect {
    Hull(f32),
    Water(f32),
    List(f32),
    Fatigue(f32),
    Morale(f32),
    Topside,
    PumpDown(usize),
    PumpsRun(usize),
    TogglePump(usize),
    RepairPump,
    RadarDown(usize),
    Speed(SpeedOrder),
    SetFlag(String),
    SetRoute(RouteId),
    Lee(f32),
    Delay(f32),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum Condition {
    Flag(String),
    NoFlag(String),
    RouteIs(RouteId),
    WaterAbove(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum EventMode {
    Log,
    Panel,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    pub label: String,
    pub result: String,
    pub effects: Vec<Effect>,
}

fn always() -> f32 {
    1.0
}

#[derive(Debug, Clone, Deserialize)]
pub struct EventDef {
    pub id: String,
    pub at: At,
    pub mode: EventMode,
    #[serde(default = "always")]
    pub chance: f32,
    #[serde(default)]
    pub when: Vec<Condition>,
    pub title: String,
    pub body: String,
    /// Applied as soon as the event fires, before any choice.
    #[serde(default)]
    pub effects: Vec<Effect>,
    #[serde(default)]
    pub choices: Vec<Choice>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HelmDef {
    pub id: String,
    pub at: At,
    pub minutes: f32,
    pub seconds: f32,
    pub title: String,
    pub scroll: f32,
    pub waves: f32,
    pub logs: f32,
    #[serde(default)]
    pub shoals: f32,
    /// Shoal density after a wide pass, if it differs.
    #[serde(default)]
    pub shoals_if_wide: f32,
    #[serde(default)]
    pub snow: bool,
    #[serde(default)]
    pub wind_shifts: bool,
    #[serde(default)]
    pub beacon_dark: bool,
    #[serde(default)]
    pub rogue_set: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClampLevel {
    pub name: String,
    pub secured: f32,
    pub delay_minutes: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ShipTuning {
    pub slow_kn: f32,
    pub cruise_kn: f32,
    pub full_kn: f32,
    pub head_sea_loss: f32,
    pub base_freeboard_ft: f32,
    pub freeboard_per_water: f32,
    pub freeboard_per_list: f32,
    pub ingress_base: f32,
    pub ingress_clamps: f32,
    pub ingress_topside: f32,
    pub pump_rate: f32,
    pub list_per_water: f32,
    pub list_topside: f32,
    pub list_rate: f32,
    pub hull_wear: f32,
    pub fuel_burn: f32,
    pub fuel_per_pump: f32,
    pub fatigue_calm: f32,
    pub fatigue_storm: f32,
    pub morale_loss: f32,
    pub pump_fail_per_hour: f32,
    pub founder_list_deg: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct HelmTuning {
    pub wave_water: f32,
    pub wave_hull: f32,
    pub wave_list: f32,
    pub log_hull: f32,
    pub shoal_hull: f32,
    pub shoal_water: f32,
    pub rogue_water: f32,
    pub rogue_hull: f32,
    pub rogue_list: f32,
    pub slow_damage_factor: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Survival {
    pub clamps_at_least: f32,
    pub required_flags: Vec<String>,
    pub one_of_flags: Vec<String>,
    pub forbidden_flags: Vec<String>,
    pub max_water: f32,
    pub min_hull: f32,
    pub max_list_deg: f32,
    pub max_rogue_hits: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Balance {
    /// Master game speed, applied to the whole game clock.
    pub pace: f32,
    /// Game speed during a helm section, in place of `pace`.
    pub helm_pace: f32,
    pub voyage_minutes_per_second: f32,
    pub last_contact_gate: At,
    pub final_stretch_nm: f32,
    pub north_route_wave_factor: f32,
    pub north_route_lee_until: f32,
    pub lee_wave_factor: f32,
    pub lee_speed_factor: f32,
    pub clamp_levels: Vec<ClampLevel>,
    pub ship: ShipTuning,
    pub helm: HelmTuning,
    pub survival: Survival,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Texts {
    pub closing_gone: String,
    pub closing_position: String,
    pub lost_title: String,
    pub lost_body: String,
    pub memorial_bell_caption: String,
    pub reached_title: String,
    pub reached_body: String,
    pub memorial_title: String,
    pub memorial_body: String,
}

#[derive(Resource, Debug, Clone)]
pub struct Content {
    pub weather: Vec<WeatherKey>,
    pub chart: Chart,
    pub events: Vec<EventDef>,
    pub helm: Vec<HelmDef>,
    pub balance: Balance,
    pub text: Texts,
}

/// The files that make up the content, under `assets/data/`.
pub const FILES: [&str; 6] = ["weather", "chart", "events", "helm", "balance", "text"];

/// Text columns that fit inside an event panel.
pub const PANEL_COLUMNS: usize = 48;
/// Text rows available for an event body.
const PANEL_ROWS: usize = 11;

fn parse_one<T: for<'de> Deserialize<'de>>(
    name: &str,
    text: &str,
    errors: &mut Vec<String>,
) -> Option<T> {
    match ron::from_str(text) {
        Ok(value) => Some(value),
        Err(error) => {
            errors.push(format!("{name}.ron: {error}"));
            None
        }
    }
}

impl Content {
    /// Parse the text of each file, in the order of [`FILES`], then validate.
    pub fn parse(texts: [&str; 6]) -> Result<Self, Vec<String>> {
        let mut errors = Vec::new();
        let weather = parse_one(FILES[0], texts[0], &mut errors);
        let chart = parse_one(FILES[1], texts[1], &mut errors);
        let events = parse_one(FILES[2], texts[2], &mut errors);
        let helm = parse_one(FILES[3], texts[3], &mut errors);
        let balance = parse_one(FILES[4], texts[4], &mut errors);
        let text = parse_one(FILES[5], texts[5], &mut errors);
        let (Some(weather), Some(chart), Some(events), Some(helm), Some(balance), Some(text)) =
            (weather, chart, events, helm, balance, text)
        else {
            return Err(errors);
        };
        let content = Self {
            weather,
            chart,
            events,
            helm,
            balance,
            text,
        };
        content.validate()?;
        Ok(content)
    }

    /// Check what the type system cannot, and report every problem found.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.weather.is_empty() {
            errors.push("weather: the timeline is empty".to_string());
        }
        for pair in self.weather.windows(2) {
            if pair[0].at.minute() >= pair[1].at.minute() {
                errors.push(format!("weather: {:?} is out of order", pair[1].at));
            }
        }
        for key in &self.weather {
            if !(0.0..=1.0).contains(&key.visibility) {
                errors.push(format!("weather: bad visibility at {:?}", key.at));
            }
        }

        for id in [RouteId::Standard, RouteId::North] {
            match self.chart.route(id) {
                Some(route) if route.points.len() >= 2 => {}
                _ => errors.push(format!("chart: route {id:?} is missing or too short")),
            }
        }
        if self.chart.lake.len() < 3 {
            errors.push("chart: the lake outline is too short".to_string());
        }

        let mut seen = HashSet::new();
        for pair in self.events.windows(2) {
            if pair[0].at.minute() > pair[1].at.minute() {
                errors.push(format!("events: {} is out of order", pair[1].id));
            }
        }
        for event in &self.events {
            if !seen.insert(event.id.as_str()) {
                errors.push(format!("events: duplicate id {}", event.id));
            }
            if !(0.0..=1.0).contains(&event.chance) {
                errors.push(format!("events: {} has a bad chance", event.id));
            }
            if event.mode == EventMode::Log && !event.choices.is_empty() {
                errors.push(format!("events: log event {} has choices", event.id));
            }
            let rows = crate::ui::font::wrap(&event.body, PANEL_COLUMNS).len();
            if rows > PANEL_ROWS {
                errors.push(format!("events: {} body has {rows} rows", event.id));
            }
            for choice in &event.choices {
                if choice.label.len() > PANEL_COLUMNS - 4 {
                    errors.push(format!("events: long choice label in {}", event.id));
                }
            }
        }

        for section in &self.helm {
            if section.seconds <= 0.0 || section.minutes <= 0.0 || section.scroll <= 0.0 {
                errors.push(format!("helm: {} needs positive timing", section.id));
            }
        }

        let balance = &self.balance;
        if balance.clamp_levels.is_empty() {
            errors.push("balance: no clamp levels".to_string());
        }
        if balance.pace <= 0.0 || balance.helm_pace <= 0.0 {
            errors.push("balance: pace and helm_pace must be above zero".to_string());
        }
        if balance.voyage_minutes_per_second <= 0.0 {
            errors.push("balance: the voyage clock must run forward".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Handles to the raw files while they load.
#[derive(Resource)]
pub struct ContentFiles(pub Vec<Handle<RonText>>);

pub fn start_content_load(mut commands: Commands, server: Res<AssetServer>) {
    let handles = FILES
        .iter()
        .map(|name| server.load(format!("data/{name}.ron")))
        .collect();
    commands.insert_resource(ContentFiles(handles));
}

pub struct ContentPlugin;

impl Plugin for ContentPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<RonText>()
            .init_asset_loader::<RonTextLoader>();
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// The real content files, parsed and validated.
    pub fn real_content() -> Content {
        let files = FILES.map(|name| {
            std::fs::read_to_string(format!("assets/data/{name}.ron"))
                .unwrap_or_else(|error| panic!("{name}.ron: {error}"))
        });
        Content::parse([
            &files[0], &files[1], &files[2], &files[3], &files[4], &files[5],
        ])
        .unwrap_or_else(|errors| panic!("{errors:#?}"))
    }

    #[test]
    fn shipped_content_is_valid() {
        real_content();
    }

    #[test]
    fn timeline_matches_the_reference() {
        let content = real_content();
        let at = |id: &str| {
            content
                .events
                .iter()
                .find(|event| event.id == id)
                .unwrap_or_else(|| panic!("no event {id}"))
                .at
        };
        assert_eq!(at("gale_warning"), At(9, 19, 0));
        assert_eq!(at("storm_hits"), At(10, 1, 0));
        assert_eq!(at("storm_warning"), At(10, 2, 0));
        assert_eq!(at("snow"), At(10, 14, 45));
        assert_eq!(at("topside"), At(10, 15, 30));
        assert_eq!(at("radar_lost"), At(10, 16, 10));
        assert_eq!(at("beacon_out"), At(10, 16, 39));
        assert_eq!(at("last_contact"), At(10, 19, 10));
        assert_eq!(content.balance.last_contact_gate, At(10, 19, 20));
    }

    #[test]
    fn validation_reports_bad_content() {
        let mut content = real_content();
        content.events[0].chance = 2.0;
        content.weather.reverse();
        let errors = content.validate().unwrap_err();
        assert!(errors.iter().any(|error| error.contains("bad chance")));
        assert!(errors.iter().any(|error| error.contains("out of order")));
    }

    #[test]
    fn clock_formats_twelve_hour_time() {
        assert_eq!(clock_text(At(9, 14, 15).minute()), "NOV 9  2:15 PM");
        assert_eq!(clock_text(At(10, 19, 20).minute()), "NOV 10  7:20 PM");
        assert_eq!(clock_text(At(10, 0, 5).minute()), "NOV 10 12:05 AM");
        assert_eq!(clock_text(At(10, 12, 0).minute()), "NOV 10 12:00 PM");
    }
}
