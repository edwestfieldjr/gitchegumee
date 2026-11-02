//! The helm simulation, free of rendering so that it can be tested.

use crate::content::{HelmDef, HelmTuning};
use crate::rng::GameRng;
use crate::ship::ShipDamage;

pub const FIELD_W: f32 = 256.0;
pub const SHIP_W: f32 = 12.0;
pub const SHIP_H: f32 = 54.0;
pub const SHIP_TOP: f32 = 113.0;
const SHIP_MARGIN: f32 = 14.0;
/// Clear water at the start and end of a section, in pixels of scroll.
const LEAD_IN: f32 = 180.0;
const LEAD_OUT: f32 = 160.0;
const CREST_H: f32 = 8.0;
const MAX_STEER_SPEED: f32 = 72.0;
const LIST_DRIFT_PER_DEG: f32 = 1.2;
const WIND_DRIFT: f32 = 26.0;
const WIND_SHIFT_SECONDS: f32 = 6.0;
const DRIFT_RATE: f32 = 0.8;
/// How far a gap may move per pixel of scroll between waves, so the next gap
/// can always be reached from the last.
const GAP_REACH: f32 = 0.55;
/// Each gap's narrowest width range, and how many times wider it may be drawn.
const GAP_MIN: f32 = 56.0;
const GAP_MAX: f32 = 84.0;
const GAP_WIDEST_FACTOR: f32 = 1.75;
/// How far a crest sways from side to side, in pixels. The gap sways with it,
/// and collision follows, so what is on screen is what can be hit.
pub const CREST_SWAY: f32 = 5.0;
const CREST_SWAY_RATE: f32 = 0.022;
/// The three-wave set at the end of the last stretch: heights and gap widths.
const ROGUE_SET: [(f32, f32); 3] = [(10.0, 80.0), (14.0, 60.0), (18.0, 44.0)];
const ROGUE_SPACING: f32 = 240.0;
const ROGUE_SET_AT: f32 = 0.6;
/// Keeps the widest rogue gap wholly on the field as it sways.
const GAP_EDGE_ROOM: f32 = 76.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HazardKind {
    /// A crest across the field with one gap in it.
    Wave {
        gap_x: f32,
        gap_w: f32,
    },
    /// Like a wave, but larger and heavier.
    Rogue {
        gap_x: f32,
        gap_w: f32,
        scale: f32,
    },
    Debris,
    Shoal,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hazard {
    /// Scroll distance at which its top edge enters the field.
    pub at: f32,
    pub x: f32,
    pub w: f32,
    pub h: f32,
    pub kind: HazardKind,
}

fn wave_gap(rng: &mut GameRng, last_gap: f32, spacing: f32) -> (f32, f32) {
    let gap_w = rng.range(GAP_MIN, GAP_MAX) * rng.range(1.0, GAP_WIDEST_FACTOR);
    let reach = spacing * GAP_REACH;
    let low = (last_gap - reach).max(SHIP_MARGIN + gap_w / 2.0);
    let high = (last_gap + reach).min(FIELD_W - SHIP_MARGIN - gap_w / 2.0);
    (rng.range(low, high.max(low)), gap_w)
}

/// Lay out a section. `sea_factor` thins the waves when the route gives lee,
/// and a wide pass of the shoals uses the section's lower shoal density.
pub fn plan_section(
    section: &HelmDef,
    sea_factor: f32,
    passed_wide: bool,
    rng: &mut GameRng,
) -> (Vec<Hazard>, f32) {
    let length = section.scroll * section.seconds;
    let mut hazards = Vec::new();

    let wave_density = section.waves * sea_factor;
    if wave_density > 0.0 {
        let mut at = LEAD_IN;
        let mut last_gap = FIELD_W / 2.0;
        while at < length - LEAD_OUT {
            let spacing = 100.0 / wave_density * rng.range(0.8, 1.3);
            let (gap_x, gap_w) = wave_gap(rng, last_gap, spacing);
            hazards.push(Hazard {
                at,
                x: 0.0,
                w: FIELD_W,
                h: CREST_H,
                kind: HazardKind::Wave { gap_x, gap_w },
            });
            last_gap = gap_x;
            at += spacing;
        }
    }

    let mut scatter = |density: f32, kind: HazardKind, w: (f32, f32), h: (f32, f32)| {
        if density <= 0.0 {
            return;
        }
        let mut at = LEAD_IN + rng.range(0.0, 100.0);
        while at < length - LEAD_OUT {
            let width = rng.range(w.0, w.1);
            hazards.push(Hazard {
                at,
                x: rng.range(0.0, FIELD_W - width),
                w: width,
                h: rng.range(h.0, h.1),
                kind,
            });
            at += 100.0 / density * rng.range(0.6, 1.4);
        }
    };
    scatter(section.logs, HazardKind::Debris, (10.0, 18.0), (3.0, 5.0));
    let shoals = if passed_wide {
        section.shoals_if_wide
    } else {
        section.shoals
    };
    scatter(shoals, HazardKind::Shoal, (30.0, 70.0), (24.0, 48.0));

    if section.rogue_set {
        let start = length * ROGUE_SET_AT;
        hazards.retain(|hazard| {
            hazard.at < start - ROGUE_SPACING / 2.0
                || hazard.at > start + ROGUE_SPACING * ROGUE_SET.len() as f32
        });
        for (index, (height, gap_w)) in ROGUE_SET.into_iter().enumerate() {
            let scale = 1.0 + index as f32 * 0.5;
            hazards.push(Hazard {
                at: start + index as f32 * ROGUE_SPACING,
                x: 0.0,
                w: FIELD_W,
                h: height,
                kind: HazardKind::Rogue {
                    gap_x: rng.range(GAP_EDGE_ROOM, FIELD_W - GAP_EDGE_ROOM),
                    gap_w: gap_w * rng.range(1.0, GAP_WIDEST_FACTOR),
                    scale,
                },
            });
        }
    }

    hazards.sort_by(|a, b| a.at.total_cmp(&b.at));
    (hazards, length)
}

/// Drawn height of a wave column: rough along its length, and falling away
/// toward the gap so that the gap reads as a low place in the sea.
pub fn crest_column_height(crest_h: f32, x: f32, seed: f32, gap_x: f32, gap_w: f32) -> f32 {
    const TAPER: f32 = 22.0;
    const STEP: f32 = 2.0;
    let rough = 0.72 + 0.28 * (x * 0.13 + seed * 0.37).sin() * (x * 0.041 + seed * 0.11).cos();
    let from_gap = ((x - gap_x).abs() - gap_w / 2.0).max(0.0);
    let taper = (from_gap / TAPER).clamp(0.3, 1.0);
    let height = crest_h * rough * taper;
    ((height / STEP).round() * STEP).clamp(STEP, crest_h)
}

/// How far a crest has swayed from its home at this point in the scroll.
pub fn crest_sway(hazard_at: f32, progress: f32) -> f32 {
    (progress * CREST_SWAY_RATE + hazard_at * 0.7).sin() * CREST_SWAY
}

/// What the ship's body covers, for collision.
fn ship_touches(ship_x: f32, hazard: &Hazard, y: f32, progress: f32) -> bool {
    let vertical = y + hazard.h >= SHIP_TOP && y <= SHIP_TOP + SHIP_H;
    if !vertical {
        return false;
    }
    let (left, right) = (ship_x - SHIP_W / 2.0, ship_x + SHIP_W / 2.0);
    match hazard.kind {
        HazardKind::Wave { gap_x, gap_w } | HazardKind::Rogue { gap_x, gap_w, .. } => {
            let center = gap_x + crest_sway(hazard.at, progress);
            left < center - gap_w / 2.0 || right > center + gap_w / 2.0
        }
        HazardKind::Debris | HazardKind::Shoal => right > hazard.x && left < hazard.x + hazard.w,
    }
}

/// Damage from one hit, given how well secured the ship is and how fast it is
/// moving. Loose hatches let in more of every sea.
pub fn hit_damage(
    kind: HazardKind,
    clamps: f32,
    slow_ahead: bool,
    tuning: &HelmTuning,
) -> ShipDamage {
    let leak = 2.0 - clamps;
    let ease = if slow_ahead {
        tuning.slow_damage_factor
    } else {
        1.0
    };
    match kind {
        HazardKind::Wave { .. } => ShipDamage {
            hull: tuning.wave_hull * ease,
            water: tuning.wave_water * leak * ease,
            list: tuning.wave_list * ease,
            rogue: false,
        },
        HazardKind::Rogue { scale, .. } => ShipDamage {
            hull: tuning.rogue_hull * scale * ease,
            water: tuning.rogue_water * scale * leak * ease,
            list: tuning.rogue_list * scale * ease,
            rogue: true,
        },
        HazardKind::Debris => ShipDamage {
            hull: tuning.log_hull * ease,
            ..ShipDamage::default()
        },
        HazardKind::Shoal => ShipDamage {
            hull: tuning.shoal_hull,
            water: tuning.shoal_water * leak,
            ..ShipDamage::default()
        },
    }
}

/// A running helm section.
#[derive(Debug, Clone)]
pub struct HelmSim {
    pub hazards: Vec<Hazard>,
    pub struck: Vec<bool>,
    pub progress: f32,
    pub length: f32,
    pub ship_x: f32,
    pub rudder: f32,
    pub drift: f32,
    drift_target: f32,
    shift_clock: f32,
    wind_shifts: bool,
    /// Counts up each time the wind shifts, so the screen can announce it.
    pub shifts: u32,
}

impl HelmSim {
    pub fn new(hazards: Vec<Hazard>, length: f32, wind_shifts: bool, base_drift: f32) -> Self {
        Self {
            struck: vec![false; hazards.len()],
            hazards,
            progress: 0.0,
            length,
            ship_x: FIELD_W / 2.0,
            rudder: 0.0,
            drift: base_drift,
            drift_target: base_drift,
            shift_clock: WIND_SHIFT_SECONDS,
            wind_shifts,
            shifts: 0,
        }
    }

    pub fn finished(&self) -> bool {
        self.progress >= self.length
    }

    /// Top of a hazard on screen at the current scroll.
    pub fn hazard_y(&self, hazard: &Hazard) -> f32 {
        self.progress - hazard.at - hazard.h
    }

    /// Advance the section and return the hazards struck in this step.
    pub fn step(
        &mut self,
        steer: f32,
        scroll: f32,
        list_deg: f32,
        rudder_rate: f32,
        dt: f32,
        rng: &mut GameRng,
    ) -> Vec<HazardKind> {
        let rudder_step = rudder_rate * dt;
        self.rudder += (steer.clamp(-1.0, 1.0) - self.rudder).clamp(-rudder_step, rudder_step);

        if self.wind_shifts {
            self.shift_clock -= dt;
            if self.shift_clock <= 0.0 {
                self.shift_clock = WIND_SHIFT_SECONDS * rng.range(0.8, 1.3);
                self.drift_target = if self.drift_target > 0.0 {
                    -WIND_DRIFT
                } else {
                    WIND_DRIFT
                };
                self.shifts += 1;
            }
        }
        let drift_step = WIND_DRIFT * DRIFT_RATE * dt;
        self.drift += (self.drift_target - self.drift).clamp(-drift_step, drift_step);

        let lateral = self.rudder * MAX_STEER_SPEED + self.drift + list_deg * LIST_DRIFT_PER_DEG;
        self.ship_x = (self.ship_x + lateral * dt).clamp(SHIP_MARGIN, FIELD_W - SHIP_MARGIN);
        self.progress += scroll * dt;

        let mut hits = Vec::new();
        for index in 0..self.hazards.len() {
            if self.struck[index] {
                continue;
            }
            let hazard = self.hazards[index];
            if ship_touches(self.ship_x, &hazard, self.hazard_y(&hazard), self.progress) {
                self.struck[index] = true;
                hits.push(hazard.kind);
            }
        }
        hits
    }

    /// The next wave or rogue gap ahead of the bow: what a guide would call.
    pub fn next_gap(&self) -> Option<f32> {
        self.hazards
            .iter()
            .filter(|hazard| self.hazard_y(hazard) < SHIP_TOP + SHIP_H)
            .find_map(|hazard| match hazard.kind {
                HazardKind::Wave { gap_x, .. } | HazardKind::Rogue { gap_x, .. } => {
                    (self.hazard_y(hazard) + hazard.h < SHIP_TOP + SHIP_H).then_some(gap_x)
                }
                _ => None,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::real_content;

    const DT: f32 = 1.0 / 64.0;

    fn section(id: &str) -> HelmDef {
        real_content()
            .helm
            .into_iter()
            .find(|section| section.id == id)
            .unwrap_or_else(|| panic!("no section {id}"))
    }

    fn play(id: &str, seed: u64, steer: impl Fn(&HelmSim) -> f32) -> usize {
        let section = section(id);
        let mut rng = GameRng::new(seed);
        let (hazards, length) = plan_section(&section, 1.0, false, &mut rng);
        let mut sim = HelmSim::new(hazards, length, section.wind_shifts, 0.0);
        let mut hits = 0;
        while !sim.finished() {
            let input = steer(&sim);
            hits += sim
                .step(input, section.scroll, 0.0, 4.0, DT, &mut rng)
                .len();
        }
        hits
    }

    /// A careful helmsman who steers for the next gap and around debris.
    fn steer_for_gap(sim: &HelmSim) -> f32 {
        let ahead = sim.hazards.iter().find(|hazard| {
            let y = sim.hazard_y(hazard);
            y + hazard.h >= -40.0
                && y <= SHIP_TOP + SHIP_H
                && matches!(
                    hazard.kind,
                    HazardKind::Wave { .. } | HazardKind::Rogue { .. }
                )
                && !sim
                    .struck
                    .iter()
                    .zip(&sim.hazards)
                    .any(|(s, h)| *s && h == *hazard)
        });
        let target = match ahead.map(|hazard| hazard.kind) {
            Some(HazardKind::Wave { gap_x, .. } | HazardKind::Rogue { gap_x, .. }) => gap_x,
            _ => FIELD_W / 2.0,
        };
        ((target - sim.ship_x) / 10.0).clamp(-1.0, 1.0)
    }

    #[test]
    fn the_same_seed_gives_the_same_section() {
        let section = section("last_stretch");
        let plan = |seed| plan_section(&section, 1.0, false, &mut GameRng::new(seed)).0;
        assert_eq!(plan(29), plan(29));
        assert_ne!(plan(29), plan(30));
    }

    #[test]
    fn no_hazards_in_the_lead_in() {
        for section in real_content().helm {
            let (hazards, _) = plan_section(&section, 1.0, false, &mut GameRng::new(7));
            assert!(
                hazards.iter().all(|hazard| hazard.at >= LEAD_IN),
                "{}",
                section.id
            );
        }
    }

    #[test]
    fn every_wave_gap_is_wide_enough_and_on_the_field() {
        for seed in 0..20 {
            for section in real_content().helm {
                let (hazards, _) = plan_section(&section, 1.0, false, &mut GameRng::new(seed));
                for hazard in hazards {
                    if let HazardKind::Wave { gap_x, gap_w }
                    | HazardKind::Rogue { gap_x, gap_w, .. } = hazard.kind
                    {
                        assert!(gap_w > SHIP_W);
                        assert!(gap_x - gap_w / 2.0 >= -CREST_SWAY);
                        assert!(gap_x + gap_w / 2.0 <= FIELD_W + CREST_SWAY);
                    }
                }
            }
        }
    }

    #[test]
    fn the_last_stretch_has_three_growing_rogue_waves() {
        let (hazards, _) = plan_section(&section("last_stretch"), 1.0, false, &mut GameRng::new(1));
        let rogues: Vec<&Hazard> = hazards
            .iter()
            .filter(|hazard| matches!(hazard.kind, HazardKind::Rogue { .. }))
            .collect();
        assert_eq!(rogues.len(), 3);
        assert!(rogues[0].h < rogues[1].h && rogues[1].h < rogues[2].h);
    }

    #[test]
    fn only_the_last_stretch_has_rogue_waves() {
        for section in real_content().helm {
            if !section.rogue_set {
                let (hazards, _) = plan_section(&section, 1.0, false, &mut GameRng::new(1));
                assert!(
                    !hazards
                        .iter()
                        .any(|h| matches!(h.kind, HazardKind::Rogue { .. }))
                );
            }
        }
    }

    #[test]
    fn a_lee_thins_the_waves() {
        let section = section("first_blow");
        let count = |factor| {
            plan_section(&section, factor, false, &mut GameRng::new(3))
                .0
                .len()
        };
        assert!(count(0.5) < count(1.0));
    }

    #[test]
    fn a_wide_pass_means_fewer_shoals() {
        let section = section("caribou_shoals");
        let shoals = |wide| {
            plan_section(&section, 1.0, wide, &mut GameRng::new(3))
                .0
                .iter()
                .filter(|hazard| hazard.kind == HazardKind::Shoal)
                .count()
        };
        assert!(shoals(true) < shoals(false));
    }

    #[test]
    fn steering_for_the_gaps_beats_holding_still() {
        for id in ["first_blow", "snow_squall", "last_stretch"] {
            let idle: usize = (0..10).map(|seed| play(id, seed, |_| 0.0)).sum();
            let careful: usize = (0..10).map(|seed| play(id, seed, steer_for_gap)).sum();
            assert!(careful * 2 < idle, "{id}: careful {careful}, idle {idle}");
        }
    }

    #[test]
    fn wave_columns_are_rough_stay_in_bounds_and_taper_at_the_gap() {
        let height = |x: f32| crest_column_height(14.0, x, 300.0, 128.0, 60.0);
        let heights: Vec<f32> = (0..43).map(|step| height(step as f32 * 6.0)).collect();
        assert!(heights.iter().all(|h| (2.0..=14.0).contains(h)));
        assert!(heights.iter().any(|h| *h != heights[0]), "a flat crest");
        assert!(height(160.0) < height(190.0));
    }

    #[test]
    fn gaps_vary_from_the_old_width_up_to_seventy_five_percent_wider() {
        let mut widths = Vec::new();
        for seed in 0..40 {
            let (hazards, _) =
                plan_section(&section("first_blow"), 1.0, false, &mut GameRng::new(seed));
            for hazard in hazards {
                if let HazardKind::Wave { gap_w, .. } = hazard.kind {
                    assert!((GAP_MIN..=GAP_MAX * GAP_WIDEST_FACTOR).contains(&gap_w));
                    widths.push(gap_w);
                }
            }
        }
        assert!(widths.iter().any(|w| *w < GAP_MAX));
        assert!(widths.iter().any(|w| *w > GAP_MAX * 1.3));
    }

    #[test]
    fn crests_sway_a_little_and_the_collision_goes_with_them() {
        let hazard = Hazard {
            at: 0.0,
            x: 0.0,
            w: FIELD_W,
            h: 8.0,
            kind: HazardKind::Wave {
                gap_x: 100.0,
                gap_w: 14.0,
            },
        };
        let y = SHIP_TOP;
        // A quarter wave along, the crest is at the far end of its sway.
        let progress = std::f32::consts::FRAC_PI_2 / CREST_SWAY_RATE;
        assert!((crest_sway(0.0, progress) - CREST_SWAY).abs() < 0.01);
        assert!(!ship_touches(100.0 + CREST_SWAY, &hazard, y, progress));
        assert!(ship_touches(100.0, &hazard, y, progress));
    }

    #[test]
    fn each_hazard_strikes_only_once() {
        let hazard = Hazard {
            at: 0.0,
            x: 0.0,
            w: FIELD_W,
            h: 8.0,
            kind: HazardKind::Wave {
                gap_x: 10.0,
                gap_w: 12.0,
            },
        };
        let mut rng = GameRng::new(1);
        let mut sim = HelmSim::new(vec![hazard], 400.0, false, 0.0);
        let mut hits = 0;
        while !sim.finished() {
            hits += sim.step(0.0, 60.0, 0.0, 4.0, DT, &mut rng).len();
        }
        assert_eq!(hits, 1);
    }

    #[test]
    fn staying_in_the_gap_avoids_the_wave() {
        let hazard = Hazard {
            at: 0.0,
            x: 0.0,
            w: FIELD_W,
            h: 8.0,
            kind: HazardKind::Wave {
                gap_x: FIELD_W / 2.0,
                gap_w: 40.0,
            },
        };
        let mut rng = GameRng::new(1);
        let mut sim = HelmSim::new(vec![hazard], 400.0, false, 0.0);
        while !sim.finished() {
            assert!(sim.step(0.0, 60.0, 0.0, 4.0, DT, &mut rng).is_empty());
        }
    }

    #[test]
    fn the_ship_stays_on_the_field() {
        let mut rng = GameRng::new(1);
        let mut sim = HelmSim::new(Vec::new(), 10_000.0, false, 0.0);
        for _ in 0..2000 {
            sim.step(1.0, 60.0, 20.0, 4.0, DT, &mut rng);
            assert!(sim.ship_x <= FIELD_W - SHIP_MARGIN);
        }
    }

    #[test]
    fn a_list_pulls_the_ship_to_starboard() {
        let mut rng = GameRng::new(1);
        let mut sim = HelmSim::new(Vec::new(), 10_000.0, false, 0.0);
        let start = sim.ship_x;
        for _ in 0..64 {
            sim.step(0.0, 60.0, 10.0, 4.0, DT, &mut rng);
        }
        assert!(sim.ship_x > start);
    }

    #[test]
    fn wind_shifts_reverse_the_drift() {
        let mut rng = GameRng::new(1);
        let mut sim = HelmSim::new(Vec::new(), 100_000.0, true, WIND_DRIFT);
        for _ in 0..(64 * 12) {
            sim.step(0.0, 60.0, 0.0, 4.0, DT, &mut rng);
        }
        assert!(sim.shifts >= 1);
    }

    #[test]
    fn rogue_waves_hurt_more_and_slowing_down_helps() {
        let tuning = real_content().balance.helm;
        let wave = HazardKind::Wave {
            gap_x: 0.0,
            gap_w: 0.0,
        };
        let rogue = HazardKind::Rogue {
            gap_x: 0.0,
            gap_w: 0.0,
            scale: 1.0,
        };
        let wave_hit = hit_damage(wave, 1.0, false, &tuning);
        let rogue_hit = hit_damage(rogue, 1.0, false, &tuning);
        let eased = hit_damage(rogue, 1.0, true, &tuning);
        assert!(rogue_hit.water > wave_hit.water);
        assert!(rogue_hit.rogue && !wave_hit.rogue);
        assert!(eased.water < rogue_hit.water);
    }

    #[test]
    fn loose_clamps_let_in_more_water() {
        let tuning = real_content().balance.helm;
        let wave = HazardKind::Wave {
            gap_x: 0.0,
            gap_w: 0.0,
        };
        assert!(
            hit_damage(wave, 0.5, false, &tuning).water
                > hit_damage(wave, 1.0, false, &tuning).water
        );
    }
}
