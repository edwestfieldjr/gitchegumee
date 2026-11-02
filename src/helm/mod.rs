//! The action layer: a top-down run through the seas at the helm.

pub mod sim;

use bevy::prelude::*;

use crate::audio::Sfx;
use crate::boot::{FRAME_SIZE, HELM_ANGLES, Sprites};
use crate::content::{Content, Effect, HelmDef, SpeedOrder, clock_text};
use crate::events::{ActiveHelm, ApplyEffect};
use crate::rng::GameRng;
use crate::ship::{Ship, ShipDamage};
use crate::states::{GameState, SimSet};
use crate::ui::palette::Pal;
use crate::ui::{Bar, Label, at, bar_parts, label, rect, z};
use crate::voyage::{ClockRate, Voyage};
use crate::weather::Weather;
use sim::{
    FIELD_W, HazardKind, HelmSim, SHIP_H, SHIP_TOP, crest_column_height, crest_sway, hit_damage,
    plan_section,
};

const FRAME: f32 = FRAME_SIZE as f32;
/// Where the rudder must be to show the small and the full turn frames.
const SMALL_TURN_AT: f32 = 0.35;
const FULL_TURN_AT: f32 = 0.75;
/// Width of one column of a drawn wave, and the depth of its whitecap.
const CREST_COLUMN: f32 = 6.0;
const CREST_CAP: f32 = 2.0;
const SIDE_X: f32 = 258.0;
const RADAR_X: f32 = 259.0;
const RADAR_Y: f32 = 62.0;
const RADAR_SIZE: f32 = 58.0;
const RADAR_RANGE: f32 = 320.0;
const BLIPS: usize = 28;
const SPECKS: usize = 40;
const FLAKES: usize = 50;
/// Pixels ahead of the bow that can be seen at zero visibility and at full.
const SEE_NEAR: f32 = 50.0;
const SEE_FAR: f32 = 220.0;
const BANNER_SECONDS: f32 = 3.5;
const FLASH_SECONDS: f32 = 0.5;
const BASE_RUDDER_RATE: f32 = 4.0;
const FATIGUE_RUDDER_LOSS: f32 = 1.0 / 160.0;
/// Scroll multiplier for each speed order.
const SLOW_SCROLL: f32 = 0.75;
const FULL_SCROLL: f32 = 2.275;

#[derive(Resource)]
struct HelmRun {
    sim: HelmSim,
    section: HelmDef,
    banner: f32,
    flash: f32,
    flash_text: &'static str,
    spawned: usize,
    last_shifts: u32,
    shift_notice: f32,
}

/// Left and right, read in `Update` and consumed by the fixed step.
#[derive(Resource, Default)]
struct HelmInput(f32);

#[derive(Component)]
struct HazardPart {
    index: usize,
    /// Pixels below the hazard's top edge.
    drop: f32,
    height: f32,
    x: f32,
    w: f32,
    /// A crest piece, which heaves and sways. Debris and shoals hold still.
    crest: bool,
}

#[derive(Component)]
struct PlayerShip;

#[derive(Component)]
struct Speck {
    x: f32,
    y: f32,
}

#[derive(Component)]
struct Flake {
    x: f32,
    y: f32,
    fall: f32,
}

#[derive(Component)]
struct Blip(usize);

#[derive(Component)]
struct Banner;

#[derive(Component)]
struct HitNotice;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HudText {
    Clock,
    List,
    Speed,
    Radar,
    GuideName,
    GuideCall,
    Beacon,
    Notice,
}

#[derive(Component)]
struct HudLabel(HudText);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HelmGauge {
    Hull,
    Water,
}

#[derive(Component)]
struct HelmBar(HelmGauge);

fn scroll_speed(section: &HelmDef, order: SpeedOrder) -> f32 {
    section.scroll
        * match order {
            SpeedOrder::Slow => SLOW_SCROLL,
            SpeedOrder::Cruise => 1.0,
            SpeedOrder::Full => FULL_SCROLL,
        }
}

