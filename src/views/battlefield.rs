use ratatui::{
    Frame,
    layout::{Rect, Layout, Direction, Constraint},
    style::{Style, Color, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, List, ListItem},
};
use alloc::{format, vec::Vec};

use crate::{
    events::{InputEvent, TaskSenders},
    tasks::output::sound::SoundCommand,
};

use super::{View, ViewType, NavigationAction};

#[derive(Debug, Clone, PartialEq)]
pub enum BattlefieldGameState {
    Waiting,
    Active,
    Ended,
}

#[derive(Debug, Clone)]
pub struct ControlPoint {
    pub name: &'static str,
    pub owner: Option<Team>,
    pub capture_progress: u8, // 0-100%
}

#[derive(Debug, Clone, PartialEq)]
pub enum Team {
    Team1,
    Team2,
}

pub struct BattlefieldView {
    game_state: BattlefieldGameState,
    team1_tickets: u32,
    team2_tickets: u32,
    control_points: [ControlPoint; 3],
    selected_point_index: usize,
    match_time_remaining: u32, // in seconds
    max_tickets: u32,
}

impl Default for BattlefieldView {
    fn default() -> Self {
        Self::new()
    }
}

impl BattlefieldView {
    pub fn new() -> Self {
        Self {
            game_state: BattlefieldGameState::Waiting,
            team1_tickets: 500,
            team2_tickets: 500,
            control_points: [
                ControlPoint {
                    name: "Alpha",
                    owner: None,
                    capture_progress: 0,
                },
                ControlPoint {
                    name: "Bravo",
                    owner: None,
                    capture_progress: 0,
                },
                ControlPoint {
                    name: "Charlie",
                    owner: None,
                    capture_progress: 0,
                },
            ],
            selected_point_index: 0,
            match_time_remaining: 1200, // 20 minutes
            max_tickets: 500,
        }
    }
    
    fn start_match(&mut self, task_senders: &TaskSenders) {
        self.game_state = BattlefieldGameState::Active;
        self.match_time_remaining = 1200; // 20 minutes
        
        // Play match start sound
        task_senders.sound.try_send(SoundCommand::Beep).ok();
    }
    
    fn capture_point(&mut self, team: Team, task_senders: &TaskSenders) {
        let point = &mut self.control_points[self.selected_point_index];
        
        match &point.owner {
            Some(current_owner) if *current_owner == team => {
                // Already owned by this team
            },
            _ => {
                point.owner = Some(team);
                point.capture_progress = 100;
                task_senders.sound.try_send(SoundCommand::SuccessBeep).ok();
            }
        }
    }
    
    fn neutralize_point(&mut self, task_senders: &TaskSenders) {
        let point = &mut self.control_points[self.selected_point_index];
        point.owner = None;
        point.capture_progress = 0;
        task_senders.sound.try_send(SoundCommand::Beep).ok();
    }
    
    fn move_selection_left(&mut self) {
        if self.selected_point_index > 0 {
            self.selected_point_index -= 1;
        } else {
            self.selected_point_index = self.control_points.len() - 1;
        }
    }
    
    fn move_selection_right(&mut self) {
        if self.selected_point_index < self.control_points.len() - 1 {
            self.selected_point_index += 1;
        } else {
            self.selected_point_index = 0;
        }
    }
    
    fn deduct_tickets(&mut self, team: Team, amount: u32) {
        match team {
            Team::Team1 => {
                self.team1_tickets = self.team1_tickets.saturating_sub(amount);
            },
            Team::Team2 => {
                self.team2_tickets = self.team2_tickets.saturating_sub(amount);
            }
        }
        
        if self.team1_tickets == 0 || self.team2_tickets == 0 {
            self.game_state = BattlefieldGameState::Ended;
        }
    }
    
    fn reset_match(&mut self) {
        self.game_state = BattlefieldGameState::Waiting;
        self.team1_tickets = self.max_tickets;
        self.team2_tickets = self.max_tickets;
        self.match_time_remaining = 1200;
        self.selected_point_index = 0;
        
        for point in &mut self.control_points {
            point.owner = None;
            point.capture_progress = 0;
        }
    }
    
    fn is_match_over(&self) -> bool {
        self.team1_tickets == 0 || self.team2_tickets == 0 || self.match_time_remaining == 0
    }
}

