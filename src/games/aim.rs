use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode};
use rand::{Rng, SeedableRng, rngs::StdRng};
use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{GameAction, GameKind, StatRecord, navigation::VimMotionState};

const GRID: usize = 16;
const TARGETS: u32 = 10;

#[derive(Debug)]
pub struct AimTrainerState {
    cursor: (usize, usize),
    target: (usize, usize),
    hits: u32,
    total_time: Duration,
    spawn: Instant,
    run_start: Instant,
    rng: StdRng,
    finished: bool,
    best_total_ms: Option<f64>,
    status: String,
    nav: VimMotionState,
}

impl AimTrainerState {
    pub fn new() -> Self {
        let mut rng = StdRng::from_entropy();
        let target = (rng.gen_range(0..GRID), rng.gen_range(0..GRID));
        Self {
            cursor: (GRID / 2, GRID / 2),
            target,
            hits: 0,
            total_time: Duration::ZERO,
            spawn: Instant::now(),
            run_start: Instant::now(),
            rng,
            finished: false,
            best_total_ms: None,
            status: "Move with hjkl · counts + 0/$/gg/G work".into(),
            nav: VimMotionState::default(),
        }
    }

    fn spawn_target(&mut self) {
        self.target = (self.rng.gen_range(0..GRID), self.rng.gen_range(0..GRID));
        self.spawn = Instant::now();
    }

    fn tag(&mut self) -> GameAction {
        if self.finished {
            return GameAction::None;
        }
        if self.cursor == self.target {
            let elapsed = Instant::now() - self.spawn;
            self.total_time += elapsed;
            self.hits += 1;
            if self.hits == TARGETS {
                self.finished = true;
                let total_ms = self.total_time.as_secs_f64() * 1000.0;
                self.status = format!(
                    "Complete! total {:.0} ms (avg {:.0} ms)",
                    total_ms,
                    total_ms / TARGETS as f64
                );
                if self
                    .best_total_ms
                    .map(|best| total_ms < best)
                    .unwrap_or(true)
                {
                    self.best_total_ms = Some(total_ms);
                    return GameAction::Record(
                        StatRecord::new("Total", format!("{total_ms:.0} ms"), total_ms),
                        GameKind::AimTrainer,
                    );
                }
            } else {
                self.status = format!("Target {}/{}", self.hits + 1, TARGETS);
                self.spawn_target();
            }
        } else {
            self.status = "Missed – move onto the target".into();
        }
        GameAction::None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Aim Trainer")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let elapsed_ms = if self.finished {
            self.total_time.as_secs_f64() * 1000.0
        } else {
            (Instant::now() - self.run_start).as_secs_f64() * 1000.0
        };
        let mut lines = vec![Line::from(format!(
            "Hits: {}/{} · Elapsed {:.0} ms",
            self.hits, TARGETS, elapsed_ms
        ))];
        let status_text = if let Some(count) = self.nav.prefix() {
            format!("{} · count {}", self.status, count)
        } else {
            self.status.clone()
        };
        lines.push(Line::from(status_text));
        if let Some(best) = self.best_total_ms {
            lines.push(Line::from(format!("Best run: {:.0} ms", best)));
        }

        let mut grid_lines = Vec::new();
        for y in 0..GRID {
            let mut row = String::new();
            for x in 0..GRID {
                if (x, y) == self.cursor {
                    if (x, y) == self.target {
                        row.push('✚');
                    } else {
                        row.push('⌖');
                    }
                } else if (x, y) == self.target {
                    row.push('●');
                } else {
                    row.push('·');
                }
                row.push(' ');
            }
            grid_lines.push(Line::from(row));
        }
        lines.extend(grid_lines);
        frame.render_widget(Paragraph::new(lines), inner);
    }

    pub fn handle_event(&mut self, event: &Event) -> GameAction {
        if let Event::Key(key) = event {
            if self.nav.handle_key(key, &mut self.cursor, GRID, GRID) {
                return GameAction::None;
            }

            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.nav.clear();
                    return self.tag();
                }
                KeyCode::Esc => self.nav.clear(),
                _ => {}
            }
        }
        GameAction::None
    }

    pub fn handle_tick(&mut self, _now: Instant) -> GameAction {
        GameAction::None
    }

    pub fn status_line(&self) -> String {
        let base = if self.finished {
            self.status.clone()
        } else {
            format!(
                "Target {}/{} · cursor ({}, {}) · elapsed {:.1}s",
                self.hits + 1,
                TARGETS,
                self.cursor.0 + 1,
                self.cursor.1 + 1,
                (Instant::now() - self.run_start).as_secs_f64()
            )
        };
        if let Some(count) = self.nav.prefix() {
            format!("{} · count {}", base, count)
        } else {
            base
        }
    }
}
