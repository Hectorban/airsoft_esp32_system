use ratatui::{
    Frame,
    layout::{Rect, Layout, Direction, Constraint},
    style::{Style, Color, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Gauge, List, ListItem},
};
use alloc::{format, vec::Vec};

use crate::{
    events::{InputEvent, TaskSenders},
    tasks::output::sound::SoundCommand,
};

use super::{View, ViewType, NavigationAction};

#[derive(Debug, Clone, PartialEq)]
pub enum TheFinalsGameState {
    Waiting,
    Active,
    CashoutPhase,
    Ended,
}

#[derive(Debug, Clone)]
pub struct CashToken {
    pub id: u8,
    pub value: u32,
    pub location: TokenLocation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenLocation {
    Field,
    Captured(Team),
    Lost,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Team {
    Team1,
    Team2,
    Team3,
}

pub struct TheFinalsView {
    game_state: TheFinalsGameState,
    team1_cash: u32,
    team2_cash: u32,
    team3_cash: u32,
    cash_tokens: Vec<CashToken>,
    selected_team_index: usize,
    match_time_remaining: u32, // in seconds
    cashout_time_remaining: Option<u32>,
    tokens_in_field: u8,
    target_cash: u32,
}

impl TheFinalsView {
    pub fn new() -> Self {
        let mut tokens = Vec::new();
        // Initialize with some cash tokens
        for i in 0..5 {
            tokens.push(CashToken {
                id: i,
                value: 1000,
                location: TokenLocation::Field,
            });
        }
        
        Self {
            game_state: TheFinalsGameState::Waiting,
            team1_cash: 0,
            team2_cash: 0,
            team3_cash: 0,
            cash_tokens: tokens,
            selected_team_index: 0,
            match_time_remaining: 900, // 15 minutes
            cashout_time_remaining: None,
            tokens_in_field: 5,
            target_cash: 10000,
        }
    }
    
    fn start_match(&mut self, task_senders: &TaskSenders) {
        self.game_state = TheFinalsGameState::Active;
        self.match_time_remaining = 900; // 15 minutes
        
        // Drop all tokens back to field
        for token in &mut self.cash_tokens {
            token.location = TokenLocation::Field;
        }
        self.tokens_in_field = self.cash_tokens.len() as u8;
        
        // Play match start sound
        task_senders.sound.try_send(SoundCommand::Beep).ok();
    }
    
    fn pickup_token_nfc(&mut self, task_senders: &TaskSenders) {
        if self.game_state != TheFinalsGameState::Active {
            return;
        }
        
        // Find an available token in the field
        if let Some(token) = self.cash_tokens.iter_mut().find(|t| t.location == TokenLocation::Field) {
            // For now, just assign to Team 1 - could be enhanced with team selection
            let team = Team::Team1;
            
            token.location = TokenLocation::Captured(team.clone());
            self.tokens_in_field -= 1;
            
            match team {
                Team::Team1 => self.team1_cash += token.value,
                Team::Team2 => self.team2_cash += token.value,
                Team::Team3 => self.team3_cash += token.value,
            }
            
            task_senders.sound.try_send(SoundCommand::SuccessBeep).ok();
            
            // Check if any team reached target
            if self.team1_cash >= self.target_cash || 
               self.team2_cash >= self.target_cash || 
               self.team3_cash >= self.target_cash {
                self.start_cashout_phase(task_senders);
            }
        }
    }
    
    fn start_cashout_phase(&mut self, task_senders: &TaskSenders) {
        self.game_state = TheFinalsGameState::CashoutPhase;
        self.cashout_time_remaining = Some(60); // 1 minute to cash out
        task_senders.sound.try_send(SoundCommand::VictorySound).ok();
    }
    
    fn complete_cashout(&mut self, task_senders: &TaskSenders) {
        self.game_state = TheFinalsGameState::Ended;
        self.cashout_time_remaining = None;
        task_senders.sound.try_send(SoundCommand::VictorySound).ok();
    }
    
    fn move_team_selection(&mut self, direction: i8) {
        match direction {
            -1 => {
                if self.selected_team_index > 0 {
                    self.selected_team_index -= 1;
                } else {
                    self.selected_team_index = 2; // 3 teams (0, 1, 2)
                }
            },
            1 => {
                if self.selected_team_index < 2 {
                    self.selected_team_index += 1;
                } else {
                    self.selected_team_index = 0;
                }
            },
            _ => {}
        }
    }
    
    fn award_cash_to_selected_team(&mut self, amount: u32, task_senders: &TaskSenders) {
        if self.game_state != TheFinalsGameState::Active {
            return;
        }
        
        match self.selected_team_index {
            0 => self.team1_cash += amount,
            1 => self.team2_cash += amount,
            2 => self.team3_cash += amount,
            _ => return,
        }
        
        task_senders.sound.try_send(SoundCommand::SuccessBeep).ok();
        
        // Check if target reached
        if self.team1_cash >= self.target_cash || 
           self.team2_cash >= self.target_cash || 
           self.team3_cash >= self.target_cash {
            self.start_cashout_phase(task_senders);
        }
    }
    
    fn reset_match(&mut self) {
        self.game_state = TheFinalsGameState::Waiting;
        self.team1_cash = 0;
        self.team2_cash = 0;
        self.team3_cash = 0;
        self.match_time_remaining = 900;
        self.cashout_time_remaining = None;
        self.selected_team_index = 0;
        
        // Reset all tokens to field
        for token in &mut self.cash_tokens {
            token.location = TokenLocation::Field;
        }
        self.tokens_in_field = self.cash_tokens.len() as u8;
    }
    
    fn get_winning_team(&self) -> Option<Team> {
        let max_cash = self.team1_cash.max(self.team2_cash.max(self.team3_cash));
        
        if self.team1_cash == max_cash && max_cash > self.team2_cash && max_cash > self.team3_cash {
            Some(Team::Team1)
        } else if self.team2_cash == max_cash && max_cash > self.team1_cash && max_cash > self.team3_cash {
            Some(Team::Team2)
        } else if self.team3_cash == max_cash && max_cash > self.team1_cash && max_cash > self.team2_cash {
            Some(Team::Team3)
        } else {
            None // Tie or no clear winner
        }
    }
}

impl View for TheFinalsView {
    fn handle_input(&mut self, event: InputEvent, task_senders: &TaskSenders) -> Option<NavigationAction> {
        match event {
            InputEvent::KeypadEvent(key) => {
                match key {
                    '1' => { // '1' key - Start/Reset match
                        match self.game_state {
                            TheFinalsGameState::Waiting | TheFinalsGameState::Ended => {
                                if self.game_state == TheFinalsGameState::Ended {
                                    self.reset_match();
                                } else {
                                    self.start_match(task_senders);
                                }
                            },
                            _ => {}
                        }
                        None
                    },
                    '4' => { // '4' key - Move team selection left
                        self.move_team_selection(-1);
                        task_senders.sound.try_send(SoundCommand::Beep).ok();
                        None
                    },
                    '6' => { // '6' key - Move team selection right
                        self.move_team_selection(1);
                        task_senders.sound.try_send(SoundCommand::Beep).ok();
                        None
                    },
                    '5' => { // '5' key - Award cash to selected team
                        self.award_cash_to_selected_team(1000, task_senders);
                        None
                    },
                    '7' => { // '7' key - Complete cashout (if in cashout phase)
                        if self.game_state == TheFinalsGameState::CashoutPhase {
                            self.complete_cashout(task_senders);
                        }
                        None
                    },
                    '0' => { // '0' key - Back to main menu
                        Some(NavigationAction::Back)
                    },
                    _ => None,
                }
            },
            InputEvent::CardDetected => {
                // This is the key feature - NFC tokens for cash pickup
                self.pickup_token_nfc(task_senders);
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
                Constraint::Length(4), // Cash scores
                Constraint::Length(3), // Tokens in field
                Constraint::Length(3), // Timer/Cashout
                Constraint::Length(3), // Game state
                Constraint::Min(3),    // Instructions
            ])
            .split(area);
        
        // Title
        let title = Paragraph::new("THE FINALS - Cashout")
            .style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);
        
        // Cash scores
        let team_items: Vec<ListItem> = [
            ("Team 1", self.team1_cash, Team::Team1),
            ("Team 2", self.team2_cash, Team::Team2),
            ("Team 3", self.team3_cash, Team::Team3),
        ]
        .iter()
        .enumerate()
        .map(|(i, (name, cash, _team))| {
            let cash_text = format!("{}: ${}", name, cash);
            let style = if i == self.selected_team_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                match i {
                    0 => Style::default().fg(Color::Blue),
                    1 => Style::default().fg(Color::Red),
                    2 => Style::default().fg(Color::Green),
                    _ => Style::default().fg(Color::White),
                }
            };
            
            ListItem::new(Line::from(Span::styled(cash_text, style)))
        })
        .collect();
        
        let cash_list = List::new(team_items)
            .block(Block::default()
                .title("Cash (4/6 to select team)")
                .borders(Borders::ALL));
        frame.render_widget(cash_list, chunks[1]);
        
        // Tokens in field
        let tokens_text = format!("Cash Tokens in Field: {}", self.tokens_in_field);
        let tokens = Paragraph::new(tokens_text)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().title("Field Status").borders(Borders::ALL));
        frame.render_widget(tokens, chunks[2]);
        
