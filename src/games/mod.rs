use std::time::Instant;

use crossterm::event::Event;
use ratatui::Frame;
use ratatui::prelude::*;

pub mod aim;
pub mod chimp_test;
pub mod number_memory;
pub mod reaction;
pub mod sequence;
pub mod typing_game;
pub mod verbal_memory;
pub mod visual_memory;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameKind {
    Reaction,
    Sequence,
    AimTrainer,
    NumberMemory,
    VerbalMemory,
    ChimpTest,
    VisualMemory,
    Typing,
}

impl GameKind {
    pub const ALL: [GameKind; 8] = [
        GameKind::Reaction,
        GameKind::Sequence,
        GameKind::AimTrainer,
        GameKind::NumberMemory,
        GameKind::VerbalMemory,
        GameKind::ChimpTest,
        GameKind::VisualMemory,
        GameKind::Typing,
    ];

    pub fn title(self) -> &'static str {
        match self {
            GameKind::Reaction => "Reaction Time",
            GameKind::Sequence => "Sequence Memory",
            GameKind::AimTrainer => "Aim Trainer",
            GameKind::NumberMemory => "Number Memory",
            GameKind::VerbalMemory => "Verbal Memory",
            GameKind::ChimpTest => "Chimp Test",
            GameKind::VisualMemory => "Visual Memory",
            GameKind::Typing => "Typing",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            GameKind::Reaction => "Tap as soon as the screen flashes go.",
            GameKind::Sequence => "Repeat the growing list of directions.",
            GameKind::AimTrainer => "Move the crosshair and tag the target.",
            GameKind::NumberMemory => "Memorize and recall an ever longer number.",
            GameKind::VerbalMemory => "Decide if the word was seen before.",
            GameKind::ChimpTest => "Select numbers in ascending order.",
            GameKind::VisualMemory => "Remember highlighted tiles.",
            GameKind::Typing => "Type the prompt as quickly as you can.",
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatRecord {
    pub label: &'static str,
    pub value: String,
}

#[derive(Debug, Clone)]
pub enum GameAction {
    None,
    Record(StatRecord, GameKind),
}

#[derive(Debug)]
pub enum GameState {
    Reaction(reaction::ReactionState),
    Sequence(sequence::SequenceState),
    Aim(aim::AimTrainerState),
    Number(number_memory::NumberMemoryState),
    Verbal(verbal_memory::VerbalMemoryState),
    Chimp(chimp_test::ChimpTestState),
    Visual(visual_memory::VisualMemoryState),
    Typing(typing_game::TypingState),
}

impl GameState {
    pub fn new(kind: GameKind) -> Self {
        match kind {
            GameKind::Reaction => Self::Reaction(reaction::ReactionState::new()),
            GameKind::Sequence => Self::Sequence(sequence::SequenceState::new()),
            GameKind::AimTrainer => Self::Aim(aim::AimTrainerState::new()),
            GameKind::NumberMemory => Self::Number(number_memory::NumberMemoryState::new()),
            GameKind::VerbalMemory => Self::Verbal(verbal_memory::VerbalMemoryState::new()),
            GameKind::ChimpTest => Self::Chimp(chimp_test::ChimpTestState::new()),
            GameKind::VisualMemory => Self::Visual(visual_memory::VisualMemoryState::new()),
            GameKind::Typing => Self::Typing(typing_game::TypingState::new()),
        }
    }

    pub fn kind(&self) -> GameKind {
        match self {
            GameState::Reaction(_) => GameKind::Reaction,
            GameState::Sequence(_) => GameKind::Sequence,
            GameState::Aim(_) => GameKind::AimTrainer,
            GameState::Number(_) => GameKind::NumberMemory,
            GameState::Verbal(_) => GameKind::VerbalMemory,
            GameState::Chimp(_) => GameKind::ChimpTest,
            GameState::Visual(_) => GameKind::VisualMemory,
            GameState::Typing(_) => GameKind::Typing,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        match self {
            GameState::Reaction(state) => state.render(frame, area),
            GameState::Sequence(state) => state.render(frame, area),
            GameState::Aim(state) => state.render(frame, area),
            GameState::Number(state) => state.render(frame, area),
            GameState::Verbal(state) => state.render(frame, area),
            GameState::Chimp(state) => state.render(frame, area),
            GameState::Visual(state) => state.render(frame, area),
            GameState::Typing(state) => state.render(frame, area),
        }
    }

    pub fn handle_event(&mut self, event: &Event) -> GameAction {
        match self {
            GameState::Reaction(state) => state.handle_event(event),
            GameState::Sequence(state) => state.handle_event(event),
            GameState::Aim(state) => state.handle_event(event),
            GameState::Number(state) => state.handle_event(event),
            GameState::Verbal(state) => state.handle_event(event),
            GameState::Chimp(state) => state.handle_event(event),
            GameState::Visual(state) => state.handle_event(event),
            GameState::Typing(state) => state.handle_event(event),
        }
    }

    pub fn handle_tick(&mut self, now: Instant) -> GameAction {
        match self {
            GameState::Reaction(state) => state.handle_tick(now),
            GameState::Sequence(state) => state.handle_tick(now),
            GameState::Aim(state) => state.handle_tick(now),
            GameState::Number(state) => state.handle_tick(now),
            GameState::Verbal(state) => state.handle_tick(now),
            GameState::Chimp(state) => state.handle_tick(now),
            GameState::Visual(state) => state.handle_tick(now),
            GameState::Typing(state) => state.handle_tick(now),
        }
    }

    pub fn status_line(&self) -> String {
        match self {
            GameState::Reaction(state) => state.status_line(),
            GameState::Sequence(state) => state.status_line(),
            GameState::Aim(state) => state.status_line(),
            GameState::Number(state) => state.status_line(),
            GameState::Verbal(state) => state.status_line(),
            GameState::Chimp(state) => state.status_line(),
            GameState::Visual(state) => state.status_line(),
            GameState::Typing(state) => state.status_line(),
        }
    }
}
