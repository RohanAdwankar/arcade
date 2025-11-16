use std::collections::HashSet;
use std::time::Instant;

use crossterm::event::{Event, KeyCode};
use rand::{Rng, SeedableRng, rngs::StdRng};
use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{GameAction, GameKind, StatRecord};

const WORDS: &[&str] = &[
    "apple", "galaxy", "harbor", "quantum", "saffron", "vector", "marble", "amber", "citadel",
    "nebula", "orchid", "raven", "timber", "glacier", "summit", "horizon", "lantern", "pioneer",
    "anthem", "compass",
];

#[derive(Debug)]
pub struct VerbalMemoryState {
    rng: StdRng,
    seen: HashSet<&'static str>,
    current: &'static str,
    score: u32,
    lives: u8,
    best: u32,
    status: String,
    pending_best: Option<u32>,
}

impl VerbalMemoryState {
    pub fn new() -> Self {
        let mut rng = StdRng::from_entropy();
        let idx = rng.gen_range(0..WORDS.len());
        Self {
            rng,
            seen: HashSet::new(),
            current: WORDS[idx],
            score: 0,
            lives: 3,
            best: 0,
            status: "Press l for NEW, h for SEEN".into(),
            pending_best: None,
        }
    }

    fn next_word(&mut self) {
        let idx = self.rng.gen_range(0..WORDS.len());
        self.current = WORDS[idx];
    }

    fn evaluate(&mut self, guess_seen: bool) -> GameAction {
        if self.lives == 0 {
            return GameAction::None;
        }
        let was_seen = self.seen.contains(&self.current);
        if guess_seen == was_seen {
            self.score += 1;
            self.status = "Correct".into();
            self.seen.insert(self.current);
            if self.score > self.best {
                self.best = self.score;
                self.pending_best = Some(self.best);
            }
        } else {
            self.status = "Wrong!".into();
            self.lives = self.lives.saturating_sub(1);
            if self.lives == 0 {
                self.status = format!("Game over · final score {}", self.score);
                let action = self.flush_pending_record();
                self.next_word();
                return action;
            }
        }
        self.next_word();
        GameAction::None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Verbal Memory")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines = vec![Line::from(format!("Lives: {}", self.lives))];
        lines.push(Line::from(format!(
            "Score: {} (best {})",
            self.score, self.best
        )));
        if self.lives > 0 {
            lines.push(Line::from("Seen this word before?"));
            lines.push(Line::from(format!("› {}", self.current)));
        } else {
            lines.push(Line::from("Press enter to restart"));
        }
        lines.push(Line::from(self.status.as_str()));
        frame.render_widget(Paragraph::new(lines), inner);
    }

    pub fn handle_event(&mut self, event: &Event) -> GameAction {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('h') | KeyCode::Left => return self.evaluate(true),
                KeyCode::Char('l') | KeyCode::Right => return self.evaluate(false),
                KeyCode::Enter if self.lives == 0 => {
                    self.pending_best = None;
                    *self = Self::new();
                }
                _ => {}
            }
        }
        GameAction::None
    }

    pub fn handle_tick(&mut self, _now: Instant) -> GameAction {
        GameAction::None
    }

    pub fn status_line(&self) -> String {
        format!("Score {} · Lives {}", self.score, self.lives)
    }

    fn flush_pending_record(&mut self) -> GameAction {
        if let Some(score) = self.pending_best.take() {
            let record = StatRecord::new("Score", score.to_string(), score as f64);
            GameAction::Record(record, GameKind::VerbalMemory)
        } else {
            GameAction::None
        }
    }
}
