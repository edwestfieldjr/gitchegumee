//! Whole voyages run without a window, to check the balance of the game.

use super::*;
use crate::content::tests::real_content;
use crate::content::{At, EventMode, SpeedOrder};
use crate::events::{Scheduled, Timeline, event_applies};
use crate::weather::sample;

/// How a simulated player answers events and fares at the helm.
struct Policy {
    clamps: usize,
    route: RouteId,
    /// Index of the choice to take in a named event.
    choose: fn(&str) -> usize,
    wave_hits_per_section: u32,
    rogue_hits: u32,
}

struct Finished {
    fate: Fate,
    minute: f32,
    ship: Ship,
    voyage: Voyage,
}

fn apply(effect: &Effect, ship: &mut Ship, voyage: &mut Voyage, content: &Content) {
    ship.apply_effect(effect);
    voyage.apply_effect(effect, &content.chart);
}

fn run(policy: &Policy) -> Finished {
    let content = real_content();
    let balance = &content.balance;
    let level = &balance.clamp_levels[policy.clamps];
    let mut ship = Ship {
        clamps: level.secured,
        ..Ship::default()
    };
    let mut voyage = Voyage::depart(
        &content.chart,
        policy.route,
        At(9, 14, 15).minute() + level.delay_minutes,
    );
    let mut timeline = Timeline::new(&content);
    let hit = balance.helm;

    loop {
        while let Some(item) = timeline.pop_due(voyage.minute) {
            match item {
                Scheduled::Helm(index) => {
                    let section = &content.helm[index];
                    let factor = 2.0 - ship.clamps;
                    for _ in 0..policy.wave_hits_per_section {
                        ship.apply_damage(ShipDamage {
                            hull: hit.wave_hull,
                            water: hit.wave_water * factor,
                            list: hit.wave_list,
                            rogue: false,
                        });
                    }
                    if section.rogue_set {
                        for _ in 0..policy.rogue_hits {
                            ship.apply_damage(ShipDamage {
                                hull: hit.rogue_hull,
                                water: hit.rogue_water * factor,
                                list: hit.rogue_list,
                                rogue: true,
                            });
                            voyage.rogue_hits += 1;
                        }
                    }
                }
                Scheduled::Event(index) => {
                    let event = &content.events[index];
                    if !event_applies(event, &ship, &voyage) {
                        continue;
                    }
                    for effect in &event.effects {
                        apply(effect, &mut ship, &mut voyage, &content);
                    }
                    if event.mode == EventMode::Panel && !event.choices.is_empty() {
                        let choice = (policy.choose)(&event.id).min(event.choices.len() - 1);
                        for effect in &event.choices[choice].effects {
                            apply(effect, &mut ship, &mut voyage, &content);
                        }
                    }
                }
            }
        }
        let step = 1.0;
        voyage.minute += step;
        let weather = sample(&content.weather, voyage.minute);
        ship.step(
            &weather,
            voyage.sea_state(balance),
            step / 60.0,
            &balance.ship,
        );
        voyage.advance(ship.speed_kn, step / 60.0, balance.final_stretch_nm);
        match check_fate(&ship, &voyage, balance) {
            Fate::Continue => {}
            Fate::PassGate => voyage.past_gate = true,
            fate @ (Fate::Lost | Fate::Reached) => {
                return Finished {
                    fate,
                    minute: voyage.minute,
                    ship,
                    voyage,
                };
            }
        }
        assert!(voyage.minute < At(12, 0, 0).minute(), "voyage never ended");
    }
}

/// The choices closest to what history records.
fn historical_choice(id: &str) -> usize {
    match id {
        "gale_warning" | "anderson_astern" | "lee_north" | "lee_open" | "schedule" => {
            usize::from(id != "gale_warning")
        }
        "storm_hits" | "caribou" => 1,
        _ => 0,
    }
}

/// The most careful answer to every event.
fn prudent_choice(id: &str) -> usize {
    match id {
        "topside" => 1,
        _ => 0,
    }
}

fn historical() -> Policy {
    Policy {
        clamps: 0,
        route: RouteId::Standard,
        choose: historical_choice,
        wave_hits_per_section: 4,
        rogue_hits: 2,
    }
}

fn prudent() -> Policy {
    Policy {
        clamps: 2,
        route: RouteId::North,
        choose: prudent_choice,
        wave_hits_per_section: 1,
        rogue_hits: 0,
    }
}

#[test]
fn a_historical_run_ends_in_the_historical_loss() {
    let finished = run(&historical());
    assert_eq!(finished.fate, Fate::Lost);
    assert!(!finished.voyage.past_gate);
}

#[test]
fn a_historical_run_lasts_into_the_final_hours() {
    let finished = run(&historical());
    assert!(
        finished.minute >= At(10, 17, 0).minute(),
        "lost too early: {} with {:?}",
        crate::content::clock_text(finished.minute),
        finished.ship
    );
}

#[test]
fn a_typical_run_is_held_short_of_the_bay() {
    let finished = run(&historical());
    let stretch = real_content().balance.final_stretch_nm;
    let remaining = finished.voyage.length_nm() - finished.voyage.nm;
    assert!(remaining >= stretch - 0.01);
}

#[test]
fn a_near_perfect_run_can_reach_whitefish_bay() {
    let finished = run(&prudent());
    assert_eq!(
        finished.fate,
        Fate::Reached,
        "ended {} with {:?}",
        crate::content::clock_text(finished.minute),
        finished.ship
    );
    assert!(finished.voyage.past_gate);
}

#[test]
fn a_careful_run_with_a_bad_last_stretch_is_still_lost() {
    let finished = run(&Policy {
        rogue_hits: 3,
        ..prudent()
    });
    assert_eq!(finished.fate, Fate::Lost);
}

#[test]
fn careful_choices_with_loose_clamps_are_still_lost() {
    let finished = run(&Policy {
        clamps: 1,
        ..prudent()
    });
    assert_eq!(finished.fate, Fate::Lost);
}

#[test]
fn pushing_the_ship_wears_the_hull_more_than_nursing_it() {
    let fast = run(&Policy {
        choose: |_| 1,
        ..prudent()
    });
    let slow = run(&prudent());
    assert!(fast.ship.hull < slow.ship.hull);
    assert_eq!(SpeedOrder::Slow, slow.ship.order);
}