        // Timer/Cashout
        if let Some(cashout_time) = self.cashout_time_remaining {
            let cashout_gauge = Gauge::default()
                .block(Block::default().title("CASHOUT TIMER").borders(Borders::ALL))
                .gauge_style(Style::default().fg(Color::Magenta))
                .ratio(cashout_time as f64 / 60.0);
            frame.render_widget(cashout_gauge, chunks[3]);
        } else {
            let timer_text = format!("Match Time: {}:{:02}", 
                self.match_time_remaining / 60, 
                self.match_time_remaining % 60);
            let timer = Paragraph::new(timer_text)
                .style(Style::default().fg(Color::Cyan))
                .block(Block::default().title("Match Timer").borders(Borders::ALL));
            frame.render_widget(timer, chunks[3]);
        }
        
        // Game state
        let state_text = match self.game_state {
            TheFinalsGameState::Waiting => "Waiting to start...",
            TheFinalsGameState::Active => "Match in progress - Collect cash tokens!",
            TheFinalsGameState::CashoutPhase => "CASHOUT PHASE! Secure your winnings!",
            TheFinalsGameState::Ended => {
                match self.get_winning_team() {
                    Some(Team::Team1) => "TEAM 1 WINS!",
                    Some(Team::Team2) => "TEAM 2 WINS!",
                    Some(Team::Team3) => "TEAM 3 WINS!",
                    None => "TIE GAME!",
                }
            },
        };
        let state_color = match self.game_state {
            TheFinalsGameState::CashoutPhase => Color::Magenta,
            TheFinalsGameState::Ended => Color::Green,
            _ => Color::White,
        };
        let state = Paragraph::new(state_text)
            .style(Style::default().fg(state_color).add_modifier(Modifier::BOLD))
            .block(Block::default().title("Status").borders(Borders::ALL));
        frame.render_widget(state, chunks[4]);
        
        // Instructions
        let instructions = match self.game_state {
            TheFinalsGameState::Waiting => "1: Start Match | 0: Back to Menu",
            TheFinalsGameState::Active => "NFC: Pickup Cash | 4/6: Select Team | 5: Award Cash | 0: Back",
            TheFinalsGameState::CashoutPhase => "7: Complete Cashout | 0: Back to Menu",
            TheFinalsGameState::Ended => "1: New Match | 0: Back to Menu",
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
        ViewType::TheFinals
    }
}