/// The column of `HELM_ANGLES` to show for a rudder position.
fn turn_frame(rudder: f32) -> usize {
    let center = HELM_ANGLES.len() / 2;
    let magnitude = rudder.abs();
    let steps = if magnitude > FULL_TURN_AT {
        2
    } else if magnitude > SMALL_TURN_AT {
        1
    } else {
        0
    };
    if rudder < 0.0 {
        center - steps
    } else {
        center + steps
    }
}

fn frame_rect(column: usize) -> Rect {
    let x = column as f32 * FRAME;
    Rect::new(x, 0.0, x + FRAME, FRAME)
}

fn enter_helm(
    mut commands: Commands,
    active: Res<ActiveHelm>,
    content: Res<Content>,
    sprites: Res<Sprites>,
    voyage: Res<Voyage>,
    mut rng: ResMut<GameRng>,
) {
    let section = content.helm[active.0].clone();
    let sea = voyage.sea_state(&content.balance);
    let (hazards, length) = plan_section(
        &section,
        sea.wave_factor,
        voyage.has_flag("wide_of_caribou"),
        &mut rng,
    );
    let drift = if section.wind_shifts { 18.0 } else { 0.0 };
    commands.insert_resource(HelmRun {
        sim: HelmSim::new(hazards, length, section.wind_shifts, drift),
        banner: BANNER_SECONDS,
        flash: 0.0,
        flash_text: "",
        spawned: 0,
        last_shifts: 0,
        shift_notice: 0.0,
        section: section.clone(),
    });
    commands.insert_resource(HelmInput::default());

    let scope = DespawnOnExit(GameState::Helm);
    commands.spawn((
        rect(0.0, 0.0, FIELD_W, 200.0, Pal::Deep, z::BACKGROUND),
        scope.clone(),
    ));
    for _ in 0..SPECKS {
        let (x, y) = (rng.range(0.0, FIELD_W - 6.0), rng.range(0.0, 200.0));
        let width = rng.range(3.0, 9.0).floor();
        commands.spawn((
            rect(
                x.floor(),
                y.floor(),
                width,
                1.0,
                Pal::Lake,
                z::BACKGROUND + 1.0,
            ),
            Speck { x, y },
            scope.clone(),
        ));
    }
    commands.spawn((
        Sprite {
            image: sprites.helm_frames.clone(),
            rect: Some(frame_rect(turn_frame(0.0))),
            ..default()
        },
        at(FIELD_W / 2.0, SHIP_TOP + SHIP_H / 2.0, z::SHIP),
        PlayerShip,
        scope.clone(),
    ));
    if section.snow {
        for _ in 0..FLAKES {
            commands.spawn((
                rect(0.0, 0.0, 1.0, 1.0, Pal::White, z::WEATHER),
                Flake {
                    x: rng.range(0.0, FIELD_W),
                    y: rng.range(0.0, 200.0),
                    fall: rng.range(50.0, 110.0),
                },
                scope.clone(),
            ));
        }
    }

    // Side panel.
    commands.spawn((
        rect(FIELD_W, 0.0, 320.0 - FIELD_W, 200.0, Pal::Night, z::HUD),
        scope.clone(),
    ));
    commands.spawn((
        rect(FIELD_W, 0.0, 1.0, 200.0, Pal::Slate, z::HUD + 1.0),
        scope.clone(),
    ));
    for (text, y, gauge, fill) in [
        ("HULL", 12.0, HelmGauge::Hull, Pal::Pine),
        ("WATER", 28.0, HelmGauge::Water, Pal::Red),
    ] {
        commands.spawn((label(text, SIDE_X, y, Pal::Gray), scope.clone()));
        let (back, front) = bar_parts(Bar::new(SIDE_X, y + 9.0, 58.0, 4.0), fill, Pal::Slate);
        commands.spawn((back, scope.clone()));
        commands.spawn((front, HelmBar(gauge), scope.clone()));
    }
    commands.spawn((
        rect(
            RADAR_X,
            RADAR_Y,
            RADAR_SIZE,
            RADAR_SIZE,
            Pal::Black,
            z::HUD + 1.0,
        ),
        scope.clone(),
    ));
    for index in 0..BLIPS {
        commands.spawn((
            rect(0.0, 0.0, 2.0, 2.0, Pal::Pine, z::HUD + 2.0),
            Visibility::Hidden,
            Blip(index),
            scope.clone(),
        ));
    }
    for (kind, x, y, color) in [
        (HudText::Clock, 2.0, 1.0, Pal::White),
        (HudText::List, SIDE_X, 46.0, Pal::White),
        (HudText::Speed, SIDE_X, 123.0, Pal::White),
        (HudText::Radar, RADAR_X + 5.0, RADAR_Y + 25.0, Pal::Red),
        (HudText::GuideName, SIDE_X, 135.0, Pal::Gray),
        (HudText::GuideCall, SIDE_X, 144.0, Pal::Yellow),
        (HudText::Beacon, SIDE_X, 158.0, Pal::Amber),
        (HudText::Notice, 80.0, 24.0, Pal::Yellow),
    ] {
        commands.spawn((label("", x, y, color), HudLabel(kind), scope.clone()));
    }
    commands.spawn((label("L/R STEER", SIDE_X, 178.0, Pal::Gray), scope.clone()));
    commands.spawn((label("U/D SPEED", SIDE_X, 187.0, Pal::Gray), scope.clone()));
    let width = section.title.len() as f32 * 6.0 + 12.0;
    let x = ((FIELD_W - width) / 2.0).floor();
    commands.spawn((
        rect(x, 80.0, width, 14.0, Pal::Night, z::PANEL),
        Banner,
        scope.clone(),
    ));
    commands.spawn((
        label(section.title.clone(), x + 6.0, 83.0, Pal::White),
        Banner,
        scope.clone(),
    ));
    commands.spawn((label("", 84.0, 40.0, Pal::Red), HitNotice, scope));
}

