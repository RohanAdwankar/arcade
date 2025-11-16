use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode};
use rand::{Rng, SeedableRng, rngs::StdRng};
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{GameAction, GameKind, StatRecord};

const GRID: usize = 3;
const FLASH_ON: Duration = Duration::from_millis(450);
const FLASH_OFF: Duration = Duration::from_millis(180);

#[derive(Debug)]
pub struct SequenceState {
    sequence: Vec<(usize, usize)>,
    cursor: (usize, usize),
    idx: usize,
    best: usize,
    pending_best: Option<usize>,
    rng: StdRng,
    phase: Phase,
    status: String,
}

#[derive(Debug, Clone, Copy)]
enum Phase {
    Showing {
        step: usize,
        visible: bool,
        since: Instant,
    },
    Input,
}

impl SequenceState {
    pub fn new() -> Self {
        let mut rng = StdRng::from_entropy();
        let seq = vec![random_cell(&mut rng)];
        Self {
            sequence: seq,
            cursor: (0, 0),
            idx: 0,
            best: 0,
            pending_best: None,
            rng,
            phase: Phase::Showing {
                step: 0,
                visible: true,
                since: Instant::now(),
            },
            status: "Watch the pattern".into(),
        }
    }

    fn start_show(&mut self) {
        self.idx = 0;
        self.phase = Phase::Showing {
            step: 0,
            visible: true,
            since: Instant::now(),
        };
        self.status = format!("Watch the pattern ({} tiles)", self.sequence.len());
    }

    fn begin_new_round(&mut self, advance: bool) -> GameAction {
        if advance {
            self.sequence.push(random_cell(&mut self.rng));
        } else if self.sequence.is_empty() {
            self.sequence.push(random_cell(&mut self.rng));
        }
        self.start_show();
        GameAction::None
    }

    fn handle_selection(&mut self) -> GameAction {
        if !matches!(self.phase, Phase::Input) {
            return GameAction::None;
        }
        if let Some(expected) = self.sequence.get(self.idx) {
            if *expected == self.cursor {
                self.idx += 1;
                if self.idx == self.sequence.len() {
                    let completed = self.sequence.len();
                    if completed > self.best {
                        self.best = completed;
                        self.pending_best = Some(completed);
                    }
                    self.begin_new_round(true);
                } else {
                    self.status = format!("{} / {}", self.idx, self.sequence.len());
                }
            } else {
                if self.sequence.len().saturating_sub(1) > self.best {
                    self.best = self.sequence.len() - 1;
                    self.pending_best = Some(self.best);
                }
                let record = self.flush_pending_record();
                self.status = "Wrong square! Starting over".into();
                self.sequence.clear();
                self.idx = 0;
                self.begin_new_round(false);
                if !matches!(record, GameAction::None) {
                    return record;
                }
            }
        }
        GameAction::None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Sequence Memory")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines = vec![Line::from(format!(
            "Sequence length {} · Best {}",
            self.sequence.len(),
            self.best
        ))];
        lines.push(Line::from(self.status.as_str()));
        let flash_cell = match self.phase {
            Phase::Showing { step, visible, .. } if visible => self.sequence.get(step).copied(),
            _ => None,
        };
        for y in 0..GRID {
            let mut spans = Vec::with_capacity(GRID * 2);
            for x in 0..GRID {
                let mut style = Style::default();
                if Some((x, y)) == flash_cell {
                    style = style
                        .bg(Color::Yellow)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD);
                } else if matches!(self.phase, Phase::Input) && (x, y) == self.cursor {
                    style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
                }
                spans.push(Span::styled("■", style));
                spans.push(Span::raw(" "));
            }
            lines.push(Line::from(spans));
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
                KeyCode::Enter | KeyCode::Char(' ') => return self.handle_selection(),
                _ => {}
            }
        }
        GameAction::None
    }

    fn move_cursor(&mut self, dx: isize, dy: isize) {
        let (mut x, mut y) = self.cursor;
        x = ((x as isize + dx).clamp(0, (GRID - 1) as isize)) as usize;
        y = ((y as isize + dy).clamp(0, (GRID - 1) as isize)) as usize;
        self.cursor = (x, y);
    }

    pub fn handle_tick(&mut self, now: Instant) -> GameAction {
        if let Phase::Showing {
            step,
            visible,
            since,
        } = &mut self.phase
        {
            if *visible && now.duration_since(*since) >= FLASH_ON {
                *visible = false;
                *since = now;
            } else if !*visible && now.duration_since(*since) >= FLASH_OFF {
                *since = now;
                if *step + 1 >= self.sequence.len() {
                    self.phase = Phase::Input;
                    self.status = "Repeat the pattern".into();
                } else {
                    *step += 1;
                    *visible = true;
                }
            }
        }
        GameAction::None
    }

    pub fn status_line(&self) -> String {
        match self.phase {
            Phase::Input => format!("Repeat {}/{}", self.idx + 1, self.sequence.len()),
            Phase::Showing { .. } => format!("Showing pattern ({} tiles)", self.sequence.len()),
        }
    }

    fn flush_pending_record(&mut self) -> GameAction {
        if let Some(score) = self.pending_best.take() {
            GameAction::Record(
                StatRecord::new("Pattern", score.to_string(), score as f64),
                GameKind::Sequence,
            )
        } else {
            GameAction::None
        }
    }
}

fn random_cell(rng: &mut StdRng) -> (usize, usize) {
    (rng.gen_range(0..GRID), rng.gen_range(0..GRID))
}
