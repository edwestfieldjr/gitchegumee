//! Ship state: hull, water, list, machinery and crew.

use bevy::prelude::*;

use crate::content::{Content, Effect, ShipTuning, SpeedOrder};
use crate::events::ApplyEffect;
use crate::rng::GameRng;
use crate::states::{SimSet, sim_running};
use crate::voyage::{SimStep, Voyage};
use crate::weather::Weather;

pub const PUMPS: usize = 6;
pub const RADARS: usize = 2;
const MAX_LEVEL: f32 = 100.0;
/// Crew fatigue from putting a failed pump back on line.
const REPAIR_FATIGUE: f32 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pump {
    Standby,
    Running,
    Failed,
}

/// A hit or other direct change reported by the action layer.
#[derive(Message, Debug, Clone, Copy, Default)]
pub struct ShipDamage {
    pub hull: f32,
    pub water: f32,
    pub list: f32,
    pub rogue: bool,
}

#[derive(Resource, Debug, Clone)]
pub struct Ship {
    /// Percent of full strength.
    pub hull: f32,
    /// Percent of what the ship can take on and stay afloat.
    pub water: f32,
    /// Degrees. Positive is to starboard.
    pub list_deg: f32,
    pub speed_kn: f32,
    pub heading_deg: f32,
    pub fuel: f32,
    pub pumps: [Pump; PUMPS],
    pub radars: [bool; RADARS],
    pub fatigue: f32,
    pub morale: f32,
    /// Fraction of the hatch clamps secured at departure.
    pub clamps: f32,
    pub topside_damage: bool,
    pub order: SpeedOrder,
}

impl Default for Ship {
    fn default() -> Self {
        Self {
            hull: MAX_LEVEL,
            water: 0.0,
            list_deg: 0.0,
            speed_kn: 0.0,
            heading_deg: 90.0,
            fuel: MAX_LEVEL,
            pumps: [Pump::Standby; PUMPS],
            radars: [true; RADARS],
            fatigue: 0.0,
            morale: 80.0,
            clamps: 0.5,
            topside_damage: false,
            order: SpeedOrder::Cruise,
        }
    }
}

/// What the route gives or takes from the ship during a step.
#[derive(Debug, Clone, Copy)]
pub struct SeaState {
    pub wave_factor: f32,
    pub speed_factor: f32,
}

impl Default for SeaState {
    fn default() -> Self {
        Self {
            wave_factor: 1.0,
            speed_factor: 1.0,
        }
    }
}

impl Ship {
    pub fn ordered_kn(&self, tuning: &ShipTuning) -> f32 {
        match self.order {
            SpeedOrder::Slow => tuning.slow_kn,
            SpeedOrder::Cruise => tuning.cruise_kn,
            SpeedOrder::Full => tuning.full_kn,
        }
    }

    /// Deck height above the water, in feet.
    pub fn freeboard_ft(&self, tuning: &ShipTuning) -> f32 {
        (tuning.base_freeboard_ft
            - self.water * tuning.freeboard_per_water
            - self.list_deg.abs() * tuning.freeboard_per_list)
            .max(0.0)
    }

    pub fn running_pumps(&self) -> usize {
        self.pumps.iter().filter(|p| **p == Pump::Running).count()
    }

    pub fn radar_up(&self) -> bool {
        self.radars.iter().any(|up| *up)
    }

    /// True once the state of the ship can no longer keep it afloat.
    pub fn foundered(&self, tuning: &ShipTuning) -> bool {
        self.hull <= 0.0
            || self.water >= MAX_LEVEL
            || self.list_deg.abs() >= tuning.founder_list_deg
    }

    /// Advance the ship by `hours` of the voyage clock.
    pub fn step(&mut self, weather: &Weather, sea: SeaState, hours: f32, tuning: &ShipTuning) {
        let ordered = self.ordered_kn(tuning);
        let pounding = (ordered / tuning.cruise_kn).powi(2) * sea.speed_factor;
        let storm = weather.storm();
        let seas = weather.wave_ft * sea.wave_factor;

        // Water comes aboard once seas stand higher than the deck.
        let over_deck = (seas - self.freeboard_ft(tuning)).max(0.0);
        let leak = tuning.ingress_base
            + tuning.ingress_clamps * (1.0 - self.clamps)
            + if self.topside_damage {
                tuning.ingress_topside
            } else {
                0.0
            };
        let ingress = over_deck * leak * pounding;
        let pumped = self.running_pumps() as f32 * tuning.pump_rate;
        self.water = (self.water + (ingress - pumped) * hours).clamp(0.0, MAX_LEVEL);

        // The list follows the water, and leans to starboard.
        let topside = if self.topside_damage {
            tuning.list_topside
        } else {
            0.0
        };
        let target = self.water * tuning.list_per_water + topside;
        let rate = tuning.list_rate * hours;
        self.list_deg += (target - self.list_deg).clamp(-rate, rate);

        self.hull =
            (self.hull - tuning.hull_wear * storm * storm * pounding * hours).clamp(0.0, MAX_LEVEL);

        let burn = tuning.fuel_burn * (ordered / tuning.cruise_kn)
            + tuning.fuel_per_pump * self.running_pumps() as f32;
        self.fuel = (self.fuel - burn * hours).clamp(0.0, MAX_LEVEL);

        self.fatigue = (self.fatigue
            + (tuning.fatigue_calm + tuning.fatigue_storm * storm) * hours)
            .clamp(0.0, MAX_LEVEL);
        self.morale = (self.morale - tuning.morale_loss * (self.water / MAX_LEVEL + storm) * hours)
            .clamp(0.0, MAX_LEVEL);

        let loss = 1.0 - tuning.head_sea_loss * storm;
        self.speed_kn = if self.fuel > 0.0 {
            ordered * loss * sea.speed_factor
        } else {
            0.0
        };
    }

