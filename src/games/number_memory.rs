use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode};
use rand::{Rng, SeedableRng, rngs::StdRng};
use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{GameAction, GameKind, StatRecord};

const REVEAL_TIME: Duration = Duration::from_secs(2);

#[derive(Debug)]
pub struct NumberMemoryState {
    round: usize,
    best_round: usize,
    number: String,
    input: String,
    phase: Phase,
    status: String,
    rng: StdRng,
}

#[derive(Debug)]
enum Phase {
    Ready,
    Reveal { since: Instant },
    Recall,
    Result,
}

impl NumberMemoryState {
    pub fn new() -> Self {
        Self {
            round: 1,
            best_round: 0,
            number: String::new(),
            input: String::new(),
            phase: Phase::Ready,
            status: "Press enter to reveal the number".into(),
            rng: StdRng::from_entropy(),
        }
    }

    fn build_number(&mut self) {
        self.number = (0..self.round)
            .map(|_| char::from(b'0' + self.rng.gen_range(0..10) as u8))
            .collect();
        self.phase = Phase::Reveal {
            since: Instant::now(),
        };
        self.status = format!("Memorize {} digits", self.round);
    }

    fn handle_submission(&mut self) -> GameAction {
        if self.input == self.number {
            self.status = "Correct!".into();
            self.round += 1;
            self.phase = Phase::Result;
            self.input.clear();
            if self.round - 1 > self.best_round {
                self.best_round = self.round - 1;
                return GameAction::Record(
                    StatRecord::new(
                        "Digits",
                        self.best_round.to_string(),
                        self.best_round as f64,
                    ),
                    GameKind::NumberMemory,
                );
            }
        } else {
            self.status = format!("Oops! It was {}", self.number);
            self.round = 1;
            self.phase = Phase::Result;
            self.input.clear();
        }
        GameAction::None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Number Memory")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines = vec![Line::from(format!("Round: {} digits", self.round))];
        lines.push(Line::from(format!("Best: {}", self.best_round)));
        match self.phase {
            Phase::Recall => {
                lines.push(Line::from(format!("Type: {}", self.input)));
            }
            Phase::Reveal { .. } => lines.push(Line::from(format!("Number: {}", self.number))),
            _ => {}
        }
        lines.push(Line::from(self.status.as_str()));
        frame.render_widget(Paragraph::new(lines), inner);
    }

    pub fn handle_event(&mut self, event: &Event) -> GameAction {
        if let Event::Key(key) = event {
            match (&self.phase, key.code) {
                (Phase::Ready | Phase::Result, KeyCode::Enter) => {
                    self.build_number();
                }
                (Phase::Recall, KeyCode::Enter) => return self.handle_submission(),
                (Phase::Recall, KeyCode::Backspace) => {
                    self.input.pop();
                }
                (Phase::Recall, KeyCode::Char(ch)) if ch.is_ascii_digit() => {
                    self.input.push(ch);
                }
                _ => {}
            }
        }
        GameAction::None
    }

    pub fn handle_tick(&mut self, now: Instant) -> GameAction {
        if let Phase::Reveal { since } = self.phase {
            if now.duration_since(since) >= REVEAL_TIME {
                self.phase = Phase::Recall;
                self.status = "Type the number and press enter".into();
            }
        }
        GameAction::None
    }

    pub fn status_line(&self) -> String {
        self.status.clone()
    }
}
