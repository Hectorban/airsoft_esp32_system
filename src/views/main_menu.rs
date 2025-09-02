use ratatui::{
    Frame,
    layout::{Rect, Layout, Direction, Constraint},
    style::{Style, Color, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use alloc::vec::Vec;
use alloc::vec;

use crate::{
    events::{InputEvent, TaskSenders},
    tasks::output::sound::SoundCommand,
};

use super::{View, ViewType, NavigationAction};

pub struct MainMenuView {
    selected_index: usize,
    menu_items: [&'static str; 4],
}

impl Default for MainMenuView {
    fn default() -> Self {
        Self::new()
    }
}

impl MainMenuView {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            menu_items: [
                "Search & Destroy",
                "Domination", 
                "Cashout",
                "Configuration",
            ],
        }
    }
    
    fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            self.selected_index = self.menu_items.len() - 1;
        }
    }
    
    fn move_selection_down(&mut self) {
        if self.selected_index < self.menu_items.len() - 1 {
            self.selected_index += 1;
        } else {
            self.selected_index = 0;
        }
    }
    
    fn select_current_item(&self) -> Option<NavigationAction> {
        match self.selected_index {
            0 => Some(NavigationAction::GoTo(ViewType::SearchAndDestroy)),
            1 => Some(NavigationAction::GoTo(ViewType::Battlefield)),
            2 => Some(NavigationAction::GoTo(ViewType::TheFinals)),
            _ => None,
        }
    }
}

impl View for MainMenuView {
    fn handle_input(&mut self, event: InputEvent, task_senders: &TaskSenders) -> Option<NavigationAction> {
        match event {
            InputEvent::KeypadEvent(key) => {
                match key {
                    'a' => { // Up arrow or '2' key
                        self.move_selection_up();
                        task_senders.sound.try_send(SoundCommand::Beep).ok();
                        None
                    },
                    'b' => { // Down arrow or '8' key  
                        self.move_selection_down();
                        task_senders.sound.try_send(SoundCommand::Beep).ok();
                        None
                    },
                    'd' => { // Enter/Select or '5' key
                        task_senders.sound.try_send(SoundCommand::SuccessBeep).ok();
                        self.select_current_item()
                    },
                    _ => None,
                }
            },
            InputEvent::CardDetected => {
                // NFC detected on main menu - could be used for quick game selection
                task_senders.sound.try_send(SoundCommand::Beep).ok();
                None
            },
            _ => None,
        }
    }
    
    fn render(&self, frame: &mut Frame, area: Rect) {
        // Menu items
        let menu_items: Vec<ListItem> = self.menu_items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == self.selected_index {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                
                ListItem::new(Line::from(Span::styled(*item, style)))
            })
            .collect();

        let instructions = Line::from(vec![
            " → ".into(),
            "<D>".into(),
            " ↑ ".into(),
            "<A>".into(),
            " ↓ ".into(),
            "<B>".into(),
        ]);
        
        let menu_list = List::new(menu_items)
            .block(Block::default()
                .title("Select Game Mode")
                .title_bottom(instructions)
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White)));
        
        frame.render_widget(menu_list, area);
    }
    
    fn on_enter(&mut self, task_senders: &TaskSenders) {
        // Play welcome sound when entering main menu
        task_senders.sound.try_send(SoundCommand::Beep).ok();
    }
    
    fn view_type(&self) -> ViewType {
        ViewType::MainMenu
    }
}