    pub fn apply_damage(&mut self, damage: ShipDamage) {
        self.hull = (self.hull - damage.hull).clamp(0.0, MAX_LEVEL);
        self.water = (self.water + damage.water).clamp(0.0, MAX_LEVEL);
        self.list_deg += damage.list;
    }

    /// Apply an event effect. Effects for the voyage are ignored here.
    pub fn apply_effect(&mut self, effect: &Effect) {
        match effect {
            Effect::Hull(amount) => self.hull = (self.hull + amount).clamp(0.0, MAX_LEVEL),
            Effect::Water(amount) => self.water = (self.water + amount).clamp(0.0, MAX_LEVEL),
            Effect::List(amount) => self.list_deg += amount,
            Effect::Fatigue(amount) => {
                self.fatigue = (self.fatigue + amount).clamp(0.0, MAX_LEVEL);
            }
            Effect::Morale(amount) => self.morale = (self.morale + amount).clamp(0.0, MAX_LEVEL),
            Effect::Topside => self.topside_damage = true,
            Effect::PumpDown(index) => {
                if let Some(pump) = self.pumps.get_mut(*index) {
                    *pump = Pump::Failed;
                }
            }
            Effect::PumpsRun(count) => {
                let mut wanted = *count;
                for pump in &mut self.pumps {
                    if *pump == Pump::Failed {
                        continue;
                    }
                    *pump = if wanted > 0 {
                        wanted -= 1;
                        Pump::Running
                    } else {
                        Pump::Standby
                    };
                }
            }
            Effect::TogglePump(index) => self.toggle_pump(*index),
            Effect::RepairPump => {
                self.repair_pump(REPAIR_FATIGUE);
            }
            Effect::RadarDown(index) => {
                if let Some(radar) = self.radars.get_mut(*index) {
                    *radar = false;
                }
            }
            Effect::Speed(order) => self.order = *order,
            Effect::SetFlag(_) | Effect::SetRoute(_) | Effect::Lee(_) | Effect::Delay(_) => {}
        }
    }

    /// Switch a pump between standby and running. A failed pump stays down.
    pub fn toggle_pump(&mut self, index: usize) {
        if let Some(pump) = self.pumps.get_mut(index) {
            *pump = match *pump {
                Pump::Standby => Pump::Running,
                Pump::Running => Pump::Standby,
                Pump::Failed => Pump::Failed,
            };
        }
    }

    /// Put a crew on the first failed pump. Returns true if one was repaired.
    pub fn repair_pump(&mut self, fatigue_cost: f32) -> bool {
        let Some(pump) = self.pumps.iter_mut().find(|p| **p == Pump::Failed) else {
            return false;
        };
        *pump = Pump::Standby;
        self.fatigue = (self.fatigue + fatigue_cost).clamp(0.0, MAX_LEVEL);
        true
    }
}

fn step_ship(
    step: Res<SimStep>,
    weather: Res<Weather>,
    voyage: Res<Voyage>,
    content: Res<Content>,
    mut ship: ResMut<Ship>,
) {
    let hours = step.0 / 60.0;
    let sea = voyage.sea_state(&content.balance);
    ship.step(&weather, sea, hours, &content.balance.ship);
    ship.heading_deg = voyage.heading();
}

fn wear_pumps(
    step: Res<SimStep>,
    content: Res<Content>,
    mut rng: ResMut<GameRng>,
    mut ship: ResMut<Ship>,
) {
    let chance = content.balance.ship.pump_fail_per_hour * step.0 / 60.0;
    for pump in &mut ship.pumps {
        if *pump == Pump::Running && rng.chance(chance) {
            *pump = Pump::Failed;
        }
    }
}

fn take_damage(mut damage: MessageReader<ShipDamage>, mut ship: ResMut<Ship>) {
    for hit in damage.read() {
        ship.apply_damage(*hit);
    }
}

fn take_effects(mut effects: MessageReader<ApplyEffect>, mut ship: ResMut<Ship>) {
    for ApplyEffect(effect) in effects.read() {
        ship.apply_effect(effect);
    }
}

