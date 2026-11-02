//! Text events, radio traffic, and the schedule that fires them.

use bevy::prelude::*;

use crate::content::{Condition, Content, Effect, EventDef};
use crate::ship::Ship;
use crate::voyage::Voyage;

/// An event's effect, handled by whichever plugin owns the state it touches.
#[derive(Message, Debug, Clone)]
pub struct ApplyEffect(pub Effect);

pub fn condition_met(condition: &Condition, ship: &Ship, voyage: &Voyage) -> bool {
    match condition {
        Condition::Flag(flag) => voyage.has_flag(flag),
        Condition::NoFlag(flag) => !voyage.has_flag(flag),
        Condition::RouteIs(route) => voyage.route == *route,
        Condition::WaterAbove(level) => ship.water > *level,
    }
}

pub fn event_applies(event: &EventDef, ship: &Ship, voyage: &Voyage) -> bool {
    event
        .when
        .iter()
        .all(|condition| condition_met(condition, ship, voyage))
}

/// Something on the timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheduled {
    Event(usize),
    Helm(usize),
}

/// Events and helm sections in clock order, with a cursor into them.
#[derive(Resource, Debug, Clone, Default)]
pub struct Timeline {
    items: Vec<(f32, Scheduled)>,
    next: usize,
}

impl Timeline {
    pub fn new(content: &Content) -> Self {
        let events = content
            .events
            .iter()
            .enumerate()
            .map(|(index, event)| (event.at.minute(), Scheduled::Event(index)));
        let helm = content
            .helm
            .iter()
            .enumerate()
            .map(|(index, section)| (section.at.minute(), Scheduled::Helm(index)));
        let mut items: Vec<_> = events.chain(helm).collect();
        items.sort_by(|a, b| a.0.total_cmp(&b.0));
        Self { items, next: 0 }
    }

    /// The next item due at or before `minute`, moving the cursor past it.
    pub fn pop_due(&mut self, minute: f32) -> Option<Scheduled> {
        let (at, item) = *self.items.get(self.next)?;
        if at > minute {
            return None;
        }
        self.next += 1;
        Some(item)
    }
}

pub struct EventsPlugin;

impl Plugin for EventsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ApplyEffect>().init_resource::<Timeline>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::real_content;
    use crate::content::{At, RouteId};

    #[test]
    fn conditions_read_ship_and_voyage() {
        let ship = Ship {
            water: 30.0,
            ..Ship::default()
        };
        let mut voyage = Voyage {
            route: RouteId::North,
            ..Voyage::default()
        };
        voyage.flags.insert("kept_pace".to_string());
        let met = |condition| condition_met(&condition, &ship, &voyage);
        assert!(met(Condition::Flag("kept_pace".to_string())));
        assert!(!met(Condition::NoFlag("kept_pace".to_string())));
        assert!(met(Condition::RouteIs(RouteId::North)));
        assert!(!met(Condition::RouteIs(RouteId::Standard)));
        assert!(met(Condition::WaterAbove(20.0)));
        assert!(!met(Condition::WaterAbove(40.0)));
    }

    #[test]
    fn exactly_one_lee_event_applies_on_each_route() {
        let content = real_content();
        for route in [RouteId::North, RouteId::Standard] {
            let voyage = Voyage {
                route,
                ..Voyage::default()
            };
            let count = content
                .events
                .iter()
                .filter(|event| event.id.starts_with("lee_"))
                .filter(|event| event_applies(event, &Ship::default(), &voyage))
                .count();
            assert_eq!(count, 1);
        }
    }

    #[test]
    fn timeline_comes_due_in_clock_order() {
        let content = real_content();
        let mut timeline = Timeline::new(&content);
        assert_eq!(timeline.pop_due(0.0), None);
        let mut last = 0.0;
        let mut count = 0;
        while let Some(item) = timeline.pop_due(f32::MAX) {
            let at = match item {
                Scheduled::Event(index) => content.events[index].at.minute(),
                Scheduled::Helm(index) => content.helm[index].at.minute(),
            };
            assert!(at >= last);
            last = at;
            count += 1;
        }
        assert_eq!(count, content.events.len() + content.helm.len());
    }

    #[test]
    fn every_helm_section_ends_before_the_next_item_needs_it() {
        let content = real_content();
        for section in &content.helm {
            assert!(section.at.minute() + section.minutes < At(10, 19, 10).minute());
        }
    }
}