fn helm_input(
    keys: Res<ButtonInput<KeyCode>>,
    ship: Res<Ship>,
    mut input: ResMut<HelmInput>,
    mut effects: MessageWriter<ApplyEffect>,
) {
    let left = keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA);
    let right = keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD);
    input.0 = f32::from(u8::from(right)) - f32::from(u8::from(left));
    if keys.just_pressed(KeyCode::ArrowUp) {
        let faster = match ship.order {
            SpeedOrder::Slow => SpeedOrder::Cruise,
            SpeedOrder::Cruise | SpeedOrder::Full => SpeedOrder::Full,
        };
        effects.write(ApplyEffect(Effect::Speed(faster)));
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        let slower = match ship.order {
            SpeedOrder::Full => SpeedOrder::Cruise,
            SpeedOrder::Cruise | SpeedOrder::Slow => SpeedOrder::Slow,
        };
        effects.write(ApplyEffect(Effect::Speed(slower)));
    }
}

/// Keep the voyage clock in step with the scroll. It runs before the clock.
fn set_helm_clock(run: Res<HelmRun>, ship: Res<Ship>, mut rate: ResMut<ClockRate>) {
    let section = &run.section;
    let scroll = scroll_speed(section, ship.order);
    rate.0 = section.minutes / section.seconds * (scroll / section.scroll);
}

