use std::f32::consts::TAU;
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode};
use once_cell::sync::Lazy;
use rand::{Rng, SeedableRng, rngs::StdRng};
use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{GameAction, GameKind, StatRecord};

const FIELD_ROWS: usize = 18;
const BASE_FIELD_COLS: usize = 60;
const HIT_WINDOW: f32 = 1.5;
const CLOSE_THRESHOLD: f32 = 0.75;
const BASE_SPAWN_MS: u64 = 1400;
const MIN_SPAWN_MS: u64 = 350;
const ENEMY_BASE_SPEED: f32 = 1.4;
const ENEMY_SPEED_STEP: f32 = 0.08;
const MAX_DEPTH: f32 = 20.0;
const TURN_STEP: f32 = TAU / 48.0;
const FIRE_FLASH_MS: u64 = 120;
const MAG_CAPACITY: u32 = 5;
const RELOAD_MS: u64 = 800;

static PISTOL_ART: Lazy<Vec<String>> = Lazy::new(|| {
    include_str!("../../assets/pistol.txt")
        .lines()
        .map(|line| line.to_string())
        .collect()
});

const WEAPON_NAME: &str = "Pistol";

fn weapon_art() -> &'static [String] {
    &PISTOL_ART
}

fn weapon_max_width() -> usize {
    weapon_art()
        .iter()
        .map(|line| line.len())
        .max()
        .unwrap_or(BASE_FIELD_COLS)
        .max(BASE_FIELD_COLS)
}

fn weapon_lines(field_cols: usize) -> Vec<Line<'static>> {
    let cols = field_cols.max(1);
    weapon_art()
        .iter()
        .map(|line| {
            let mut owned = line.clone();
            if owned.len() < cols {
                owned.push_str(&" ".repeat(cols - owned.len()));
            }
            Line::from(owned)
        })
        .collect()
}

const FAR_SPRITE: [&str; 1] = [r"  ·  "];
const MID_SPRITE: [&str; 3] = [r"  /\  ", r" (**) ", r" /  \ "];
const CLOSE_SPRITE: [&str; 5] = [
    r"  /MM\  ",
    r" [====] ",
    r"  |  |  ",
    r" /|  |\ ",
    r"/_/  \_\",
];

fn sprite_for_depth(depth: f32) -> &'static [&'static str] {
    if depth < 3.0 {
        &CLOSE_SPRITE
    } else if depth < 7.0 {
        &MID_SPRITE
    } else {
        &FAR_SPRITE
    }
}

