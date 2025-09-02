use ratatui::{
    Frame,
    layout::{Rect, Layout, Direction, Constraint},
    style::{Style, Color, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use alloc::vec::Vec;

use crate::{
    events::{InputEvent, TaskSenders},
    tasks::output::sound::SoundCommand,
};

use super::{View, ViewType, NavigationAction};

pub struct MainMenuView {
    selected_index: usize,
    menu_items: [&'static str; 3],
}

impl MainMenuView {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            menu_items: [
                "CSGO - Search & Destroy",
                "Battlefield - Domination", 
                "The Finals - Cashout"
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
            0 => Some(NavigationAction::GoTo(ViewType::CSGO)),
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
                    '2' => { // Up arrow or '2' key
                        self.move_selection_up();
                        task_senders.sound.try_send(SoundCommand::Beep).ok();
                        None
                    },
                    '8' => { // Down arrow or '8' key  
                        self.move_selection_down();
                        task_senders.sound.try_send(SoundCommand::Beep).ok();
                        None
                    },
                    '5' => { // Enter/Select or '5' key
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
        // Create layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(5),    // Menu items
                Constraint::Length(3), // Instructions
            ])
            .split(area);
        
        // Title
        let title = Paragraph::new("AIRSOFT GAME SYSTEM")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);
        
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
        
        let menu_list = List::new(menu_items)
            .block(Block::default()
                .title("Select Game Mode")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White)));
        
        frame.render_widget(menu_list, chunks[1]);
        
        // Instructions
        let instructions = Paragraph::new("Use 2/8 to navigate, 5 to select")
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(instructions, chunks[2]);
    }
    
    fn on_enter(&mut self, task_senders: &TaskSenders) {
        // Play welcome sound when entering main menu
        task_senders.sound.try_send(SoundCommand::Beep).ok();
    }
    
    fn view_type(&self) -> ViewType {
        ViewType::MainMenu
    }
}
