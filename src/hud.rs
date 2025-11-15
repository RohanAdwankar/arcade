use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub struct HudContext<'a> {
    pub primary: &'a str,
    pub secondary: &'a str,
    pub command: Option<&'a str>,
    pub toast: Option<&'a str>,
}

pub fn render(frame: &mut Frame, area: Rect, ctx: HudContext<'_>) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::DarkGray));
    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);
    let mut text = vec![Line::from(ctx.primary), Line::from(ctx.secondary)];

    if let Some(command) = ctx.command {
        text.push(Line::from(Span::styled(
            command,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
    }

    if let Some(toast) = ctx.toast {
        text.push(Line::from(Span::styled(
            toast,
            Style::default().fg(Color::LightGreen),
        )));
    }

    let paragraph = Paragraph::new(text).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, inner);
}
