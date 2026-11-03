//! A venetian-blind wipe that opens each new screen.

use bevy::prelude::*;

use super::palette::Pal;
use super::{VIEW_H, VIEW_W, at};
use crate::states::GameState;

const BLIND_H: f32 = 8.0;
/// Game seconds the screen stays covered, then game seconds the blinds take to
/// open. At a `pace` of 0.4375 that is about 0.1 and then 0.45 real seconds.
const HOLD: f32 = 0.05;
const OPEN: f32 = 0.2;
const WIPE_Z: f32 = 90.0;

#[derive(Component)]
struct Blind {
    y: f32,
}

/// Height of each blind at a time since the screen changed, in whole pixels.
pub fn blind_height(t: f32) -> f32 {
    let open = ((t - HOLD) / OPEN).clamp(0.0, 1.0);
    (BLIND_H * (1.0 - open)).ceil()
}

/// Where a blind starts. Sprites are centered, so the middle of the view keeps
/// a view-wide blind on screen from edge to edge.
fn blind_start() -> Transform {
    at(VIEW_W as f32 / 2.0, 0.0, WIPE_Z)
}

fn spawn_blinds(mut commands: Commands) {
    let rows = (VIEW_H as f32 / BLIND_H) as usize;
    for row in 0..rows {
        commands.spawn((
            Sprite {
                color: Pal::Black.color(),
                custom_size: Some(Vec2::ONE),
                ..default()
            },
            blind_start(),
            Blind {
                y: row as f32 * BLIND_H,
            },
        ));
    }
}

/// The blinds keep opening during a pause, so they never freeze over the screen.
fn run_wipe(
    time: Res<Time>,
    virt: Res<Time<Virtual>>,
    real: Res<Time<Real>>,
    state: Res<State<GameState>>,
    mut clock: Local<f32>,
    mut blinds: Query<(&Blind, &mut Transform, &mut Visibility)>,
) {
    if state.is_changed() {
        *clock = 0.0;
    }
    *clock += if virt.is_paused() {
        real.delta_secs() * virt.relative_speed()
    } else {
        time.delta_secs()
    };
    let height = blind_height(*clock);
    for (blind, mut transform, mut visibility) in &mut blinds {
        *visibility = if height > 0.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        transform.translation.y = VIEW_H as f32 / 2.0 - blind.y - height / 2.0;
        transform.scale = Vec3::new(VIEW_W as f32, height.max(1.0), 1.0);
    }
}

pub struct WipePlugin;

impl Plugin for WipePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_blinds)
            .add_systems(Update, run_wipe);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_screen_holds_covered_then_opens() {
        assert_eq!(blind_height(0.0), BLIND_H);
        assert_eq!(blind_height(HOLD), BLIND_H);
        assert!(blind_height(HOLD + OPEN / 2.0) < BLIND_H);
        assert_eq!(blind_height(HOLD + OPEN), 0.0);
        assert_eq!(blind_height(100.0), 0.0);
    }

    #[test]
    fn a_blind_spans_the_whole_view() {
        let center = blind_start().translation.x;
        assert_eq!(center - VIEW_W as f32 / 2.0, -(VIEW_W as f32) / 2.0);
        assert_eq!(center + VIEW_W as f32 / 2.0, VIEW_W as f32 / 2.0);
    }
}
