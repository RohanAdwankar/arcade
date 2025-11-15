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
const ROUND_DURATION: Duration = Duration::from_secs(30);

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
    timer_duration: Duration,
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
            status: "30s typing sprint · start typing to begin".into(),
            timer_duration: ROUND_DURATION,
        }
    }

    fn restart(&mut self) {
        self.prompt = generate_prompt(&mut self.rng);
        self.prompt_len = self.prompt.graphemes(true).count();
        self.typed.clear();
        self.typed_len = 0;
        self.started = None;
        self.finished = None;
        self.status = "30s typing sprint · start typing to begin".into();
    }

    fn ensure_prompt_capacity(&mut self) {
        if self.prompt_len.saturating_sub(self.typed_len) < 10 {
            let extra = generate_prompt(&mut self.rng);
            if !self.prompt.ends_with(' ') {
                self.prompt.push(' ');
            }
            self.prompt.push_str(&extra);
            self.prompt_len = self.prompt.graphemes(true).count();
        }
    }

    fn accuracy(&self) -> f64 {
        if self.typed.is_empty() {
            return 100.0;
        }
        let total = self.typed_len.max(1);
        let correct = self
            .typed
            .graphemes(true)
            .zip(self.prompt.graphemes(true))
            .filter(|(a, b)| a == b)
            .count();
        100.0 * correct as f64 / total as f64
    }

    fn finish_round(&mut self, elapsed: Duration) -> GameAction {
        if self.finished.is_some() {
            return GameAction::None;
        }
        let elapsed = elapsed
            .max(Duration::from_millis(100))
            .min(self.timer_duration);
        let minutes = elapsed.as_secs_f64() / 60.0;
        let wpm = if minutes > 0.0 {
            (self.typed_len as f64 / 5.0) / minutes
        } else {
            0.0
        };
        let acc = self.accuracy();
        self.status = format!(
            "Time! {:.1} WPM · {:.1}% accuracy · {} chars",
            wpm, acc, self.typed_len
        );
        let finish_time = self.started.unwrap_or_else(Instant::now) + elapsed;
        self.finished = Some(finish_time);
        if wpm > self.wpm_best {
            self.wpm_best = wpm;
            return GameAction::Record(
                StatRecord::new("WPM", format!("{wpm:.1}"), wpm),
                GameKind::Typing,
            );
        }
        GameAction::None
    }

    fn remaining_time(&self) -> Duration {
        if let Some(start) = self.started {
            if let Some(finished) = self.finished {
                let elapsed = finished
                    .saturating_duration_since(start)
                    .min(self.timer_duration);
                self.timer_duration.saturating_sub(elapsed)
            } else {
                let elapsed = Instant::now().saturating_duration_since(start);
                self.timer_duration.saturating_sub(elapsed)
            }
        } else {
            self.timer_duration
        }
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
        let remaining = self.remaining_time();
        lines.push(Line::from(format!(
            "Time left {:>5.1}s · Accuracy {:>5.1}% · Typed {} chars",
            remaining.as_secs_f64().max(0.0),
            self.accuracy(),
            self.typed_len
        )));
        lines.push(Line::from(format!("Best {:.1} WPM", self.wpm_best)));
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    }

    pub fn handle_event(&mut self, event: &Event) -> GameAction {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char(ch) if !ch.is_control() => {
                    if self.finished.is_none() {
                        if self.started.is_none() {
                            self.started = Some(Instant::now());
                            self.status = "Timer running · keep typing".into();
                        }
                        self.typed.push(ch);
                        self.typed_len += 1;
                        self.ensure_prompt_capacity();
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
                    if let Some(start) = self.started {
                        if let Some(_) = self.finished {
                            self.restart();
                        } else {
                            let elapsed = Instant::now().saturating_duration_since(start);
                            return self.finish_round(elapsed);
                        }
                    } else if self.finished.is_some() {
                        self.restart();
                    }
                }
                _ => {}
            }
        }
        GameAction::None
    }

    pub fn handle_tick(&mut self, now: Instant) -> GameAction {
        if let Some(start) = self.started {
            if self.finished.is_none() {
                let elapsed = now.saturating_duration_since(start);
                if elapsed >= self.timer_duration {
                    return self.finish_round(self.timer_duration);
                }
            }
        }
        GameAction::None
    }

    pub fn status_line(&self) -> String {
        if self.finished.is_some() {
            self.status.clone()
        } else {
            format!(
                "Time left {:>4.1}s · Typed {} chars · {:.1}% accuracy",
                self.remaining_time().as_secs_f64().max(0.0),
                self.typed_len,
                self.accuracy()
            )
        }
    }
}

fn generate_prompt(rng: &mut StdRng) -> String {
    (0..WORD_COUNT)
        .map(|_| WORD_BANK[rng.gen_range(0..WORD_BANK.len())])
        .collect::<Vec<_>>()
        .join(" ")
}