fn draw_sprite(rows: &mut [Vec<char>], sprite: &[&str], base_row: isize, center_col: isize) {
    let height = sprite.len() as isize;
    for (row_offset, line) in sprite.iter().enumerate() {
        let row = base_row - (height - 1 - row_offset as isize);
        if row < 0 || row >= rows.len() as isize {
            continue;
        }
        let row_idx = row as usize;
        let line_chars: Vec<char> = line.chars().collect();
        let width = line_chars.len() as isize;
        let start_col = center_col - width / 2;
        for (col_offset, ch) in line_chars.into_iter().enumerate() {
            if ch == ' ' {
                continue;
            }
            let col = start_col + col_offset as isize;
            if col < 0 || col >= rows[row_idx].len() as isize {
                continue;
            }
            rows[row_idx][col as usize] = ch;
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Vec2 {
    x: f32,
    y: f32,
}

impl Vec2 {
    fn distance(self, other: Vec2) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    fn rotate(self, angle: f32) -> Vec2 {
        let cos = angle.cos();
        let sin = angle.sin();
        Vec2 {
            x: self.x * cos - self.y * sin,
            y: self.x * sin + self.y * cos,
        }
    }
}

#[derive(Debug, Clone)]
struct Enemy {
    pos: Vec2,
    speed: f32,
}

impl Enemy {
    fn update(&mut self, target: Vec2, dt: f32) {
        let dy = target.y - self.pos.y;
        let dx = target.x - self.pos.x;
        let distance = (dx * dx + dy * dy).sqrt().max(0.0001);
        let ny = dy / distance;
        self.pos.y += ny * self.speed * dt;
        self.pos.x = target.x;
    }

    fn relative_to(&self, player: Vec2) -> Vec2 {
        Vec2 {
            x: self.pos.x - player.x,
            y: self.pos.y - player.y,
        }
    }
}

#[derive(Debug)]
pub struct ShooterState {
    rng: StdRng,
    enemies: Vec<Enemy>,
    player: Vec2,
    alive: bool,
    kills: u32,
    start_time: Instant,
    last_tick: Instant,
    last_spawn: Instant,
    status: String,
    best_survival: Option<Duration>,
    heading: f32,
    last_fire: Option<Instant>,
    frozen_elapsed: Option<Duration>,
    shots_remaining: u32,
    reloading_until: Option<Instant>,
}

impl ShooterState {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            rng: StdRng::from_entropy(),
            enemies: Vec::new(),
            player: Vec2::default(),
            alive: true,
            kills: 0,
            start_time: now,
            last_tick: now,
            last_spawn: now,
            status: "Rotate with A/D · h/l · ←/→ · space fires".into(),
            best_survival: None,
            heading: 0.0,
            last_fire: None,
            frozen_elapsed: None,
            shots_remaining: MAG_CAPACITY,
            reloading_until: None,
        }
    }

    fn elapsed(&self, now: Instant) -> Duration {
        if let Some(frozen) = self.frozen_elapsed {
            frozen
        } else {
            now.saturating_duration_since(self.start_time)
        }
    }

    fn spawn_interval(&self) -> Duration {
        let factor = (0.92_f64).powf(self.kills as f64 / 4.0);
        let ms = (BASE_SPAWN_MS as f64 * factor).max(MIN_SPAWN_MS as f64);
        Duration::from_millis(ms as u64)
    }

    fn spawn_enemy(&mut self) {
        let angle = self.rng.gen_range(0.0..TAU);
        let distance = self.rng.gen_range(8.0..16.0);
        let speed = ENEMY_BASE_SPEED + ENEMY_SPEED_STEP * self.kills as f32;
        self.enemies.push(Enemy {
            pos: Vec2 {
                x: self.player.x + angle.cos() * distance,
                y: self.player.y + angle.sin() * distance,
            },
            speed,
        });
    }

    fn maybe_spawn(&mut self, now: Instant) {
        let interval = self.spawn_interval();
        while now.duration_since(self.last_spawn) >= interval {
            self.last_spawn += interval;
            self.spawn_enemy();
        }
    }

    fn rotate_camera(&mut self, delta: f32) {
        if !self.alive {
            return;
        }
        self.heading = (self.heading + delta) % TAU;
        if self.heading < 0.0 {
            self.heading += TAU;
        }
        self.status = format!("Heading {:.0}°", self.heading_degrees());
    }

    fn fire(&mut self) {
        if !self.alive {
            return;
        }
        if let Some(ready_at) = self.reloading_until {
            if Instant::now() < ready_at {
                self.status = "Reloading...".into();
                return;
            }
            self.reloading_until = None;
        }
        if self.shots_remaining == 0 {
            let until = Instant::now() + Duration::from_millis(RELOAD_MS);
            self.reloading_until = Some(until);
            self.status = "Reloading...".into();
            return;
        }
        self.last_fire = Some(Instant::now());
        self.shots_remaining -= 1;
        let mut hit_index: Option<usize> = None;
        let mut best_distance = f32::MAX;
        for (idx, enemy) in self.enemies.iter().enumerate() {
            let rel = enemy.relative_to(self.player);
            let cam = rel.rotate(-self.heading);
            if cam.y <= 0.1 {
                continue;
            }
            if cam.x.abs() <= HIT_WINDOW {
                if cam.y < best_distance {
                    best_distance = cam.y;
                    hit_index = Some(idx);
                }
            }
        }
        if let Some(idx) = hit_index {
            self.enemies.swap_remove(idx);
            self.kills += 1;
            self.status = "Direct hit!".into();
        } else {
            self.status = "Missed — keep them centered".into();
        }
    }

    fn handle_failure(&mut self, now: Instant) -> GameAction {
        if !self.alive {
            return GameAction::None;
        }
        self.alive = false;
        let elapsed = self.elapsed(now);
        self.frozen_elapsed = Some(elapsed);
        self.status = format!("Overrun after {:.1}s", elapsed.as_secs_f64());
        if self
            .best_survival
            .map(|best| elapsed > best)
            .unwrap_or(true)
        {
            self.best_survival = Some(elapsed);
            return GameAction::Record(
                StatRecord::new(
                    "Survival",
                    format!("{:.1}s", elapsed.as_secs_f64()),
                    elapsed.as_secs_f64(),
                ),
                GameKind::Shooter,
            );
        }
        GameAction::None
    }

    fn restart(&mut self) {
        let now = Instant::now();
        self.enemies.clear();
        self.player = Vec2::default();
        self.alive = true;
        self.kills = 0;
        self.start_time = now;
        self.last_tick = now;
        self.last_spawn = now;
        self.status = "Rotate with A/D · h/l · ←/→ · space fires".into();
        self.heading = 0.0;
        self.last_fire = None;
        self.frozen_elapsed = None;
        self.shots_remaining = MAG_CAPACITY;
        self.reloading_until = None;
    }

    fn render_field(&self, cols: usize) -> Vec<Line<'static>> {
        let mut rows = vec![vec![' '; cols]; FIELD_ROWS];
        let horizon = FIELD_ROWS / 3;
        let center = cols / 2;

        for enemy in &self.enemies {
            let rel = enemy.relative_to(self.player);
            let cam = rel.rotate(-self.heading);
            if cam.y <= 0.1 {
                continue;
            }
            let depth = rel.distance(Vec2 { x: 0.0, y: 0.0 });
            if depth > MAX_DEPTH {
                continue;
            }
            let depth_norm = (depth / MAX_DEPTH).clamp(0.0, 1.0);
            let sprite = sprite_for_depth(depth);
            let width_scale = cols as f32 / BASE_FIELD_COLS as f32;
            let horizontal_scale = width_scale * (1.2 + (1.0 - depth_norm) * 3.6);
            let col = (center as f32 + cam.x * horizontal_scale)
                .round()
                .clamp(1.0, (cols.saturating_sub(2)) as f32) as isize;
            let row = (horizon as f32 + (1.0 - depth_norm) * ((FIELD_ROWS - horizon - 1) as f32))
                .round()
                .clamp((horizon + 1) as f32, (FIELD_ROWS - 2) as f32)
                as isize;
            draw_sprite(&mut rows, sprite, row, col);
        }

        rows.into_iter()
            .map(|chars| Line::from(chars.into_iter().collect::<String>()))
            .collect()
    }
}

