use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode};
use rand::{Rng, SeedableRng, rngs::StdRng};
use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{GameAction, GameKind, StatRecord};

const GRID: usize = 5;
const TARGETS: u32 = 10;

#[derive(Debug)]
pub struct AimTrainerState {
    cursor: (usize, usize),
    target: (usize, usize),
    hits: u32,
    total_time: Duration,
    spawn: Instant,
    rng: StdRng,
    finished: bool,
    best_avg: Option<f64>,
    status: String,
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
            rng,
            finished: false,
            best_avg: None,
            status: "Move with hjkl · enter to tag".into(),
        }
    }

    fn move_cursor(&mut self, dx: isize, dy: isize) {
        let (mut x, mut y) = self.cursor;
        x = ((x as isize + dx).clamp(0, (GRID - 1) as isize)) as usize;
        y = ((y as isize + dy).clamp(0, (GRID - 1) as isize)) as usize;
        self.cursor = (x, y);
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
                let avg_ms = self.total_time.as_secs_f64() * 1000.0 / TARGETS as f64;
                self.status = format!("Complete! avg {:.0} ms", avg_ms);
                if self.best_avg.map(|best| avg_ms < best).unwrap_or(true) {
                    self.best_avg = Some(avg_ms);
                    return GameAction::Record(
                        StatRecord {
                            label: "Avg".into(),
                            value: format!("{avg_ms:.0} ms"),
                        },
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

        let mut lines = vec![Line::from(format!("Hits: {}/{}", self.hits, TARGETS))];
        lines.push(Line::from(self.status.as_str()));

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
            match key.code {
                KeyCode::Left | KeyCode::Char('h') => self.move_cursor(-1, 0),
                KeyCode::Right | KeyCode::Char('l') => self.move_cursor(1, 0),
                KeyCode::Up | KeyCode::Char('k') => self.move_cursor(0, -1),
                KeyCode::Down | KeyCode::Char('j') => self.move_cursor(0, 1),
                KeyCode::Enter | KeyCode::Char(' ') => return self.tag(),
                _ => {}
            }
        }
        GameAction::None
    }

    pub fn handle_tick(&mut self, _now: Instant) -> GameAction {
        GameAction::None
    }

    pub fn status_line(&self) -> String {
        if self.finished {
            self.status.clone()
        } else {
            format!(
                "Target {}/{} · cursor ({}, {})",
                self.hits + 1,
                TARGETS,
                self.cursor.0 + 1,
                self.cursor.1 + 1
            )
        }
    }
}
