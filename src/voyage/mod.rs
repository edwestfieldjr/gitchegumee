//! The voyage: clock, position along the route, decisions, and how it ends.

#[cfg(test)]
mod balance_tests;
pub mod map;
pub mod port;
pub mod route;
pub mod screen;

use std::collections::BTreeSet;

use bevy::prelude::*;

use crate::content::{Balance, Chart, Content, Effect, RouteId, Survival};
use crate::events::ApplyEffect;
use crate::rng::{GameRng, choose_seed};
use crate::ship::{SeaState, Ship, ShipDamage};
use crate::states::{GameState, SimSet, sim_running};
use crate::ui::font::wrap;
use route::{LatLon, path_heading, path_length, path_point, reroute};

/// Radio log lines kept for the voyage screen.
const LOG_LINES: usize = 40;
/// Text columns of the radio log.
const LOG_COLUMNS: usize = 50;

/// Voyage minutes passing in this fixed step.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct SimStep(pub f32);

/// Clock minutes per real second while under way. Set by the active layer.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct ClockRate(pub f32);

/// The speed used to turn lost minutes into lost miles.
const DELAY_SPEED_KN: f32 = 12.0;
/// Close enough to a waypoint to log passing it.
const PASSING_NM: f32 = 8.0;
/// The ship's own last position is never announced as a place on the route.
const UNANNOUNCED_WAYPOINT: &str = "WRECK SITE";
const NORTH_ROUTE_FLAG: &str = "north_route";

#[derive(Resource, Debug, Clone)]
pub struct Voyage {
    /// Minutes since midnight at the start of November 9.
    pub minute: f32,
    pub route: RouteId,
    pub path: Vec<LatLon>,
    /// Nautical miles run along `path`.
    pub nm: f32,
    /// Total miles run, which a change of route does not reset.
    pub run_nm: f32,
    pub flags: BTreeSet<String>,
    /// While the clock is before this, the ship is sheltering.
    pub lee_until: f32,
    /// The ship has outlived the historical last contact.
    pub past_gate: bool,
    pub log: Vec<String>,
    pub hazard_hits: u32,
    pub rogue_hits: u32,
    /// Waypoints already announced on the radio log.
    pub passed: BTreeSet<String>,
}

impl Default for Voyage {
    fn default() -> Self {
        Self {
            minute: 0.0,
            route: RouteId::Standard,
            path: Vec::new(),
            nm: 0.0,
            run_nm: 0.0,
            flags: BTreeSet::new(),
            lee_until: 0.0,
            past_gate: false,
            log: Vec::new(),
            hazard_hits: 0,
            rogue_hits: 0,
            passed: BTreeSet::new(),
        }
    }
}

impl Voyage {
    /// A voyage ready to sail on `route`, with `minute` as the departure time.
    pub fn depart(chart: &Chart, route: RouteId, minute: f32) -> Self {
        let path = chart
            .route(route)
            .map(|def| def.points.iter().map(LatLon::from).collect())
            .unwrap_or_default();
        let mut voyage = Self {
            minute,
            route,
            path,
            ..Self::default()
        };
        if route == RouteId::North {
            voyage.flags.insert(NORTH_ROUTE_FLAG.to_string());
        }
        voyage
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }

    pub fn length_nm(&self) -> f32 {
        path_length(&self.path)
    }

    pub fn position(&self) -> LatLon {
        path_point(&self.path, self.nm)
    }

    pub fn heading(&self) -> f32 {
        path_heading(&self.path, self.nm)
    }

    pub fn in_lee(&self) -> bool {
        self.minute < self.lee_until
    }

    /// How the ship's place on the lake changes what the weather does to it.
    pub fn sea_state(&self, balance: &Balance) -> SeaState {
        let mut sea = SeaState::default();
        let progress = self.nm / self.length_nm().max(1.0);
        if self.route == RouteId::North && progress < balance.north_route_lee_until {
            sea.wave_factor *= balance.north_route_wave_factor;
        }
        if self.in_lee() {
            sea.wave_factor *= balance.lee_wave_factor;
            sea.speed_factor *= balance.lee_speed_factor;
        }
        sea
    }

