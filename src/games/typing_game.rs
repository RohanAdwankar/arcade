use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode};
use rand::{Rng, SeedableRng, rngs::StdRng};
use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{GameAction, GameKind, StatRecord};

const PROMPTS: &[&str] = &[
    "the quick brown fox jumps over the lazy dog",
    "vim motions keep your fingers on home row",
    "terminal user interfaces can be surprisingly fun",
    "memory games sharpen focus during long workdays",
];

#[derive(Debug)]
pub struct TypingState {
    prompt: &'static str,
    typed: String,
    rng: StdRng,
    started: Option<Instant>,
    finished: Option<Instant>,
    wpm_best: f64,
    status: String,
}

impl TypingState {
    pub fn new() -> Self {
        let mut rng = StdRng::from_entropy();
        let prompt = PROMPTS[rng.gen_range(0..PROMPTS.len())];
        Self {
            prompt,
            typed: String::new(),
            rng,
            started: None,
            finished: None,
            wpm_best: 0.0,
            status: "Type the prompt".into(),
        }
    }

    fn restart(&mut self) {
        self.prompt = PROMPTS[self.rng.gen_range(0..PROMPTS.len())];
        self.typed.clear();
        self.started = None;
        self.finished = None;
        self.status = "Type the prompt".into();
    }

    fn complete(&mut self) -> GameAction {
        if self.finished.is_some() {
            return GameAction::None;
        }
        self.finished = Some(Instant::now());
        let elapsed = self
            .started
            .map(|start| self.finished.unwrap().saturating_duration_since(start))
            .unwrap_or(Duration::from_millis(1));
        let minutes = elapsed.as_secs_f64() / 60.0;
        let wpm = (self.typed.len() as f64 / 5.0) / minutes.max(0.01);
        self.status = format!("Finished! {:.1} WPM", wpm);
        if wpm > self.wpm_best {
            self.wpm_best = wpm;
            return GameAction::Record(
                StatRecord {
                    label: "WPM".into(),
                    value: format!("{wpm:.1}"),
                },
                GameKind::Typing,
            );
        }
        GameAction::None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Typing")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines = vec![Line::from(self.prompt)];
        lines.push(Line::from(format!("› {}", self.typed)));
        lines.push(Line::from(self.status.as_str()));
        lines.push(Line::from(format!("Best {:.1} WPM", self.wpm_best)));
        frame.render_widget(Paragraph::new(lines), inner);
    }

    pub fn handle_event(&mut self, event: &Event) -> GameAction {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char(ch) if !ch.is_control() => {
                    if self.finished.is_none() && self.typed.len() < self.prompt.len() {
                        if self.started.is_none() {
                            self.started = Some(Instant::now());
                        }
                        self.typed.push(ch);
                        if self.typed.len() == self.prompt.len() {
                            return self.complete();
                        }
                    }
                }
                KeyCode::Backspace => {
                    if self.finished.is_none() {
                        self.typed.pop();
                    }
                }
                KeyCode::Enter => {
                    if self.finished.is_some() {
                        self.restart();
                    } else if self.typed.trim_end() == self.prompt {
                        return self.complete();
                    } else {
                        self.status = "Keep typing until it matches".into();
                    }
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
        self.status.clone()
    }
}
