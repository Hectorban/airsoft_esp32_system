use ratatui::{
    Frame,
    layout::{Rect},
    style::{Style, Color, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};
use alloc::format;
use alloc::vec::Vec;
use alloc::vec;
use alloc::string::ToString;

use crate::{
    events::{InputEvent},
    views::NavigationAction,
};
use super::SearchAndDestroyView;

pub(super) fn handle_input(view: &mut SearchAndDestroyView, event: InputEvent) -> Option<NavigationAction> {
    if let InputEvent::KeypadEvent(key) = event {
        match key {
            '1' => { // Next round or restart
                if view.is_match_over() {
                    view.reset_game();
                    view.start_configuration();
                } else {
                    view.next_round_or_end();
                }
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
    let (winner_msg, winner_color) = if view.is_match_over() {
        if view.terrorist_score > view.counter_terrorist_score {
            ("TERRORISTS WIN MATCH!", Color::Red)
        } else {
            ("CT WIN MATCH!", Color::Blue)
        }
    } else {
        ("Round Complete", Color::Green)
    };

    let info_items: Vec<ListItem> = [format!("T: {} | CT: {}", view.terrorist_score, view.counter_terrorist_score),
        format!("Round: {}/{}", view.round_number, view.max_rounds),
        winner_msg.to_string()]
    .iter()
    .enumerate()
    .map(|(i, item)| {
        let style = if i == 2 {
            Style::default().fg(winner_color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        ListItem::new(Line::from(Span::styled(item.clone(), style)))
    })
    .collect();

    let instructions = if view.is_match_over() {
        Line::from(vec![
            "Config ".into(),
            "<1>".into(),
            " Back ".into(),
            "<0>".into(),
        ])
    } else {
        Line::from(vec![
            "Next ".into(),
            "<1>".into(),
            " Back ".into(),
            "<0>".into(),
        ])
    };
    
    let info_list = List::new(info_items)
        .block(Block::default()
            .title("Round Results")
            .title_bottom(instructions)
            .borders(Borders::ALL)
            .style(Style::default().fg(winner_color)));
    
    frame.render_widget(info_list, area);
}
