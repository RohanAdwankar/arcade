use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode};
use rand::{SeedableRng, rngs::StdRng, seq::SliceRandom};
use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{GameAction, GameKind, StatRecord};

const GRID: usize = 4;
const BASE_NUMBERS: u8 = 4;
const REVEAL: Duration = Duration::from_secs(2);

#[derive(Debug)]
pub struct ChimpTestState {
    tiles: Vec<Tile>,
    cursor: (usize, usize),
    next_value: u8,
    level: u8,
    best: u8,
    rng: StdRng,
    phase: Phase,
    status: String,
    numbers_hidden: bool,
}

#[derive(Debug)]
struct Tile {
    value: u8,
    pos: (usize, usize),
    cleared: bool,
}

#[derive(Debug, Clone, Copy)]
enum Phase {
    Reveal { start: Instant },
    Input,
    Result,
}

impl ChimpTestState {
    pub fn new() -> Self {
        let rng = StdRng::from_entropy();
        let mut state = Self {
            tiles: Vec::new(),
            cursor: (0, 0),
            next_value: 1,
            level: 1,
            best: 0,
            rng,
            phase: Phase::Reveal {
                start: Instant::now(),
            },
            status: "Memorize the numbers".into(),
            numbers_hidden: false,
        };
        state.generate_tiles();
        state
    }

    fn generate_tiles(&mut self) {
        self.tiles.clear();
        self.next_value = 1;
        self.numbers_hidden = false;
        let count = BASE_NUMBERS + self.level;
        let mut positions = Vec::new();
        for x in 0..GRID {
            for y in 0..GRID {
                positions.push((x, y));
            }
        }
        positions.shuffle(&mut self.rng);
        for value in 1..=count {
            let pos = positions[value as usize - 1];
            self.tiles.push(Tile {
                value,
                pos,
                cleared: false,
            });
        }
        self.phase = Phase::Reveal {
            start: Instant::now(),
        };
        self.status = format!("Level {} · remember the order", self.level);
    }

    fn move_cursor(&mut self, dx: isize, dy: isize) {
        let (mut x, mut y) = self.cursor;
        x = ((x as isize + dx).clamp(0, (GRID - 1) as isize)) as usize;
        y = ((y as isize + dy).clamp(0, (GRID - 1) as isize)) as usize;
        self.cursor = (x, y);
    }

    fn select(&mut self) -> GameAction {
        if !matches!(self.phase, Phase::Input) {
            return GameAction::None;
        }
        if let Some(idx) = self.tiles.iter().position(|t| t.pos == self.cursor) {
            let value = self.tiles[idx].value;
            if value == self.next_value {
                self.tiles[idx].cleared = true;
                self.next_value += 1;
                if value == 1 {
                    self.numbers_hidden = true;
                }
                if self.tiles.iter().all(|t| t.cleared) {
                    self.level += 1;
                    if self.level > self.best {
                        self.best = self.level;
                        let record = StatRecord {
                            label: "Level".into(),
                            value: self.best.to_string(),
                        };
                        self.generate_tiles();
                        return GameAction::Record(record, GameKind::ChimpTest);
                    }
                    self.generate_tiles();
                }
            } else {
                self.status = format!("Missed! the next number was {}", self.next_value);
                self.phase = Phase::Result;
                self.level = 1;
            }
        }
        GameAction::None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Chimp Test")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines = vec![Line::from(format!(
            "Level {} (best {})",
            self.level, self.best
        ))];
        lines.push(Line::from(self.status.as_str()));

        for y in 0..GRID {
            let mut row = String::new();
            for x in 0..GRID {
                if Some((x, y)) == Some(self.cursor) {
                    row.push('[');
                }
                if let Some(tile) = self.tiles.iter().find(|t| t.pos == (x, y)) {
                    let numbers_visible = matches!(self.phase, Phase::Reveal { .. })
                        || (!self.numbers_hidden && !tile.cleared);
                    let show_number = numbers_visible;
                    if show_number {
                        row.push(char::from_digit(tile.value as u32, 10).unwrap_or('*'));
                    } else {
                        row.push('·');
                    }
                } else {
                    row.push('·');
                }
                if Some((x, y)) == Some(self.cursor) {
                    row.push(']');
                }
                row.push(' ');
            }
            lines.push(Line::from(row));
        }
        frame.render_widget(Paragraph::new(lines), inner);
    }

    pub fn handle_event(&mut self, event: &Event) -> GameAction {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Left | KeyCode::Char('h') => self.move_cursor(-1, 0),
                KeyCode::Right | KeyCode::Char('l') => self.move_cursor(1, 0),
                KeyCode::Up | KeyCode::Char('k') => self.move_cursor(0, -1),
                KeyCode::Down | KeyCode::Char('j') => self.move_cursor(0, 1),
                KeyCode::Enter => {
                    if matches!(self.phase, Phase::Result) {
                        self.level = 1;
                        self.generate_tiles();
                    } else {
                        return self.select();
                    }
                }
                KeyCode::Char(' ') => return self.select(),
                _ => {}
            }
        }
        GameAction::None
    }

    pub fn handle_tick(&mut self, now: Instant) -> GameAction {
        if let Phase::Reveal { start } = self.phase {
            if now.duration_since(start) >= REVEAL {
                self.phase = Phase::Input;
                self.status = "Select numbers in order".into();
            }
        }
        GameAction::None
    }

    pub fn status_line(&self) -> String {
        format!("Level {} · Next {}", self.level, self.next_value)
    }
}
