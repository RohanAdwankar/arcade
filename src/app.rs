use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent};
use dirs::config_dir;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::*;
use serde::Deserialize;

use crate::games::{GameAction, GameKind, GameState, StatRecord};
use crate::hud::{self, HudContext};
use crate::menu::MenuState;

const TICK_RATE: Duration = Duration::from_millis(50);
const HISTORY_LIMIT: usize = 64;

pub struct App {
    menu: MenuState,
    active: Option<GameState>,
    stats: HashMap<GameKind, Vec<StatRecord>>,
    should_quit: bool,
    toast: Option<Toast>,
    command: Option<CommandPalette>,
    stats_path: Option<PathBuf>,
    show_help: bool,
}

impl Default for App {
    fn default() -> Self {
        let (mut stats, stats_path) = load_persisted_stats();
        for history in stats.values_mut() {
            if history.len() > HISTORY_LIMIT {
                let overflow = history.len() - HISTORY_LIMIT;
                history.drain(0..overflow);
            }
        }
        Self {
            menu: MenuState::default(),
            active: None,
            stats,
            should_quit: false,
            toast: None,
            command: None,
            stats_path,
            show_help: false,
        }
    }
}

impl App {
    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<(), Box<dyn Error>> {
        let mut last_tick = Instant::now();
        while !self.should_quit {
            terminal.draw(|frame| self.render(frame))?;
            let timeout = TICK_RATE
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));
            if event::poll(timeout)? {
                let evt = event::read()?;
                self.handle_event(evt);
            }
            if last_tick.elapsed() >= TICK_RATE {
                self.on_tick();
                last_tick = Instant::now();
            }
        }
        Ok(())
    }

    fn handle_event(&mut self, event: Event) {
        match &event {
            Event::Key(key) => self.handle_key(*key),
            _ => {
                if let Some(active) = &mut self.active {
                    let action = active.handle_event(&event);
                    self.handle_game_action(action);
                }
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.handle_command_key(key) {
            return;
        }

        match key.code {
            KeyCode::Char(':') if key.modifiers.is_empty() => {
                self.command = Some(CommandPalette::default());
                return;
            }
            _ => {}
        }

        if let Some(active) = &mut self.active {
            let action = active.handle_event(&Event::Key(key));
            self.handle_game_action(action);
        } else {
            self.handle_menu_key(key);
        }
    }

    fn handle_command_key(&mut self, key: KeyEvent) -> bool {
        if let Some(command) = &mut self.command {
            match key.code {
                KeyCode::Esc => {
                    self.command = None;
                }
                KeyCode::Enter => {
                    let buffer = command.buffer.trim().to_string();
                    self.command = None;
                    self.execute_command(buffer);
                }
                KeyCode::Backspace => {
                    command.buffer.pop();
                }
                KeyCode::Char(ch) => {
                    if !ch.is_control() {
                        command.buffer.push(ch);
                    }
                }
                _ => {}
            }
            true
        } else {
            false
        }
    }

    fn execute_command(&mut self, buffer: String) {
        match buffer.as_str() {
            "qa" | "quitall" => self.should_quit = true,
            "q" | "quit" => {
                if self.active.is_some() {
                    self.active = None;
                    self.toast = Some(Toast::new("Returned to menu"));
                } else {
                    self.should_quit = true;
                }
            }
            "menu" => {
                self.active = None;
                self.toast = Some(Toast::new("Returned to menu"));
            }
            "restart" => {
                if let Some(kind) = self.active.as_ref().map(GameState::kind) {
                    self.active = Some(GameState::new(kind));
                    self.toast = Some(Toast::new(format!("Restarted {}", kind.title())));
                }
            }
            "help" => {
                self.show_help = !self.show_help;
                let state = if self.show_help { "shown" } else { "hidden" };
                self.toast = Some(Toast::new(format!("Controls {state}")));
            }
            other if other.is_empty() => {}
            other => {
                self.toast = Some(Toast::new(format!("Unknown command :{other}")));
            }
        }
    }

    fn handle_menu_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.menu.next(),
            KeyCode::Up | KeyCode::Char('k') => self.menu.previous(),
            KeyCode::Enter | KeyCode::Char('l') => self.launch_selected_game(),
            KeyCode::Char('h') => self.toast = Some(Toast::new("Use enter to launch a game")),
            _ => {}
        }
    }

    fn launch_selected_game(&mut self) {
        let kind = self.menu.selected_kind();
        self.active = Some(GameState::new(kind));
        self.toast = Some(Toast::new(format!("Starting {}", kind.title())));
    }

    fn on_tick(&mut self) {
        if let Some(toast) = &self.toast {
            if toast.is_expired() {
                self.toast = None;
            }
        }

        if let Some(active) = &mut self.active {
            let action = active.handle_tick(Instant::now());
            self.handle_game_action(action);
        }
    }

    fn handle_game_action(&mut self, action: GameAction) {
        match action {
            GameAction::None => {}
            GameAction::Record(record, kind) => {
                let history = self.stats.entry(kind).or_default();
                history.push(record);
                if history.len() > HISTORY_LIMIT {
                    let overflow = history.len() - HISTORY_LIMIT;
                    history.drain(0..overflow);
                }
                self.persist_stats();
            }
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(4)])
            .split(frame.size());

        if let Some(active) = &self.active {
            active.render(frame, areas[0]);
        } else {
            self.menu.render(frame, areas[0], &self.stats);
        }

        let status_line = if self.show_help {
            Some(if let Some(active) = &self.active {
                active.status_line()
            } else {
                self.menu.status_line()
            })
        } else {
            None
        };
        let help_line = if self.show_help {
            Some(if self.active.is_some() {
                "hjkl/arrow keys to move · space/enter to act · :q menu · :qa quit · :help hide"
                    .to_string()
            } else {
                "j/k to move · enter to play · :q quit · :help to show commands".to_string()
            })
        } else {
            None
        };
        let command_text = self.command.as_ref().map(|cmd| format!(":{}", cmd.buffer));
        let toast_text = self.toast.as_ref().map(|t| t.message.as_str());
        hud::render(
            frame,
            areas[1],
            HudContext {
                primary: status_line.as_deref().unwrap_or(""),
                secondary: help_line.as_deref().unwrap_or(""),
                command: command_text.as_deref(),
                toast: toast_text,
            },
        );
    }
}