fn step_helm(
    time: Res<Time>,
    input: Res<HelmInput>,
    ship: Res<Ship>,
    content: Res<Content>,
    mut run: ResMut<HelmRun>,
    mut rng: ResMut<GameRng>,
    mut damage: MessageWriter<ShipDamage>,
    mut sfx: MessageWriter<Sfx>,
    mut next: ResMut<NextState<GameState>>,
) {
    let scroll = scroll_speed(&run.section, ship.order);
    let rudder_rate = BASE_RUDDER_RATE * (1.0 - ship.fatigue * FATIGUE_RUDDER_LOSS);
    let hits = run.sim.step(
        input.0,
        scroll,
        ship.list_deg,
        rudder_rate,
        time.delta_secs(),
        &mut rng,
    );
    let slow_ahead = ship.order == SpeedOrder::Slow;
    for kind in hits {
        damage.write(hit_damage(
            kind,
            ship.clamps,
            slow_ahead,
            &content.balance.helm,
        ));
        sfx.write(match kind {
            HazardKind::Wave { .. } => Sfx::Sea,
            HazardKind::Rogue { .. } => Sfx::Rogue,
            HazardKind::Debris => Sfx::Debris,
            HazardKind::Shoal => Sfx::Shoal,
        });
        run.flash = FLASH_SECONDS;
        run.flash_text = match kind {
            HazardKind::Wave { .. } | HazardKind::Rogue { .. } => "SEA ABOARD",
            HazardKind::Debris => "DEBRIS STRUCK",
            HazardKind::Shoal => "SHOAL WATER",
        };
    }
    if run.sim.finished() {
        next.set(GameState::Voyage);
    }
}

fn spawn_hazards(mut commands: Commands, mut run: ResMut<HelmRun>) {
    const SPAWN_ABOVE: f32 = -4.0;
    while run.spawned < run.sim.hazards.len() {
        let index = run.spawned;
        let hazard = run.sim.hazards[index];
        if run.sim.hazard_y(&hazard) < SPAWN_ABOVE {
            break;
        }
        run.spawned += 1;
        let scope = DespawnOnExit(GameState::Helm);
        let order = z::FIELD + 1.0;
        let crest = matches!(
            hazard.kind,
            HazardKind::Wave { .. } | HazardKind::Rogue { .. }
        );
        let mut piece = |x: f32, drop: f32, w: f32, h: f32, color: Pal, order: f32| {
            commands.spawn((
                rect(x.floor(), 0.0, w.floor(), h.floor(), color, order),
                HazardPart {
                    index,
                    drop,
                    height: h.floor(),
                    x: x.floor(),
                    w: w.floor(),
                    crest,
                },
                Visibility::Hidden,
                scope.clone(),
            ));
        };
        match hazard.kind {
            HazardKind::Wave { gap_x, gap_w } | HazardKind::Rogue { gap_x, gap_w, .. } => {
                let rogue = matches!(hazard.kind, HazardKind::Rogue { .. });
                let (body, cap) = if rogue {
                    (Pal::Foam, Pal::White)
                } else {
                    (Pal::Sky, Pal::Foam)
                };
                let (left, right) = (gap_x - gap_w / 2.0, gap_x + gap_w / 2.0);
                // Columns run past both edges, so a sway never bares the edge.
                let mut x = -CREST_COLUMN * 2.0;
                while x < FIELD_W + CREST_COLUMN {
                    let middle = x + CREST_COLUMN / 2.0;
                    if middle < left || middle > right {
                        let height = crest_column_height(hazard.h, middle, hazard.at, gap_x, gap_w);
                        let drop = hazard.h - height;
                        piece(x, drop, CREST_COLUMN, height, body, order);
                        piece(
                            x,
                            drop,
                            CREST_COLUMN,
                            CREST_CAP.min(height),
                            cap,
                            order + 0.5,
                        );
                    }
                    x += CREST_COLUMN;
                }
            }
            HazardKind::Debris | HazardKind::Shoal => {
                piece(hazard.x, 0.0, hazard.w, hazard.h, Pal::Sand, order);
            }
        }
    }
}

/// Whole pixels a crest column has risen, so that the breakers work as they
/// come. This is only for looks: collision uses the crest's fixed height.
fn crest_heave(x: f32, seconds: f32) -> f32 {
    ((x * 0.09 + seconds * 2.6).sin() * 1.6).round()
}

