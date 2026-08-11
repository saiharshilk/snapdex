use crate::naming::RenamePlan;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use std::io;
use std::time::Duration;

pub fn confirm(plans: &[RenamePlan]) -> io::Result<bool> {
    let mut terminal = ratatui::init();
    let result = run_preview(&mut terminal, plans);
    ratatui::restore();
    result
}

fn run_preview(terminal: &mut Terminal<impl Backend>, plans: &[RenamePlan]) -> io::Result<bool> {
    let mut selected = 0;
    let mut offset = 0;

    loop {
        terminal
            .draw(|frame| draw(frame, plans, selected, offset))
            .map_err(|error| io::Error::other(error.to_string()))?;
        if !event::poll(Duration::from_millis(200))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(false),
            KeyCode::Char('y') | KeyCode::Enter => return Ok(true),
            KeyCode::Down | KeyCode::Char('j') => {
                selected = (selected + 1).min(plans.len().saturating_sub(1));
                offset = offset.max(selected.saturating_sub(8));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                selected = selected.saturating_sub(1);
                offset = offset.min(selected);
            }
            _ => {}
        }
    }
}

fn draw(frame: &mut Frame<'_>, plans: &[RenamePlan], selected: usize, offset: usize) {
    let area = frame.area();
    let layout = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(2),
        Constraint::Length(2),
    ])
    .split(area);
    let header = Paragraph::new(format!("{} image(s) ready to rename", plans.len()))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" snapdex preview "),
        )
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(header, layout[0]);

    let visible = plans.iter().skip(offset).enumerate().map(|(index, plan)| {
        let old = plan
            .old_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let new = plan
            .new_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let marker = if plan.fallback { "[fallback] " } else { "" };
        let style = if offset + index == selected {
            Style::default().fg(Color::Yellow)
        } else if plan.fallback {
            Style::default().fg(Color::Magenta)
        } else {
            Style::default()
        };
        ListItem::new(format!("{marker}{old}  →  {new}")).style(style)
    });
    let list = List::new(visible)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" pending renames "),
        )
        .highlight_style(Style::default().fg(Color::Yellow));
    let mut state = ListState::default();
    state.select(Some(selected.saturating_sub(offset)));
    frame.render_stateful_widget(list, layout[1], &mut state);

    let footer = Paragraph::new("↑/↓ or j/k: scroll   y/Enter: rename   q/Esc: cancel")
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(footer, layout[2]);
}