    /// Run the ship forward. Until the last-contact gate has passed, she
    /// cannot run past the wreck site, whatever her speed.
    pub fn advance(&mut self, speed_kn: f32, hours: f32, final_stretch_nm: f32) {
        let length = self.length_nm();
        let limit = if self.past_gate {
            length
        } else {
            (length - final_stretch_nm).max(0.0)
        };
        let before = self.nm;
        self.nm = (self.nm + speed_kn * hours).min(limit);
        self.run_nm += (self.nm - before).max(0.0);
    }

    /// Log any named waypoint the ship has just come up on.
    pub fn log_waypoints(&mut self, chart: &Chart) {
        let Some(route) = chart.route(self.route) else {
            return;
        };
        let here = self.position();
        for waypoint in &route.points[1..route.points.len().saturating_sub(1)] {
            if waypoint.name != UNANNOUNCED_WAYPOINT
                && here.distance_nm(LatLon::from(waypoint)) <= PASSING_NM
                && self.passed.insert(waypoint.name.clone())
            {
                self.add_log(format!("PASSING {}.", waypoint.name));
            }
        }
    }

    pub fn add_log(&mut self, line: impl Into<String>) {
        self.log.extend(wrap(&line.into(), LOG_COLUMNS));
        let excess = self.log.len().saturating_sub(LOG_LINES);
        self.log.drain(..excess);
    }

    /// Apply an event effect. Effects for the ship are ignored here.
    pub fn apply_effect(&mut self, effect: &Effect, chart: &Chart) {
        match effect {
            Effect::SetFlag(flag) => {
                self.flags.insert(flag.clone());
            }
            Effect::SetRoute(id) => {
                if *id != self.route
                    && let Some(def) = chart.route(*id)
                {
                    self.path = reroute(self.position(), def);
                    self.nm = 0.0;
                    self.route = *id;
                }
                if *id == RouteId::North {
                    self.flags.insert(NORTH_ROUTE_FLAG.to_string());
                } else {
                    self.flags.remove(NORTH_ROUTE_FLAG);
                }
            }
            Effect::Lee(minutes) => self.lee_until = self.minute + minutes,
            // The clock is history and keeps its time, so lost time is paid
            // for in ground not covered.
            Effect::Delay(minutes) => {
                self.nm = (self.nm - DELAY_SPEED_KN * minutes / 60.0).max(0.0);
            }
            Effect::Hull(_)
            | Effect::Water(_)
            | Effect::List(_)
            | Effect::Fatigue(_)
            | Effect::Morale(_)
            | Effect::Topside
            | Effect::PumpDown(_)
            | Effect::PumpsRun(_)
            | Effect::TogglePump(_)
            | Effect::RepairPump
            | Effect::RadarDown(_)
            | Effect::Speed(_) => {}
        }
    }
}

/// What the end of the voyage looks like from here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fate {
    Continue,
    /// The clock passed the last contact on the rare counterfactual path.
    PassGate,
    Lost,
    Reached,
}

/// The hidden test for outliving the historical loss.
pub fn survives_gate(ship: &Ship, voyage: &Voyage, survival: &Survival) -> bool {
    ship.clamps >= survival.clamps_at_least
        && survival.required_flags.iter().all(|f| voyage.has_flag(f))
        && survival.one_of_flags.iter().any(|f| voyage.has_flag(f))
        && !survival.forbidden_flags.iter().any(|f| voyage.has_flag(f))
        && ship.water <= survival.max_water
        && ship.hull >= survival.min_hull
        && ship.list_deg.abs() <= survival.max_list_deg
        && voyage.rogue_hits <= survival.max_rogue_hits
}