fn draw_field(
    time: Res<Time>,
    run: Res<HelmRun>,
    weather: Res<Weather>,
    ship: Res<Ship>,
    mut parts: Query<(&HazardPart, &mut Transform, &mut Visibility), Without<PlayerShip>>,
    mut player: Query<(&mut Sprite, &mut Transform), (With<PlayerShip>, Without<HazardPart>)>,
    mut specks: Query<(&mut Speck, &mut Transform), (Without<HazardPart>, Without<PlayerShip>)>,
) {
    let seen = SEE_NEAR + (SEE_FAR - SEE_NEAR) * weather.visibility;
    let elapsed = time.elapsed_secs();
    for (part, mut transform, mut visibility) in &mut parts {
        let hazard = &run.sim.hazards[part.index];
        let y = run.sim.hazard_y(hazard).round();
        let visible =
            matches!(hazard.kind, HazardKind::Rogue { .. }) || y + hazard.h >= SHIP_TOP - seen;
        *visibility = if visible && y < 200.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let (sway, heave) = if part.crest {
            (
                crest_sway(hazard.at, run.sim.progress).round(),
                crest_heave(part.x, elapsed),
            )
        } else {
            (0.0, 0.0)
        };
        transform.translation.x = part.x + part.w / 2.0 + sway - 160.0;
        transform.translation.y = 100.0 - y - part.drop - part.height / 2.0 + heave;
    }
    for (mut sprite, mut transform) in &mut player {
        sprite.rect = Some(frame_rect(turn_frame(run.sim.rudder)));
        transform.translation.x = run.sim.ship_x.round() - 160.0;
    }
    let scroll = scroll_speed(&run.section, ship.order);
    for (mut speck, mut transform) in &mut specks {
        speck.y += scroll * time.delta_secs();
        if speck.y > 200.0 {
            speck.y -= 200.0;
        }
        transform.translation.y = 100.0 - speck.y.round();
        transform.translation.x = speck.x.floor() - 160.0;
    }
}

fn draw_snow(time: Res<Time>, mut flakes: Query<(&mut Flake, &mut Transform)>) {
    for (mut flake, mut transform) in &mut flakes {
        let fall = flake.fall;
        flake.y += fall * time.delta_secs();
        flake.x -= fall * 0.4 * time.delta_secs();
        if flake.y > 200.0 {
            flake.y -= 200.0;
        }
        if flake.x < 0.0 {
            flake.x += FIELD_W;
        }
        transform.translation =
            at(flake.x.round(), flake.y.round(), transform.translation.z).translation;
    }
}

