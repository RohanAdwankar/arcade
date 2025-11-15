use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode};
use rand::{Rng, SeedableRng, rngs::StdRng};
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use unicode_segmentation::UnicodeSegmentation;

use super::{GameAction, GameKind, StatRecord};

const WORD_BANK: &[&str] = &[
    "group",
    "still",
    "system",
    "program",
    "focus",
    "public",
    "school",
    "course",
    "memory",
    "benchmark",
    "terminal",
    "cursor",
    "future",
    "design",
    "engineer",
    "policy",
    "market",
    "during",
    "order",
    "friend",
    "garden",
    "energy",
    "space",
    "motion",
    "finger",
    "option",
    "window",
    "screen",
    "signal",
    "random",
    "summer",
    "winter",
    "breeze",
    "supper",
    "coffee",
    "studio",
    "search",
    "network",
    "planet",
    "rocket",
    "python",
    "rust",
    "editor",
    "monkey",
    "type",
    "tactile",
    "canvas",
    "offset",
    "target",
    "driver",
    "trivia",
    "memory",
    "visual",
    "number",
    "chimp",
    "sequence",
    "typing",
    "sprint",
    "ocean",
    "desert",
    "forest",
    "matrix",
    "cypher",
    "future",
    "puzzle",
    "legend",
    "signal",
    "plasma",
    "vector",
    "thread",
    "buffer",
    "syntax",
    "module",
    "kernel",
    "packet",
    "girder",
    "window",
    "screen",
    "layout",
];
const WORD_COUNT: usize = 80;

#[derive(Debug)]
pub struct TypingState {
    prompt: String,
    prompt_len: usize,
    typed: String,
    typed_len: usize,
    rng: StdRng,
    started: Option<Instant>,
    finished: Option<Instant>,
    wpm_best: f64,
    status: String,
}

impl TypingState {
    pub fn new() -> Self {
        let mut rng = StdRng::from_entropy();
        let prompt = generate_prompt(&mut rng);
        let prompt_len = prompt.graphemes(true).count();
        Self {
            prompt,
            prompt_len,
            typed: String::new(),
            typed_len: 0,
            rng,
            started: None,
            finished: None,
            wpm_best: 0.0,
            status: "Type the paragraph".into(),
        }
    }

    fn restart(&mut self) {
        self.prompt = generate_prompt(&mut self.rng);
        self.prompt_len = self.prompt.graphemes(true).count();
        self.typed.clear();
        self.typed_len = 0;
        self.started = None;
        self.finished = None;
        self.status = "Type the paragraph".into();
    }

    fn accuracy(&self) -> f64 {
        let total = self.prompt_len.max(1);
        let correct = self
            .typed
            .graphemes(true)
            .zip(self.prompt.graphemes(true))
            .filter(|(a, b)| a == b)
            .count();
        100.0 * correct as f64 / total as f64
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
        let wpm = (self.prompt_len as f64 / 5.0) / minutes.max(0.01);
        let acc = self.accuracy();
        self.status = format!("Finished! {:.1} WPM · {:.1}% accuracy", wpm, acc);
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

        let prompt_chars: Vec<String> =
            self.prompt.graphemes(true).map(|g| g.to_string()).collect();
        let typed_chars: Vec<String> = self.typed.graphemes(true).map(|g| g.to_string()).collect();
        let mut spans = Vec::with_capacity(prompt_chars.len());
        for (idx, ch) in prompt_chars.iter().enumerate() {
            let style = if idx < typed_chars.len() {
                if typed_chars[idx] == *ch {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::UNDERLINED)
                }
            } else if idx == typed_chars.len() {
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            spans.push(Span::styled(ch.clone(), style));
        }

        let mut lines = vec![Line::from(spans)];
        lines.push(Line::from(self.status.as_str()));
        lines.push(Line::from(format!("Best {:.1} WPM", self.wpm_best)));
        let accuracy = if self.typed.is_empty() {
            100.0
        } else {
            self.accuracy()
        };
        lines.push(Line::from(format!(
            "Typed {} / {} chars · {:.1}% accuracy",
            self.typed_len, self.prompt_len, accuracy
        )));
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    }

    pub fn handle_event(&mut self, event: &Event) -> GameAction {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char(ch) if !ch.is_control() => {
                    if self.finished.is_none() && self.typed_len < self.prompt_len {
                        if self.started.is_none() {
                            self.started = Some(Instant::now());
                        }
                        self.typed.push(ch);
                        self.typed_len += 1;
                        if self.typed_len == self.prompt_len {
                            return self.complete();
                        }
                    }
                }
                KeyCode::Backspace => {
                    if self.finished.is_none() {
                        if self.typed.pop().is_some() {
                            self.typed_len = self.typed_len.saturating_sub(1);
                        }
                    }
                }
                KeyCode::Enter => {
                    if self.finished.is_some() {
                        self.restart();
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
        if self.finished.is_some() {
            self.status.clone()
        } else {
            format!("Typed {} / {} characters", self.typed_len, self.prompt_len)
        }
    }
}

fn generate_prompt(rng: &mut StdRng) -> String {
    (0..WORD_COUNT)
        .map(|_| WORD_BANK[rng.gen_range(0..WORD_BANK.len())])
        .collect::<Vec<_>>()
        .join(" ")
}
