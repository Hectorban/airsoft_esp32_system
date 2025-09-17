use ratatui::{
    Frame,
    layout::{Rect, Layout, Direction, Constraint},
    style::{Style, Color},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem},
};
use alloc::format;
use alloc::vec::Vec;
use alloc::vec;
use alloc::string::ToString;

use crate::{
    events::{InputEvent, TaskSenders},
    views::NavigationAction,
};
use super::SearchAndDestroyView;

pub(super) fn handle_input(view: &mut SearchAndDestroyView, event: InputEvent, task_senders: &TaskSenders) -> Option<NavigationAction> {
    match event {
        InputEvent::KeypadEvent(key) => {
            match key {
                '3' => { // Plant bomb
                    view.plant_bomb(task_senders);
                    None
                },
                '9' => { // Terrorists win round
                    view.end_round(true, task_senders);
                    None
                },
                '6' => { // Counter-terrorists win round
                    view.end_round(false, task_senders);
                    None
                },
                '0' => { // Back to main menu
                    Some(NavigationAction::Back)
                },
                _ => None,
            }
        },
        InputEvent::CardDetected(_) => {
            view.plant_bomb(task_senders);
            None
        },
        _ => None,
    }
}

pub(super) fn render(view: &SearchAndDestroyView, frame: &mut Frame, area: Rect) {
    let timer_ratio = view.round_time_remaining as f64 / (view.round_time_minutes * 60) as f64;
    let minutes = view.round_time_remaining / 60;
    let seconds = view.round_time_remaining % 60;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Timer
            Constraint::Min(0),    // Info
        ])
        .split(area);

    let timer_gauge = Gauge::default()
        .block(Block::default().title("Round Timer").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Blue))
        .ratio(timer_ratio)
        .label(format!("{minutes}:{seconds:02}"));
    frame.render_widget(timer_gauge, chunks[0]);

    let info_items: Vec<ListItem> = [format!("T: {} | CT: {}", view.terrorist_score, view.counter_terrorist_score),
        format!("Round: {}/{}", view.round_number, view.max_rounds),
        "Round Active".to_string()]
    .iter()
    .map(|item| ListItem::new(Line::from(Span::styled(item.clone(), Style::default().fg(Color::White)))))
    .collect();

    let instructions = Line::from(vec![
        "Plant ".into(),
        "<3>".into(),
        " End ".into(),
        "<6/9>".into(),
    ]);
    
    let info_list = List::new(info_items)
        .block(Block::default()
            .title("Round in Progress")
            .title_bottom(instructions)
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Yellow)));
    
    frame.render_widget(info_list, chunks[1]);
}