#[derive(Debug, Clone)]
struct Toast {
    message: String,
    expires_at: Instant,
}

impl Toast {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            expires_at: Instant::now() + Duration::from_secs(3),
        }
    }

    fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
}

fn load_persisted_stats() -> (HashMap<GameKind, Vec<StatRecord>>, Option<PathBuf>) {
    let path = stats_file_path();
    if let Some(path_ref) = &path {
        if let Ok(bytes) = fs::read(path_ref) {
            if let Ok(map) = serde_json::from_slice::<HashMap<GameKind, Vec<StatRecord>>>(&bytes) {
                return (map, path);
            }
            if let Ok(legacy) =
                serde_json::from_slice::<HashMap<GameKind, LegacyStatRecord>>(&bytes)
            {
                let converted = legacy
                    .into_iter()
                    .map(|(kind, record)| {
                        let score = parse_legacy_score(kind, &record.value);
                        let converted = StatRecord {
                            label: record.label,
                            value: record.value,
                            score,
                            recorded_at: 0,
                        };
                        (kind, vec![converted])
                    })
                    .collect();
                return (converted, path);
            }
        }
    }
    (HashMap::new(), path)
}

fn stats_file_path() -> Option<PathBuf> {
    let mut dir = config_dir()?;
    dir.push("bored");
    dir.push("scores.json");
    Some(dir)
}

impl App {
    fn persist_stats(&self) {
        if let Some(path) = &self.stats_path {
            if let Some(parent) = path.parent() {
                if fs::create_dir_all(parent).is_err() {
                    return;
                }
            }
            if let Ok(json) = serde_json::to_vec_pretty(&self.stats) {
                let _ = fs::write(path, json);
            }
        }
    }
}

#[derive(Deserialize)]
struct LegacyStatRecord {
    label: String,
    value: String,
}

fn parse_legacy_score(_kind: GameKind, value: &str) -> f64 {
    value
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect::<String>()
        .parse::<f64>()
        .unwrap_or(0.0)
}

#[derive(Default)]
struct CommandPalette {
    buffer: String,
}
