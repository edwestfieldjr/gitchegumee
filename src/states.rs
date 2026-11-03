//! Top-level game flow.

use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Boot,
    Title,
    Port,
    Voyage,
    Helm,
    Event,
    Outcome,
    Memorial,
}

/// Phases of the event screen: read and choose, then read the result.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(GameState = GameState::Event)]
pub enum EventPhase {
    #[default]
    Choosing,
    Result,
}

/// Whether the game is running or paused. It applies on every screen, so it is
/// a state of its own and not a phase of one screen.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PauseState {
    #[default]
    Running,
    Paused,
}

/// Parts of the memorial: the words, then the bell.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(GameState = GameState::Memorial)]
pub enum MemorialPhase {
    #[default]
    Words,
    Bell,
}

/// True for the whole time the ship is under way, across layer changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnderWay;

impl ComputedStates for UnderWay {
    type SourceStates = GameState;

    fn compute(sources: GameState) -> Option<Self> {
        matches!(
            sources,
            GameState::Voyage | GameState::Helm | GameState::Event
        )
        .then_some(UnderWay)
    }
}

pub struct StatesPlugin;

impl Plugin for StatesPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .init_state::<PauseState>()
            .add_sub_state::<EventPhase>()
            .add_sub_state::<MemorialPhase>()
            .add_computed_state::<UnderWay>()
            .configure_sets(
                FixedUpdate,
                (
                    SimSet::Clock,
                    SimSet::Weather,
                    SimSet::Ship,
                    SimSet::Position,
                )
                    .chain(),
            );
    }
}

/// Order of the simulation within a fixed step.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimSet {
    Clock,
    Weather,
    Ship,
    Position,
}

/// The voyage clock runs on the voyage and helm layers, and stops for events.
pub fn sim_running(state: Res<State<GameState>>) -> bool {
    matches!(state.get(), GameState::Voyage | GameState::Helm)
}
