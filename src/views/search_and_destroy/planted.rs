use ratatui::{
    Frame,
    layout::{Rect, Layout, Direction, Constraint},
    style::{Style, Color, Modifier},
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
                '7' => { // Defuse bomb
                    view.defuse_bomb(task_senders);
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
            view.defuse_bomb(task_senders);
            None
        },
        _ => None,
    }
}

pub(super) fn render(view: &SearchAndDestroyView, frame: &mut Frame, area: Rect) {
    let bomb_time = view.bomb_time_remaining.unwrap_or(0);
    let timer_ratio = bomb_time as f64 / view.bomb_timer_seconds as f64;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Bomb timer
            Constraint::Min(0),    // Info
        ])
        .split(area);

    let bomb_gauge = Gauge::default()
        .block(Block::default().title("BOMB TIMER").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Red))
        .ratio(timer_ratio)
        .label(format!("{bomb_time}s"));
    frame.render_widget(bomb_gauge, chunks[0]);

    let info_items: Vec<ListItem> = [format!("T: {} | CT: {}", view.terrorist_score, view.counter_terrorist_score),
        format!("Round: {}/{}", view.round_number, view.max_rounds),
        "BOMB PLANTED!".to_string()]
    .iter()
    .enumerate()
    .map(|(i, item)| {
        let style = if i == 2 {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        ListItem::new(Line::from(Span::styled(item.clone(), style)))
    })
    .collect();

    let instructions = Line::from(vec![
        "Defuse ".into(),
        "<7>".into(),
        " End ".into(),
        "<6/9>".into(),
    ]);
    
    let info_list = List::new(info_items)
        .block(Block::default()
            .title("Bomb Planted")
            .title_bottom(instructions)
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Red)));
    
    frame.render_widget(info_list, chunks[1]);
}
