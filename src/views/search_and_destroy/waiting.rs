use ratatui::{
    Frame,
    layout::{Rect},
    style::{Style, Color},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
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
    if let InputEvent::KeypadEvent(key) = event {
        match key {
            '1' => { // Start round
                view.start_round(task_senders);
                None
            },
            '0' => { // Back to main menu
                Some(NavigationAction::Back)
            },
            _ => None,
        }
    } else {
        None
    }
}

pub(super) fn render(view: &SearchAndDestroyView, frame: &mut Frame, area: Rect) {
    let info_items: Vec<ListItem> = [format!("T: {} | CT: {}", view.terrorist_score, view.counter_terrorist_score),
        format!("Round: {}/{}", view.round_number, view.max_rounds),
        format!("Time: {}min", view.round_time_minutes),
        "Ready to start...".to_string()]
    .iter()
    .map(|item| ListItem::new(Line::from(Span::styled(item.clone(), Style::default().fg(Color::White)))))
    .collect();

    let instructions = Line::from(vec![
        "Start ".into(),
        "<1>".into(),
        " Back ".into(),
        "<0>".into(),
    ]);
    
    let info_list = List::new(info_items)
        .block(Block::default()
            .title("Waiting to Start")
            .title_bottom(instructions)
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Green)));
    
    frame.render_widget(info_list, area);
}