impl ShooterState {
    fn heading_degrees(&self) -> f32 {
        let mut deg = self.heading.to_degrees() % 360.0;
        if deg < 0.0 {
            deg += 360.0;
        }
        deg
    }

    fn view_width(&self) -> usize {
        weapon_max_width()
    }

    fn tick_internal(&mut self, now: Instant) -> Option<GameAction> {
        let dt = now.saturating_duration_since(self.last_tick).as_secs_f32();
        self.last_tick = now;
        if self.alive {
            if let Some(until) = self.reloading_until {
                if now >= until {
                    self.reloading_until = None;
                    self.shots_remaining = MAG_CAPACITY;
                    self.status = "Magazine refilled".into();
                }
            }
            self.maybe_spawn(now);
            for enemy in &mut self.enemies {
                enemy.update(self.player, dt);
            }
            for enemy in &self.enemies {
                if enemy.pos.distance(self.player) < CLOSE_THRESHOLD {
                    return Some(self.handle_failure(now));
                }
            }
        }
        self.enemies
            .retain(|enemy| enemy.pos.distance(self.player) >= CLOSE_THRESHOLD || !self.alive);
        None
    }
}

impl ShooterState {
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Terminal FPS")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let field_cols = self.view_width();
        let now = Instant::now();
        let elapsed = self.elapsed(now).as_secs_f64();
        let heading = self.heading_degrees();
        let reload_status = if let Some(until) = self.reloading_until {
            if Instant::now() < until {
                format!(
                    " · Reloading {:.0}%",
                    (100.0
                        * (1.0
                            - until
                                .saturating_duration_since(Instant::now())
                                .as_secs_f64()
                                / (RELOAD_MS as f64 / 1000.0)))
                        .clamp(0.0, 99.0)
                )
            } else {
                "".to_string()
            }
        } else {
            format!(" · {}/{}", self.shots_remaining, MAG_CAPACITY)
        };
        let best_suffix = self
            .best_survival
            .map(|best| format!(" · Best {:.1}s", best.as_secs_f64()))
            .unwrap_or_default();
        let mut lines = vec![Line::from(format!(
            "Time {:.1}s · Kills {} · Enemies {} · Weapon {} · Heading {:>5.1}°{} · {}{}",
            elapsed,
            self.kills,
            self.enemies.len(),
            WEAPON_NAME,
            heading,
            reload_status,
            self.status,
            best_suffix
        ))];