fn draw_radar(
    run: Res<HelmRun>,
    ship: Res<Ship>,
    mut blips: Query<(&Blip, &mut Transform, &mut Visibility)>,
) {
    let up = ship.radar_up();
    let scale = RADAR_SIZE / FIELD_W;
    let mut points: Vec<(f32, f32)> = Vec::with_capacity(BLIPS);
    if up {
        let spot = |x: f32, ahead: f32| {
            (
                RADAR_X + x * scale,
                RADAR_Y + RADAR_SIZE - 6.0 - ahead / RADAR_RANGE * (RADAR_SIZE - 8.0),
            )
        };
        points.push(spot(run.sim.ship_x, 0.0));
        for hazard in &run.sim.hazards {
            let ahead = SHIP_TOP - run.sim.hazard_y(hazard);
            if !(0.0..RADAR_RANGE).contains(&ahead) {
                continue;
            }
            match hazard.kind {
                HazardKind::Wave { gap_x, gap_w } | HazardKind::Rogue { gap_x, gap_w, .. } => {
                    points.push(spot(gap_x - gap_w / 2.0, ahead));
                    points.push(spot(gap_x + gap_w / 2.0, ahead));
                }
                HazardKind::Debris | HazardKind::Shoal => {
                    points.push(spot(hazard.x + hazard.w / 2.0, ahead));
                }
            }
        }
    }
    for (blip, mut transform, mut visibility) in &mut blips {
        if let Some((x, y)) = points.get(blip.0) {
            transform.translation = at(x.round() + 1.0, y.round() + 1.0, z::HUD + 2.0).translation;
            *visibility = Visibility::Inherited;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn guide_call(ship_x: f32, gap: Option<f32>) -> &'static str {
    const STEADY_WITHIN: f32 = 14.0;
    match gap {
        Some(gap) if gap < ship_x - STEADY_WITHIN => "COME LEFT",
        Some(gap) if gap > ship_x + STEADY_WITHIN => "COME RIGHT",
        Some(_) => "STEADY",
        None => "",
    }
}

fn draw_hud(
    time: Res<Time>,
    mut run: ResMut<HelmRun>,
    ship: Res<Ship>,
    voyage: Res<Voyage>,
    mut labels: Query<(&HudLabel, &mut Label), Without<HitNotice>>,
    mut bars: Query<(&HelmBar, &mut Bar)>,
    mut banner: Query<&mut Visibility, With<Banner>>,
    mut notice: Query<&mut Label, With<HitNotice>>,
) {
    let delta = time.delta_secs();
    run.banner = (run.banner - delta).max(0.0);
    run.flash = (run.flash - delta).max(0.0);
    run.shift_notice = (run.shift_notice - delta).max(0.0);
    if run.sim.shifts != run.last_shifts {
        run.last_shifts = run.sim.shifts;
        run.shift_notice = BANNER_SECONDS;
    }
    let guided = !ship.radar_up() && voyage.has_flag("anderson_guidance");
    let guide = if guided {
        guide_call(run.sim.ship_x, run.sim.next_gap())
    } else {
        ""
    };
    for (hud, mut text) in &mut labels {
        text.set(match hud.0 {
            HudText::Clock => clock_text(voyage.minute),
            HudText::List => format!("LIST {:>2.0} S", ship.list_deg.max(0.0)),
            HudText::Speed => match ship.order {
                SpeedOrder::Slow => "SLOW".to_string(),
                SpeedOrder::Cruise => "CRUISE".to_string(),
                SpeedOrder::Full => "FULL".to_string(),
            },
            HudText::Radar => {
                if ship.radar_up() {
                    String::new()
                } else {
                    "NO SCAN".to_string()
                }
            }
            HudText::GuideName => {
                if guided {
                    "ANDERSON:".to_string()
                } else {
                    String::new()
                }
            }
            HudText::GuideCall => guide.to_string(),
            HudText::Beacon => {
                if run.section.beacon_dark {
                    "BEACON OFF".to_string()
                } else {
                    String::new()
                }
            }
            HudText::Notice => {
                if run.shift_notice > 0.0 {
                    "THE WIND HAS SHIFTED".to_string()
                } else {
                    String::new()
                }
            }
        });
    }
    for (gauge, mut bar) in &mut bars {
        bar.value = match gauge.0 {
            HelmGauge::Hull => ship.hull,
            HelmGauge::Water => ship.water,
        } / 100.0;
    }
    let banner_state = if run.banner > 0.0 {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut banner {
        *visibility = banner_state;
    }
    for mut text in &mut notice {
        text.set(if run.flash > 0.0 { run.flash_text } else { "" });
    }
}

/// Dev autoplay: steer for the next gap. Set `DOWNBOUND_AUTOPLAY=careful` to
/// also hold slow ahead, or any other value to take the seas as they come.
#[cfg(feature = "dev")]
fn auto_steer(run: Res<HelmRun>, mut input: ResMut<HelmInput>) {
    if std::env::var("DOWNBOUND_AUTOPLAY").as_deref() == Ok("careful") {
        let target = run
            .sim
            .hazards
            .iter()
            .zip(&run.sim.struck)
            .find_map(|(hazard, struck)| {
                let y = run.sim.hazard_y(hazard);
                let passing = y + hazard.h >= -60.0 && y <= SHIP_TOP + SHIP_H;
                match hazard.kind {
                    HazardKind::Wave { gap_x, .. } | HazardKind::Rogue { gap_x, .. }
                        if passing && !struck =>
                    {
                        Some(gap_x)
                    }
                    _ => None,
                }
            })
            .unwrap_or(FIELD_W / 2.0);
        input.0 = ((target - run.sim.ship_x) / 10.0).clamp(-1.0, 1.0);
    }
}

/// Swap the game speed for the helm's, keeping any dev speed-up on top of it.
fn speed_up_for_helm(content: Res<Content>, mut time: ResMut<Time<Virtual>>) {
    let speed = time.relative_speed() * content.balance.helm_pace / content.balance.pace;
    time.set_relative_speed(speed);
}

fn restore_game_speed(content: Res<Content>, mut time: ResMut<Time<Virtual>>) {
    let speed = time.relative_speed() * content.balance.pace / content.balance.helm_pace;
    time.set_relative_speed(speed);
}

pub struct HelmPlugin;

impl Plugin for HelmPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Helm), (speed_up_for_helm, enter_helm))
            .add_systems(OnExit(GameState::Helm), restore_game_speed)
            .add_systems(
                FixedUpdate,
                (
                    set_helm_clock.before(SimSet::Clock),
                    step_helm.after(SimSet::Position),
                )
                    .run_if(in_state(GameState::Helm)),
            )
            .add_systems(
                Update,
                (
                    helm_input,
                    spawn_hazards,
                    draw_field,
                    draw_snow,
                    draw_radar,
                    draw_hud,
                )
                    .chain()
                    .run_if(in_state(GameState::Helm)),
            );
        #[cfg(feature = "dev")]
        app.add_systems(
            Update,
            auto_steer
                .after(helm_input)
                .run_if(in_state(GameState::Helm)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breakers_heave_a_little_and_in_whole_pixels() {
        let heaves: Vec<f32> = (0..100)
            .map(|step| crest_heave(40.0, step as f32 * 0.1))
            .collect();
        assert!(heaves.iter().all(|h| h.fract() == 0.0 && h.abs() <= 2.0));
        assert!(heaves.iter().any(|h| *h != heaves[0]));
    }

    #[test]
    fn the_ship_leans_no_more_than_fifteen_degrees() {
        assert_eq!(HELM_ANGLES[turn_frame(0.0)], 0.0);
        assert_eq!(HELM_ANGLES[turn_frame(0.1)], 0.0);
        assert_eq!(HELM_ANGLES[turn_frame(0.5)], 8.0);
        assert_eq!(HELM_ANGLES[turn_frame(1.0)], 15.0);
        assert_eq!(HELM_ANGLES[turn_frame(-0.5)], -8.0);
        assert_eq!(HELM_ANGLES[turn_frame(-1.0)], -15.0);
    }

    #[test]
    fn the_guide_calls_toward_the_gap() {
        assert_eq!(guide_call(100.0, Some(40.0)), "COME LEFT");
        assert_eq!(guide_call(100.0, Some(160.0)), "COME RIGHT");
        assert_eq!(guide_call(100.0, Some(105.0)), "STEADY");
        assert_eq!(guide_call(100.0, None), "");
    }

    #[test]
    fn speed_orders_set_the_scroll() {
        let content = crate::content::tests::real_content();
        let section = &content.helm[0];
        assert!(scroll_speed(section, SpeedOrder::Slow) < section.scroll);
        assert_eq!(scroll_speed(section, SpeedOrder::Cruise), section.scroll);
        assert!(scroll_speed(section, SpeedOrder::Full) > section.scroll);
    }
}
