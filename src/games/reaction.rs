use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode};
use rand::{Rng, SeedableRng, rngs::StdRng};
use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{GameAction, GameKind, StatRecord};

const MIN_DELAY_MS: u64 = 1200;
const MAX_DELAY_MS: u64 = 3200;

#[derive(Debug)]
pub struct ReactionState {
    phase: Phase,
    rng: StdRng,
    last_result: Option<u128>,
    best_ms: Option<u128>,
    status: String,
}

#[derive(Debug, Clone, Copy)]
enum Phase {
    Idle,
    Waiting { start: Instant, delay: Duration },
    Go { start: Instant },
    Result,
}

impl ReactionState {
    pub fn new() -> Self {
        Self {
            phase: Phase::Idle,
            rng: StdRng::from_entropy(),
            last_result: None,
            best_ms: None,
            status: "Press enter to start".into(),
        }
    }

    fn start_wait(&mut self) {
        let delay = self.rng.gen_range(MIN_DELAY_MS..=MAX_DELAY_MS);
        self.phase = Phase::Waiting {
            start: Instant::now(),
            delay: Duration::from_millis(delay),
        };
        self.status = "Wait for GO...".into();
    }

    fn action_key(code: &KeyCode) -> bool {
        matches!(
            code,
            KeyCode::Enter
                | KeyCode::Char(' ')
                | KeyCode::Char('h')
                | KeyCode::Char('j')
                | KeyCode::Char('k')
                | KeyCode::Char('l')
        )
    }

    fn finish_attempt(&mut self, elapsed: Option<Duration>) -> Option<GameAction> {
        self.phase = Phase::Result;
        match elapsed {
            Some(duration) => {
                let ms = duration.as_millis();
                self.last_result = Some(ms);
                self.status = format!("{ms} ms · press enter to retry");
                if self.best_ms.map(|best| ms < best).unwrap_or(true) {
                    self.best_ms = Some(ms);
                    return Some(GameAction::Record(
                        StatRecord {
                            label: "Best".into(),
                            value: format!("{ms} ms"),
                        },
                        GameKind::Reaction,
                    ));
                }
            }
            None => {
                self.last_result = None;
                self.status = "Too soon! press enter to restart".into();
            }
        }
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let mut lines = vec![Line::from("Reaction Time")];
        match self.phase {
            Phase::Go { .. } => lines.push(Line::from("GO!")),
            Phase::Waiting { .. } => lines.push(Line::from("...")),
            _ => {}
        }
        if let Some(ms) = self.last_result {
            lines.push(Line::from(format!("Last: {ms} ms")));
        }
        if let Some(best) = self.best_ms {
            lines.push(Line::from(format!("Session best: {best} ms")));
        }
        lines.push(Line::from(self.status.as_str()));

        let block = Block::default()
            .title("Reaction Time")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(Paragraph::new(lines), inner);
    }

    pub fn handle_event(&mut self, event: &Event) -> GameAction {
        if let Event::Key(key) = event {
            if !Self::action_key(&key.code) {
                return GameAction::None;
            }
            match self.phase {
                Phase::Idle | Phase::Result => {
                    if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                        self.start_wait();
                    }
                }
                Phase::Waiting { .. } => {
                    if let Some(action) = self.finish_attempt(None) {
                        return action;
                    }
                }
                Phase::Go { start } => {
                    if let Some(action) = self.finish_attempt(Some(Instant::now() - start)) {
                        return action;
                    }
                }
            }
        }
        GameAction::None
    }

    pub fn handle_tick(&mut self, now: Instant) -> GameAction {
        if let Phase::Waiting { start, delay } = self.phase {
            if now.duration_since(start) >= delay {
                self.phase = Phase::Go { start: now };
                self.status = "Tap now!".into();
            }
        }
        GameAction::None
    }

    pub fn status_line(&self) -> String {
        self.status.clone()
    }
}