pub struct ShipPlugin;

impl Plugin for ShipPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Ship>()
            .add_message::<ShipDamage>()
            .add_systems(
                FixedUpdate,
                (step_ship, wear_pumps)
                    .chain()
                    .in_set(SimSet::Ship)
                    .run_if(sim_running),
            )
            .add_systems(Update, (take_damage, take_effects));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::real_content;

    fn tuning() -> ShipTuning {
        real_content().balance.ship
    }

    fn storm(wave_ft: f32) -> Weather {
        Weather {
            wind_kn: 60.0,
            wind_from: 315.0,
            wave_ft,
            visibility: 0.5,
        }
    }

    #[test]
    fn calm_water_stays_out() {
        let mut ship = Ship::default();
        ship.step(&storm(2.0), SeaState::default(), 5.0, &tuning());
        assert_eq!(ship.water, 0.0);
    }

    #[test]
    fn seas_over_the_deck_bring_water_aboard() {
        let mut ship = Ship::default();
        ship.step(&storm(25.0), SeaState::default(), 1.0, &tuning());
        assert!(ship.water > 0.0);
    }

    #[test]
    fn secured_clamps_keep_out_more_water() {
        let tuning = tuning();
        let mut loose = Ship {
            clamps: 0.5,
            ..Ship::default()
        };
        let mut tight = Ship {
            clamps: 1.0,
            ..Ship::default()
        };
        loose.step(&storm(25.0), SeaState::default(), 1.0, &tuning);
        tight.step(&storm(25.0), SeaState::default(), 1.0, &tuning);
        assert!(tight.water < loose.water);
    }

    #[test]
    fn slowing_down_reduces_water_and_wear() {
        let tuning = tuning();
        let mut fast = Ship {
            order: SpeedOrder::Full,
            ..Ship::default()
        };
        let mut slow = Ship {
            order: SpeedOrder::Slow,
            ..Ship::default()
        };
        fast.step(&storm(25.0), SeaState::default(), 1.0, &tuning);
        slow.step(&storm(25.0), SeaState::default(), 1.0, &tuning);
        assert!(slow.water < fast.water);
        assert!(slow.hull > fast.hull);
        assert!(slow.speed_kn < fast.speed_kn);
    }

    #[test]
    fn running_pumps_remove_water() {
        let mut ship = Ship {
            water: 20.0,
            ..Ship::default()
        };
        ship.apply_effect(&Effect::PumpsRun(2));
        assert_eq!(ship.running_pumps(), 2);
        ship.step(&storm(2.0), SeaState::default(), 1.0, &tuning());
        assert!(ship.water < 20.0);
    }

    #[test]
    fn failed_pumps_do_not_run_until_repaired() {
        let mut ship = Ship::default();
        ship.apply_effect(&Effect::PumpDown(0));
        ship.toggle_pump(0);
        assert_eq!(ship.pumps[0], Pump::Failed);
        ship.apply_effect(&Effect::PumpsRun(6));
        assert_eq!(ship.running_pumps(), 5);
        assert!(ship.repair_pump(3.0));
        assert_eq!(ship.pumps[0], Pump::Standby);
        assert!(!ship.repair_pump(3.0));
    }

    #[test]
    fn water_raises_the_list_to_starboard_and_cuts_freeboard() {
        let tuning = tuning();
        let mut ship = Ship {
            water: 50.0,
            ..Ship::default()
        };
        let before = ship.freeboard_ft(&tuning);
        ship.step(&storm(2.0), SeaState::default(), 1.0, &tuning);
        assert!(ship.list_deg > 0.0);
        assert!(ship.freeboard_ft(&tuning) < tuning.base_freeboard_ft);
        assert!(before < tuning.base_freeboard_ft);
    }

    #[test]
    fn starts_at_the_storm_load_line_freeboard() {
        let tuning = tuning();
        assert_eq!(Ship::default().freeboard_ft(&tuning), 11.5);
    }

    #[test]
    fn foundering_conditions() {
        let tuning = tuning();
        assert!(!Ship::default().foundered(&tuning));
        for ship in [
            Ship {
                hull: 0.0,
                ..Ship::default()
            },
            Ship {
                water: 100.0,
                ..Ship::default()
            },
            Ship {
                list_deg: 30.0,
                ..Ship::default()
            },
        ] {
            assert!(ship.foundered(&tuning));
        }
    }

    #[test]
    fn no_fuel_means_no_way() {
        let mut ship = Ship {
            fuel: 0.0,
            ..Ship::default()
        };
        ship.step(&storm(2.0), SeaState::default(), 1.0, &tuning());
        assert_eq!(ship.speed_kn, 0.0);
    }

    #[test]
    fn radar_is_up_until_both_units_fail() {
        let mut ship = Ship::default();
        ship.apply_effect(&Effect::RadarDown(0));
        assert!(ship.radar_up());
        ship.apply_effect(&Effect::RadarDown(1));
        assert!(!ship.radar_up());
    }
}