        lines.push(Line::from("-".repeat(field_cols.max(1))));
        lines.extend(self.render_field(field_cols));
        lines.extend(self.crosshair_lines(field_cols));
        lines.extend(weapon_lines(field_cols));

        frame.render_widget(Paragraph::new(lines), inner);
    }

    pub fn handle_event(&mut self, event: &Event) -> GameAction {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('h') | KeyCode::Left => self.rotate_camera(-TURN_STEP),
                KeyCode::Char('l') | KeyCode::Right => self.rotate_camera(TURN_STEP),
                KeyCode::Char(' ') | KeyCode::Enter => {
                    if self.alive {
                        self.fire();
                    } else {
                        self.restart();
                    }
                }
                KeyCode::Char('a') | KeyCode::Char('A') => self.rotate_camera(-TURN_STEP),
                KeyCode::Char('d') | KeyCode::Char('D') => self.rotate_camera(TURN_STEP),
                KeyCode::Char('r') if !self.alive => self.restart(),
                _ => {}
            }
        }
        GameAction::None
    }

    fn firing_flash(&self, now: Instant) -> bool {
        if let Some(last) = self.last_fire {
            return now.duration_since(last).as_millis() < FIRE_FLASH_MS as u128;
        }
        false
    }

    fn crosshair_lines(&self, field_cols: usize) -> Vec<Line<'static>> {
        let cols = field_cols.max(1);
        let center = cols / 2;
        let flash = self.firing_flash(Instant::now());
        let mut upper = vec![' '; cols];
        let mut lower = vec![' '; cols];
        if center < cols {
            upper[center] = if flash { '·' } else { '|' };
            lower[center] = if flash { '•' } else { '+' };
        }
        let style = if flash {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::White)
        };
        vec![
            Line::styled(upper.into_iter().collect::<String>(), style),
            Line::styled(lower.into_iter().collect::<String>(), style),
        ]
    }

    pub fn handle_tick(&mut self, now: Instant) -> GameAction {
        if let Some(action) = self.tick_internal(now) {
            return action;
        }
        GameAction::None
    }

    pub fn status_line(&self) -> String {
        let now = Instant::now();
        let elapsed = self.elapsed(now).as_secs_f64();
        format!(
            "FPS · {:.1}s alive · kills {} · {} · {:>4.0}°",
            elapsed,
            self.kills,
            if self.alive { "Alive" } else { "Down" },
            self.heading_degrees()
        )
    }
}
