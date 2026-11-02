//! The port screen: decisions made before the ship sails.

use bevy::prelude::*;

use super::Voyage;
use crate::audio::Sfx;
use crate::content::{At, Content, RouteId, clock_text};
use crate::ship::Ship;
use crate::states::GameState;
use crate::ui::palette::Pal;
use crate::ui::{Label, advance_pressed, centered, label, rect, z};

/// The scheduled departure from Superior.
const DEPARTURE: At = At(9, 14, 15);
const ROUTES: [RouteId; 2] = [RouteId::Standard, RouteId::North];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    Clamps,
    Route,
    Depart,
}

const ROWS: [Row; 3] = [Row::Clamps, Row::Route, Row::Depart];

#[derive(Resource, Debug, Clone, Default)]
struct PortMenu {
    row: usize,
    clamps: usize,
    route: usize,
}

#[derive(Component)]
struct RowLabel(Row);

fn enter_port(mut commands: Commands) {
    commands.insert_resource(PortMenu::default());
    let scope = DespawnOnExit(GameState::Port);
    commands.spawn((
        rect(0.0, 0.0, 320.0, 200.0, Pal::Night, z::BACKGROUND),
        scope.clone(),
    ));
    commands.spawn((
        centered("SUPERIOR, WISCONSIN", 12.0, Pal::White, 2.0),
        scope.clone(),
    ));
    for (text, y, color) in [
        ("NOVEMBER 9, 1975", 36.0, Pal::Foam),
        (
            "26,116 LONG TONS OF TACONITE FOR ZUG ISLAND",
            50.0,
            Pal::Gray,
        ),
        ("21 HATCHES.  CREW OF 29.  NO FATHOMETER.", 60.0, Pal::Gray),
    ] {
        commands.spawn((centered(text, y, color, 1.0), scope.clone()));
    }
    for (row, y) in ROWS.iter().zip([88.0, 110.0, 138.0]) {
        commands.spawn((
            label("", 24.0, y, Pal::White),
            RowLabel(*row),
            scope.clone(),
        ));
    }
    commands.spawn((
        centered(
            "UP/DOWN SELECT   LEFT/RIGHT CHANGE   ENTER",
            182.0,
            Pal::Gray,
            1.0,
        ),
        scope,
    ));
}

fn port_input(
    keys: Res<ButtonInput<KeyCode>>,
    content: Res<Content>,
    mut menu: ResMut<PortMenu>,
    mut ship: ResMut<Ship>,
    mut voyage: ResMut<Voyage>,
    mut sfx: MessageWriter<Sfx>,
    mut next: ResMut<NextState<GameState>>,
) {
    let levels = content.balance.clamp_levels.len();
    if keys.just_pressed(KeyCode::ArrowDown) {
        menu.row = (menu.row + 1) % ROWS.len();
        sfx.write(Sfx::Select);
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        menu.row = (menu.row + ROWS.len() - 1) % ROWS.len();
        sfx.write(Sfx::Select);
    }
    let right = keys.just_pressed(KeyCode::ArrowRight);
    let left = keys.just_pressed(KeyCode::ArrowLeft);
    if right != left {
        let cycle = |value: usize, count: usize| {
            if right {
                (value + 1) % count
            } else {
                (value + count - 1) % count
            }
        };
        match ROWS[menu.row] {
            Row::Clamps => menu.clamps = cycle(menu.clamps, levels),
            Row::Route => menu.route = cycle(menu.route, ROUTES.len()),
            Row::Depart => {}
        }
        sfx.write(Sfx::Select);
    }
    if advance_pressed(&keys) {
        if ROWS[menu.row] != Row::Depart {
            menu.row = ROWS.len() - 1;
            sfx.write(Sfx::Select);
            return;
        }
        let level = &content.balance.clamp_levels[menu.clamps];
        ship.clamps = level.secured;
        *voyage = Voyage::depart(
            &content.chart,
            ROUTES[menu.route],
            DEPARTURE.minute() + level.delay_minutes,
        );
        sfx.write(Sfx::Select);
        next.set(GameState::Voyage);
    }
}

fn draw_menu(
    menu: Res<PortMenu>,
    content: Res<Content>,
    mut labels: Query<(&RowLabel, &mut Label)>,
) {
    let level = &content.balance.clamp_levels[menu.clamps];
    for (row, mut text) in &mut labels {
        let selected = ROWS[menu.row] == row.0;
        let cursor = if selected { "> " } else { "  " };
        let body = match row.0 {
            Row::Clamps => format!(
                "{cursor}HATCH CLAMPS: {}  (SAIL {})",
                level.name,
                clock_text(DEPARTURE.minute() + level.delay_minutes)
                    .trim_start_matches("NOV 9 ")
                    .trim()
            ),
            Row::Route => {
                let name = content
                    .chart
                    .route(ROUTES[menu.route])
                    .map_or("", |route| route.name.as_str());
                format!("{cursor}ROUTE: {name}")
            }
            Row::Depart => format!("{cursor}SAIL"),
        };
        text.set(body);
        let color = if selected { Pal::Yellow } else { Pal::White };
        if text.color != color {
            text.color = color;
        }
    }
}

pub struct PortPlugin;

impl Plugin for PortPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Port), enter_port)
            .add_systems(
                Update,
                (port_input, draw_menu)
                    .chain()
                    .run_if(in_state(GameState::Port)),
            );
    }
}
