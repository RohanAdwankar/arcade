use std::collections::HashMap;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::games::{GameKind, StatRecord};

#[derive(Debug)]
pub struct MenuState {
    items: Vec<GameKind>,
    selected: usize,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            items: GameKind::ALL.to_vec(),
            selected: 0,
        }
    }
}

impl MenuState {
    pub fn selected_kind(&self) -> GameKind {
        self.items[self.selected]
    }

    pub fn next(&mut self) {
        self.selected = (self.selected + 1) % self.items.len();
    }

    pub fn previous(&mut self) {
        if self.selected == 0 {
            self.selected = self.items.len() - 1;
        } else {
            self.selected -= 1;
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, stats: &HashMap<GameKind, StatRecord>) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(area);

        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(idx, kind)| {
                let mut line = format!("{}", kind.title());
                if let Some(record) = stats.get(kind) {
                    line.push_str(&format!("  · {}: {}", record.label, record.value));
                }
                let style = if idx == self.selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Span::styled(line, style))
            })
            .collect();
        let list = List::new(items)
            .block(
                Block::default()
                    .title("Memory Arcade")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Gray)),
            )
            .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White));
        frame.render_widget(list, chunks[0]);

        let selected_kind = self.selected_kind();
        let description = format!("{}\n\n{}", selected_kind.title(), selected_kind.blurb());
        let pb = stats
            .get(&selected_kind)
            .map(|record| format!("{}: {}", record.label, record.value))
            .unwrap_or_else(|| "No score yet".to_string());
        let detail = Paragraph::new(format!("{}\n\nPersonal Best\n{}", description, pb))
            .block(
                Block::default()
                    .title("Details")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Gray)),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(detail, chunks[1]);
    }

    pub fn status_line(&self) -> String {
        format!(
            "Menu · j/k or ↑/↓ to move · enter to launch {}",
            self.selected_kind().title()
        )
    }
}
