use std::time::Instant;

use crossterm::event::{Event, KeyCode};
use rand::{Rng, SeedableRng, rngs::StdRng};
use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{GameAction, GameKind, StatRecord};

#[derive(Debug)]
pub struct SequenceState {
    sequence: Vec<Direction>,
    index: usize,
    best: usize,
    status: String,
    rng: StdRng,
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn glyph(self) -> &'static str {
        match self {
            Direction::Up => "↑",
            Direction::Down => "↓",
            Direction::Left => "←",
            Direction::Right => "→",
        }
    }

    fn matches_key(self, key: &KeyCode) -> bool {
        match (self, key) {
            (Direction::Up, KeyCode::Up | KeyCode::Char('k')) => true,
            (Direction::Down, KeyCode::Down | KeyCode::Char('j')) => true,
            (Direction::Left, KeyCode::Left | KeyCode::Char('h')) => true,
            (Direction::Right, KeyCode::Right | KeyCode::Char('l')) => true,
            _ => false,
        }
    }
}

impl SequenceState {
    pub fn new() -> Self {
        let rng = StdRng::from_entropy();
        let mut state = Self {
            sequence: Vec::new(),
            index: 0,
            best: 0,
            status: "Use hjkl/arrow keys to repeat the pattern".into(),
            rng,
        };
        let dir = state.random_direction();
        state.sequence.push(dir);
        state
    }

    fn random_direction(&mut self) -> Direction {
        match self.rng.gen_range(0..4) {
            0 => Direction::Up,
            1 => Direction::Right,
            2 => Direction::Down,
            _ => Direction::Left,
        }
    }

    fn reset(&mut self) {
        self.sequence.clear();
        let dir = self.random_direction();
        self.sequence.push(dir);
        self.index = 0;
        self.status = "Pattern reset".into();
    }

    fn extend_sequence(&mut self) -> GameAction {
        let completed = self.sequence.len();
        if completed > self.best {
            self.best = completed;
            let record = StatRecord {
                label: "Pattern",
                value: format!("{} steps", completed),
            };
            let dir = self.random_direction();
            self.sequence.push(dir);
            self.index = 0;
            self.status = format!("Round cleared! Sequence length {}", self.sequence.len());
            return GameAction::Record(record, GameKind::Sequence);
        }
        let dir = self.random_direction();
        self.sequence.push(dir);
        self.index = 0;
        self.status = format!("Sequence extended to {}", self.sequence.len());
        GameAction::None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Sequence Memory")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let arrow_string = self
            .sequence
            .iter()
            .enumerate()
            .map(|(idx, dir)| {
                if idx == self.index {
                    format!("[{}]", dir.glyph())
                } else {
                    dir.glyph().to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        let mut lines = vec![
            Line::from("Remember the directions"),
            Line::from(arrow_string),
        ];
        lines.push(Line::from(format!(
            "Progress {}/{}",
            self.index + 1,
            self.sequence.len()
        )));
        lines.push(Line::from(format!("Session best: {}", self.best)));
        lines.push(Line::from(self.status.as_str()));
        frame.render_widget(Paragraph::new(lines), inner);
    }

    pub fn handle_event(&mut self, event: &Event) -> GameAction {
        if let Event::Key(key) = event {
            if let Some(expected) = self.sequence.get(self.index).copied() {
                if expected.matches_key(&key.code) {
                    self.index += 1;
                    if self.index == self.sequence.len() {
                        return self.extend_sequence();
                    }
                } else if matches!(
                    key.code,
                    KeyCode::Char('h' | 'j' | 'k' | 'l')
                        | KeyCode::Left
                        | KeyCode::Right
                        | KeyCode::Up
                        | KeyCode::Down
                ) {
                    self.status = "Wrong move! Starting over".into();
                    if self.sequence.len().saturating_sub(1) > self.best {
                        self.best = self.sequence.len() - 1;
                    }
                    self.reset();
                }
            }
        }
        GameAction::None
    }

    pub fn handle_tick(&mut self, _now: Instant) -> GameAction {
        GameAction::None
    }

    pub fn status_line(&self) -> String {
        format!(
            "Sequence length {} · progress {}/{}",
            self.sequence.len(),
            self.index + 1,
            self.sequence.len()
        )
    }
}
