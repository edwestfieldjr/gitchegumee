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
            .add_sub_state::<EventPhase>()
            .add_computed_state::<UnderWay>();
    }
}
