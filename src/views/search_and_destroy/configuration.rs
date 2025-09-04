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
    events::InputEvent,
    views::NavigationAction,
};
use super::SearchAndDestroyView;

pub(super) fn handle_input(view: &mut SearchAndDestroyView, event: InputEvent) -> Option<NavigationAction> {
    if let InputEvent::KeypadEvent(key) = event {
        match key {
            'a' | 'A' => { // Move up
                view.move_config_selection_up();
                None
            },
            'd' | 'D' => { // Move down
                view.move_config_selection_down();
                None
            },
            'b' | 'B' => { // Adjust value (will cycle through + and -)
                // For now, just increase. We can add decrease with different key later
                view.adjust_config_value(true);
                None
            },
            '1' => { // Start game
                view.apply_configuration();
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
    let config_items: Vec<ListItem> = [format!("Max Rounds: {}", view.max_rounds),
        format!("Round Time: {}min", view.round_time_minutes),
        format!("Bomb Timer: {}s", view.bomb_timer_seconds)]
    .iter()
    .enumerate()
    .map(|(i, item)| {
        let style = if i == view.selected_config_index {
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        
        ListItem::new(Line::from(Span::styled(item.clone(), style)))
    })
    .collect();

    let instructions = Line::from(vec![
        " ↑ ".into(),
        "<A>".into(),
        " ↓ ".into(),
        "<D>".into(),
        " ± ".into(),
        "<B>".into(),
    ]);
    
    let config_list = List::new(config_items)
        .block(Block::default()
            .title("Search & Destroy Config")
            .title_bottom(instructions)
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Yellow)));
    
    frame.render_widget(config_list, area);
}