/// Decide whether the voyage goes on. The historical outcome holds unless the
/// ship passes the hidden test at the time of the last contact.
pub fn check_fate(ship: &Ship, voyage: &Voyage, balance: &Balance) -> Fate {
    if ship.foundered(&balance.ship) {
        return Fate::Lost;
    }
    if !voyage.past_gate && voyage.minute >= balance.last_contact_gate.minute() {
        return if survives_gate(ship, voyage, &balance.survival) {
            Fate::PassGate
        } else {
            Fate::Lost
        };
    }
    if voyage.past_gate && voyage.nm >= voyage.length_nm() {
        return Fate::Reached;
    }
    Fate::Continue
}

/// The numbers behind the hidden test, for the log.
fn gate_report(ship: &Ship, voyage: &Voyage) -> String {
    format!(
        "clamps {:.2} water {:.1} hull {:.1} list {:.1} rogue hits {} hits {} flags {:?}",
        ship.clamps,
        ship.water,
        ship.hull,
        ship.list_deg,
        voyage.rogue_hits,
        voyage.hazard_hits,
        voyage.flags
    )
}

/// How the voyage ended, for the outcome screen.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct Ending {
    pub reached_bay: bool,
    pub minute: f32,
    pub position: LatLon,
}

fn tick_clock(
    time: Res<Time>,
    rate: Res<ClockRate>,
    mut step: ResMut<SimStep>,
    mut voyage: ResMut<Voyage>,
) {
    step.0 = rate.0 * time.delta_secs();
    voyage.minute += step.0;
}

fn move_ship(
    step: Res<SimStep>,
    ship: Res<Ship>,
    content: Res<Content>,
    mut voyage: ResMut<Voyage>,
) {
    voyage.advance(
        ship.speed_kn,
        step.0 / 60.0,
        content.balance.final_stretch_nm,
    );
    voyage.log_waypoints(&content.chart);
}

fn resolve_fate(
    mut commands: Commands,
    ship: Res<Ship>,
    content: Res<Content>,
    mut voyage: ResMut<Voyage>,
    mut next: ResMut<NextState<GameState>>,
) {
    let mut finish = |reached_bay, voyage: &Voyage| {
        commands.insert_resource(Ending {
            reached_bay,
            minute: voyage.minute,
            position: voyage.position(),
        });
        next.set(GameState::Outcome);
    };
    match check_fate(&ship, &voyage, &content.balance) {
        Fate::Continue => {}
        Fate::PassGate => {
            info!("last contact passed: {}", gate_report(&ship, &voyage));
            voyage.past_gate = true;
            voyage.add_log("THE ANDERSON STILL HAS YOU ON HER RADAR.");
        }
        Fate::Lost => {
            info!("ship lost: {}", gate_report(&ship, &voyage));
            finish(false, &voyage);
        }
        Fate::Reached => finish(true, &voyage),
    }
}

fn take_effects(
    mut effects: MessageReader<ApplyEffect>,
    content: Res<Content>,
    mut voyage: ResMut<Voyage>,
) {
    for ApplyEffect(effect) in effects.read() {
        voyage.apply_effect(effect, &content.chart);
    }
}

fn count_hits(mut damage: MessageReader<ShipDamage>, mut voyage: ResMut<Voyage>) {
    for hit in damage.read() {
        voyage.hazard_hits += 1;
        if hit.rogue {
            voyage.rogue_hits += 1;
        }
    }
}

/// A fresh ship, voyage and seed, set up on entering port.
pub fn reset_run(mut ship: ResMut<Ship>, mut voyage: ResMut<Voyage>, mut rng: ResMut<GameRng>) {
    *ship = Ship::default();
    *voyage = Voyage::default();
    *rng = GameRng::new(choose_seed());
    info!("RNG seed: {}", rng.seed());
}

pub struct VoyagePlugin;

