use std::cmp::Ordering;
use std::collections::HashMap;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::games::{GameKind, ScoreDirection, StatRecord};

const MAX_CHART_POINTS: usize = 32;
const SPARKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

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

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        stats: &HashMap<GameKind, Vec<StatRecord>>,
    ) {
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
                if let Some(history) = stats.get(kind) {
                    if let Some(best) = best_record(*kind, history) {
                        line.push_str(&format!("  · {}: {}", best.label, best.value));
                    }
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
        let mut detail_lines = vec![
            Line::from(selected_kind.title()),
            Line::from(""),
            Line::from(selected_kind.blurb()),
            Line::from(""),
            Line::from("Personal Best"),
        ];
        let pb = stats
            .get(&selected_kind)
            .and_then(|history| best_record(selected_kind, history))
            .map(|record| format!("{}: {}", record.label, record.value))
            .unwrap_or_else(|| "No score yet".to_string());
        detail_lines.push(Line::from(pb));

        if let Some(history) = stats.get(&selected_kind) {
            if let Some((chart_line, min_score, max_score)) =
                build_chart_line(history, selected_kind.score_direction())
            {
                detail_lines.push(Line::from(""));
                detail_lines.push(Line::from("Score Progress"));
                detail_lines.push(chart_line);
                detail_lines.push(Line::from(format!(
                    "Range {} – {} · samples {}",
                    format_score(min_score),
                    format_score(max_score),
                    history.len()
                )));
            } else {
                detail_lines.push(Line::from(""));
                detail_lines.push(Line::from("Play a run to record your first score."));
            }
        } else {
            detail_lines.push(Line::from(""));
            detail_lines.push(Line::from("No attempts logged yet."));
        }

        let detail = Paragraph::new(detail_lines)
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

fn best_record<'a>(kind: GameKind, history: &'a [StatRecord]) -> Option<&'a StatRecord> {
    match kind.score_direction() {
        ScoreDirection::HigherIsBetter => history
            .iter()
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(Ordering::Equal)),
        ScoreDirection::LowerIsBetter => history
            .iter()
            .min_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(Ordering::Equal)),
    }
}

fn build_chart_line(
    history: &[StatRecord],
    direction: ScoreDirection,
) -> Option<(Line<'static>, f64, f64)> {
    if history.is_empty() {
        return None;
    }
    let len = history.len().min(MAX_CHART_POINTS);
    let slice = &history[history.len() - len..];
    let min_score = slice
        .iter()
        .fold(f64::INFINITY, |acc, record| acc.min(record.score));
    let max_score = slice
        .iter()
        .fold(f64::NEG_INFINITY, |acc, record| acc.max(record.score));
    let best_idx = match direction {
        ScoreDirection::HigherIsBetter => slice
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.score.partial_cmp(&b.score).unwrap_or(Ordering::Equal))
            .map(|(idx, _)| idx)
            .unwrap_or(0),
        ScoreDirection::LowerIsBetter => slice
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.score.partial_cmp(&b.score).unwrap_or(Ordering::Equal))
            .map(|(idx, _)| idx)
            .unwrap_or(0),
    };
    let range = (max_score - min_score).abs();
    let spans: Vec<Span> = slice
        .iter()
        .enumerate()
        .map(|(idx, record)| {
            let normalized = if range < f64::EPSILON {
                0.5
            } else {
                ((record.score - min_score) / (max_score - min_score)).clamp(0.0, 1.0)
            };
            let bucket = (normalized * (SPARKS.len() - 1) as f64).round() as usize;
            let bucket = bucket.min(SPARKS.len() - 1);
            let ch = SPARKS[bucket];
            let style = if idx == best_idx {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            Span::styled(ch.to_string(), style)
        })
        .collect();
    Some((Line::from(spans), min_score, max_score))
}

fn format_score(value: f64) -> String {
    if (value.fract()).abs() < 0.05 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}