impl View for BattlefieldView {
    fn handle_input(&mut self, event: InputEvent, task_senders: &TaskSenders) -> Option<NavigationAction> {
        match event {
            InputEvent::KeypadEvent(key) => {
                match key {
                    '1' => { // '1' key - Start/Reset match
                        match self.game_state {
                            BattlefieldGameState::Waiting | BattlefieldGameState::Ended => {
                                if self.is_match_over() {
                                    self.reset_match();
                                } else {
                                    self.start_match(task_senders);
                                }
                            },
                            _ => {}
                        }
                        None
                    },
                    '4' => { // '4' key - Move selection left
                        self.move_selection_left();
                        task_senders.sound.try_send(SoundCommand::Beep).ok();
                        None
                    },
                    '6' => { // '6' key - Move selection right
                        self.move_selection_right();
                        task_senders.sound.try_send(SoundCommand::Beep).ok();
                        None
                    },
                    '7' => { // '7' key - Team 1 captures selected point
                        if self.game_state == BattlefieldGameState::Active {
                            self.capture_point(Team::Team1, task_senders);
                        }
                        None
                    },
                    '9' => { // '9' key - Team 2 captures selected point
                        if self.game_state == BattlefieldGameState::Active {
                            self.capture_point(Team::Team2, task_senders);
                        }
                        None
                    },
                    '5' => { // '5' key - Neutralize selected point
                        if self.game_state == BattlefieldGameState::Active {
                            self.neutralize_point(task_senders);
                        }
                        None
                    },
                    '2' => { // '2' key - Team 1 loses tickets
                        if self.game_state == BattlefieldGameState::Active {
                            self.deduct_tickets(Team::Team1, 10);
                            task_senders.sound.try_send(SoundCommand::ErrorBeep).ok();
                        }
                        None
                    },
                    '8' => { // '8' key - Team 2 loses tickets
                        if self.game_state == BattlefieldGameState::Active {
                            self.deduct_tickets(Team::Team2, 10);
                            task_senders.sound.try_send(SoundCommand::ErrorBeep).ok();
                        }
                        None
                    },
                    '0' => { // '0' key - Back to main menu
                        Some(NavigationAction::Back)
                    },
                    _ => None,
                }
            },
            InputEvent::CardDetected(_) => {
                // NFC could be used for point capture
                if self.game_state == BattlefieldGameState::Active {
                    // Randomly assign to team based on some logic or use RNG
                    self.capture_point(Team::Team1, task_senders);
                }
                None
            },
            _ => None,
        }
    }
    
    fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(3), // Tickets
                Constraint::Length(6), // Control points
                Constraint::Length(3), // Timer
                Constraint::Length(3), // Status
                Constraint::Min(3),    // Instructions
            ])
            .split(area);
        
        // Title
        let title = Paragraph::new("BATTLEFIELD - Domination")
            .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);
        
        // Tickets
        let tickets_text = format!("Team 1: {} | Team 2: {}", self.team1_tickets, self.team2_tickets);
        let tickets = Paragraph::new(tickets_text)
            .style(Style::default().fg(Color::White))
            .block(Block::default().title("Tickets").borders(Borders::ALL));
        frame.render_widget(tickets, chunks[1]);
        
        // Control Points
        let points_items: Vec<ListItem> = self.control_points
            .iter()
            .enumerate()
            .map(|(i, point)| {
                let owner_text = match &point.owner {
                    Some(Team::Team1) => " [T1]",
                    Some(Team::Team2) => " [T2]",
                    None => " [--]",
                };
                
                let point_text = format!("{}{}", point.name, owner_text);
                let style = if i == self.selected_point_index {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    match &point.owner {
                        Some(Team::Team1) => Style::default().fg(Color::Blue),
                        Some(Team::Team2) => Style::default().fg(Color::Red),
                        None => Style::default().fg(Color::Gray),
                    }
                };
                
                ListItem::new(Line::from(Span::styled(point_text, style)))
            })
            .collect();
        
        let points_list = List::new(points_items)
            .block(Block::default()
                .title("Control Points (4/6 to select)")
                .borders(Borders::ALL));
        frame.render_widget(points_list, chunks[2]);
        
        // Timer
        let timer_text = format!("Time: {}:{:02}", 
            self.match_time_remaining / 60, 
            self.match_time_remaining % 60);
        let timer = Paragraph::new(timer_text)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().title("Match Timer").borders(Borders::ALL));
        frame.render_widget(timer, chunks[3]);
        
        // Status
        let status_text = match self.game_state {
            BattlefieldGameState::Waiting => "Waiting to start...",
            BattlefieldGameState::Active => "Match in progress",
            BattlefieldGameState::Ended => {
                if self.team1_tickets > self.team2_tickets {
                    "TEAM 1 WINS!"
                } else if self.team2_tickets > self.team1_tickets {
                    "TEAM 2 WINS!"
                } else {
                    "DRAW!"
                }
            },
        };
        let status_color = match self.game_state {
            BattlefieldGameState::Ended => Color::Green,
            _ => Color::White,
        };
        let status = Paragraph::new(status_text)
            .style(Style::default().fg(status_color).add_modifier(Modifier::BOLD))
            .block(Block::default().title("Status").borders(Borders::ALL));
        frame.render_widget(status, chunks[4]);
        
        // Instructions
        let instructions = match self.game_state {
            BattlefieldGameState::Waiting => "1: Start Match | 0: Back to Menu",
            BattlefieldGameState::Active => "7: T1 Cap | 9: T2 Cap | 5: Neutral | 2/8: -Tickets | 0: Back",
            BattlefieldGameState::Ended => "1: New Match | 0: Back to Menu",
        };
        let instr_widget = Paragraph::new(instructions)
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(instr_widget, chunks[5]);
    }
    
    fn on_enter(&mut self, task_senders: &TaskSenders) {
        task_senders.sound.try_send(SoundCommand::Beep).ok();
    }
    
    fn view_type(&self) -> ViewType {
        ViewType::Battlefield
    }
}