impl Plugin for VoyagePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Voyage>()
            .init_resource::<SimStep>()
            .init_resource::<ClockRate>()
            .add_systems(OnEnter(GameState::Port), reset_run)
            .add_systems(
                FixedUpdate,
                (
                    tick_clock.in_set(SimSet::Clock),
                    move_ship.in_set(SimSet::Position),
                )
                    .run_if(sim_running),
            )
            .add_systems(
                Update,
                (
                    resolve_fate.run_if(in_state(GameState::Voyage)),
                    take_effects.run_if(resource_exists::<Content>),
                    count_hits,
                ),
            )
            .add_plugins((map::MapPlugin, port::PortPlugin, screen::VoyageScreenPlugin));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::At;
    use crate::content::tests::real_content;

    fn ready() -> (Content, Voyage) {
        let content = real_content();
        let voyage = Voyage::depart(&content.chart, RouteId::Standard, At(9, 14, 15).minute());
        (content, voyage)
    }

    /// A ship and voyage that meet every part of the hidden test.
    fn prudent(content: &Content) -> (Ship, Voyage) {
        let mut voyage = Voyage::depart(&content.chart, RouteId::North, At(10, 19, 20).minute());
        for flag in [
            "kept_pace",
            "anderson_guidance",
            "wide_of_caribou",
            "took_lee",
        ] {
            voyage.flags.insert(flag.to_string());
        }
        let ship = Ship {
            clamps: 1.0,
            water: 10.0,
            hull: 80.0,
            ..Ship::default()
        };
        (ship, voyage)
    }

    #[test]
    fn the_ship_is_held_short_of_the_bay_until_the_gate() {
        let (content, mut voyage) = ready();
        let stretch = content.balance.final_stretch_nm;
        voyage.advance(14.0, 500.0, stretch);
        assert_eq!(voyage.nm, voyage.length_nm() - stretch);
        voyage.past_gate = true;
        voyage.advance(14.0, 500.0, stretch);
        assert_eq!(voyage.nm, voyage.length_nm());
    }

    #[test]
    fn the_final_stretch_matches_the_wreck_site_distance() {
        let (content, _) = ready();
        for route in &content.chart.routes {
            let count = route.points.len();
            let wreck = LatLon::from(&route.points[count - 2]);
            let bay = LatLon::from(&route.points[count - 1]);
            assert_eq!(route.points[count - 2].name, "WRECK SITE");
            let distance = wreck.distance_nm(bay);
            assert!((distance - 15.0).abs() < 1.0, "{distance}");
        }
    }

    #[test]
    fn the_default_outcome_is_the_historical_one() {
        let (content, mut voyage) = ready();
        let ship = Ship::default();
        assert_eq!(check_fate(&ship, &voyage, &content.balance), Fate::Continue);
        voyage.minute = content.balance.last_contact_gate.minute();
        assert_eq!(check_fate(&ship, &voyage, &content.balance), Fate::Lost);
    }

    #[test]
    fn prudent_run_outlives_the_gate() {
        let content = real_content();
        let (ship, voyage) = prudent(&content);
        assert_eq!(check_fate(&ship, &voyage, &content.balance), Fate::PassGate);
    }

    #[test]
    fn each_survival_requirement_matters() {
        let content = real_content();
        let balance = &content.balance;
        let lost = |ship: &Ship, voyage: &Voyage| check_fate(ship, voyage, balance) == Fate::Lost;
        let (ship, voyage) = prudent(&content);

        assert!(lost(
            &Ship {
                clamps: 0.8,
                ..ship.clone()
            },
            &voyage
        ));
        assert!(lost(
            &Ship {
                water: 40.0,
                ..ship.clone()
            },
            &voyage
        ));
        assert!(lost(
            &Ship {
                hull: 30.0,
                ..ship.clone()
            },
            &voyage
        ));
        assert!(lost(
            &Ship {
                list_deg: 12.0,
                ..ship.clone()
            },
            &voyage
        ));
        assert!(lost(
            &ship,
            &Voyage {
                rogue_hits: 2,
                ..voyage.clone()
            }
        ));

        for flag in ["kept_pace", "anderson_guidance", "wide_of_caribou"] {
            let mut missing = voyage.clone();
            missing.flags.remove(flag);
            assert!(lost(&ship, &missing), "{flag}");
        }
        let mut exposed = voyage.clone();
        exposed.flags.remove("took_lee");
        assert!(
            !lost(&ship, &exposed),
            "the north route alone gives shelter"
        );
        exposed.flags.remove("north_route");
        assert!(lost(&ship, &exposed), "no shelter of either kind");
        let mut pushed = voyage.clone();
        pushed.flags.insert("pushed".to_string());
        assert!(lost(&ship, &pushed));
    }

    #[test]
    fn foundering_ends_the_voyage_at_any_time() {
        let (content, voyage) = ready();
        let ship = Ship {
            water: 100.0,
            ..Ship::default()
        };
        assert_eq!(check_fate(&ship, &voyage, &content.balance), Fate::Lost);
    }

    #[test]
    fn reaching_the_bay_needs_the_gate_passed() {
        let content = real_content();
        let (ship, mut voyage) = prudent(&content);
        voyage.past_gate = true;
        assert_eq!(check_fate(&ship, &voyage, &content.balance), Fate::Continue);
        voyage.nm = voyage.length_nm();
        assert_eq!(check_fate(&ship, &voyage, &content.balance), Fate::Reached);
    }

    #[test]
    fn rerouting_starts_from_where_the_ship_is() {
        let (content, mut voyage) = ready();
        voyage.nm = 40.0;
        let here = voyage.position();
        voyage.apply_effect(&Effect::SetRoute(RouteId::North), &content.chart);
        assert_eq!(voyage.route, RouteId::North);
        assert_eq!(voyage.nm, 0.0);
        assert_eq!(voyage.position(), here);
    }

    #[test]
    fn miles_run_survive_a_change_of_route() {
        let (content, mut voyage) = ready();
        voyage.advance(10.0, 4.0, content.balance.final_stretch_nm);
        voyage.apply_effect(&Effect::SetRoute(RouteId::North), &content.chart);
        voyage.advance(10.0, 1.0, content.balance.final_stretch_nm);
        assert_eq!(voyage.nm, 10.0);
        assert_eq!(voyage.run_nm, 50.0);
    }

    #[test]
    fn the_north_route_shelters_early_but_not_late() {
        let content = real_content();
        let balance = &content.balance;
        let mut voyage = Voyage::depart(&content.chart, RouteId::North, 0.0);
        assert_eq!(voyage.sea_state(balance).wave_factor, 0.75);
        voyage.nm = voyage.length_nm() * 0.9;
        assert_eq!(voyage.sea_state(balance).wave_factor, 1.0);
    }

    #[test]
    fn waypoints_are_logged_once_and_the_wreck_site_never() {
        let (content, mut voyage) = ready();
        let length = voyage.length_nm();
        let mut nm = 0.0;
        while nm < length {
            voyage.nm = nm;
            voyage.log_waypoints(&content.chart);
            nm += 2.0;
        }
        let count = |text: &str| voyage.log.iter().filter(|line| line.contains(text)).count();
        assert_eq!(count("KEWEENAW"), 1);
        assert_eq!(count("CARIBOU"), 1);
        assert_eq!(count("WRECK"), 0);
    }

    #[test]
    fn lost_time_costs_ground_but_never_moves_the_clock() {
        let (content, mut voyage) = ready();
        voyage.nm = 50.0;
        let clock = voyage.minute;
        voyage.apply_effect(&Effect::Delay(30.0), &content.chart);
        assert_eq!(voyage.nm, 44.0);
        assert_eq!(voyage.minute, clock);
        voyage.apply_effect(&Effect::Delay(9999.0), &content.chart);
        assert_eq!(voyage.nm, 0.0);
    }

    #[test]
    fn shelter_lasts_until_it_runs_out() {
        let (content, mut voyage) = ready();
        voyage.apply_effect(&Effect::Lee(60.0), &content.chart);
        assert!(voyage.in_lee());
        voyage.minute += 61.0;
        assert!(!voyage.in_lee());
    }
}
