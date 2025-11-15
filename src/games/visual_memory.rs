use std::collections::HashSet;
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode};
use rand::{SeedableRng, rngs::StdRng, seq::SliceRandom};
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{GameAction, GameKind, StatRecord, navigation::VimMotionState};

const GRID: usize = 5;
const BASE_CELLS: usize = 3;
const REVEAL: Duration = Duration::from_secs(2);

#[derive(Debug)]
pub struct VisualMemoryState {
    pattern: HashSet<(usize, usize)>,
    guesses: HashSet<(usize, usize)>,
    cursor: (usize, usize),
    rng: StdRng,
    round: usize,
    best: usize,
    lives: u8,
    phase: Phase,
    status: String,
    nav: VimMotionState,
}

#[derive(Debug, Clone, Copy)]
enum Phase {
    Reveal { since: Instant },
    Recall,
    Result,
}

impl VisualMemoryState {
    pub fn new() -> Self {
        let rng = StdRng::from_entropy();
        let mut state = Self {
            pattern: HashSet::new(),
            guesses: HashSet::new(),
            cursor: (0, 0),
            rng,
            round: 1,
            best: 0,
            lives: 3,
            phase: Phase::Reveal {
                since: Instant::now(),
            },
            status: "Memorize the pattern".into(),
            nav: VimMotionState::default(),
        };
        state.generate_pattern();
        state
    }

    fn generate_pattern(&mut self) {
        self.pattern.clear();
        self.guesses.clear();
        self.cursor = (0, 0);
        self.nav.clear();
        let mut cells = Vec::new();
        for x in 0..GRID {
            for y in 0..GRID {
                cells.push((x, y));
            }
        }
        cells.shuffle(&mut self.rng);
        let count = BASE_CELLS + self.round;
        self.pattern.extend(cells.into_iter().take(count));
        self.phase = Phase::Reveal {
            since: Instant::now(),
        };
        self.status = format!("Round {} · memorize", self.round);
    }

    fn toggle(&mut self) {
        if matches!(self.phase, Phase::Recall) {
            self.nav.clear();
            if !self.guesses.insert(self.cursor) {
                self.guesses.remove(&self.cursor);
            }
        }
    }

    fn submit(&mut self) -> GameAction {
        if !matches!(self.phase, Phase::Recall) {
            return GameAction::None;
        }
        if self.guesses == self.pattern {
            self.status = "Correct".into();
            self.round += 1;
            self.phase = Phase::Result;
            if self.round - 1 > self.best {
                self.best = self.round - 1;
                let record = StatRecord::new("Round", self.best.to_string(), self.best as f64);
                self.generate_pattern();
                return GameAction::Record(record, GameKind::VisualMemory);
            }
            self.generate_pattern();
        } else {
            self.status = "Not quite".into();
            self.lives = self.lives.saturating_sub(1);
            self.phase = Phase::Result;
            if self.lives == 0 {
                self.status = format!("Out of lives · best {}", self.best);
            }
            self.round = 1;
            self.generate_pattern();
        }
        GameAction::None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Visual Memory")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines = vec![Line::from(format!(
            "Round {} (best {}) · Lives {}",
            self.round, self.best, self.lives
        ))];
        lines.push(Line::from(self.status.as_str()));
        for y in 0..GRID {
            let mut spans = Vec::with_capacity(GRID * 2);
            for x in 0..GRID {
                let filled = match self.phase {
                    Phase::Reveal { .. } => self.pattern.contains(&(x, y)),
                    _ => self.guesses.contains(&(x, y)),
                };
                let ch = if filled { "■" } else { "·" };
                let style = if (x, y) == self.cursor {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                spans.push(Span::styled(ch, style));
                spans.push(Span::raw(" "));
            }
            lines.push(Line::from(spans));
        }
        frame.render_widget(Paragraph::new(lines), inner);
    }

    pub fn handle_event(&mut self, event: &Event) -> GameAction {
        if let Event::Key(key) = event {
            if self.nav.handle_key(key, &mut self.cursor, GRID, GRID) {
                return GameAction::None;
            }

            match key.code {
                KeyCode::Char(' ') | KeyCode::Enter => self.toggle(),
                KeyCode::Char('s') | KeyCode::Char('S') => return self.submit(),
                KeyCode::Esc => self.nav.clear(),
                _ => {}
            }
        }
        GameAction::None
    }

    pub fn handle_tick(&mut self, now: Instant) -> GameAction {
        if let Phase::Reveal { since } = self.phase {
            if now.duration_since(since) >= REVEAL {
                self.phase = Phase::Recall;
                self.status = "Toggle with space/enter · submit with s".into();
            }
        }
        GameAction::None
    }

    pub fn status_line(&self) -> String {
        let base = format!("Round {} · Lives {}", self.round, self.lives);
        if let Some(count) = self.nav.prefix() {
            format!("{} · count {}", base, count)
        } else {
            base
        }
    }
}
