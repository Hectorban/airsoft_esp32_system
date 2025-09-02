use ratatui::{
    Frame,
    layout::{Rect, Layout, Direction, Constraint},
    style::{Style, Color, Modifier},
    widgets::{Block, Borders, Paragraph, Gauge},
};
use alloc::format;

use crate::{
    events::{InputEvent, TaskSenders},
    tasks::output::sound::SoundCommand,
};

use super::{View, ViewType, NavigationAction};

#[derive(Debug, Clone, PartialEq)]
pub enum CSGOGameState {
    Waiting,
    RoundActive,
    BombPlanted,
    RoundEnded,
}

pub struct CSGOView {
    game_state: CSGOGameState,
    round_number: u32,
    terrorist_score: u32,
    counter_terrorist_score: u32,
    round_time_remaining: u32, // in seconds
    bomb_time_remaining: Option<u32>, // Some when bomb is planted
    max_rounds: u32,
}

impl CSGOView {
    pub fn new() -> Self {
        Self {
            game_state: CSGOGameState::Waiting,
            round_number: 0,
            terrorist_score: 0,
            counter_terrorist_score: 0,
            round_time_remaining: 120, // 2 minutes default
            bomb_time_remaining: None,
            max_rounds: 16,
        }
    }
    
    fn start_round(&mut self, task_senders: &TaskSenders) {
        self.game_state = CSGOGameState::RoundActive;
        self.round_number += 1;
        self.round_time_remaining = 120; // 2 minutes
        self.bomb_time_remaining = None;
        
        // Play round start sound
        task_senders.sound.try_send(SoundCommand::Beep).ok();
    }
    
    fn plant_bomb(&mut self, task_senders: &TaskSenders) {
        if self.game_state == CSGOGameState::RoundActive {
            self.game_state = CSGOGameState::BombPlanted;
            self.bomb_time_remaining = Some(40); // 40 seconds to defuse
            
            // Play bomb plant sound
            task_senders.sound.try_send(SoundCommand::ErrorBeep).ok();
        }
    }
    
    fn defuse_bomb(&mut self, task_senders: &TaskSenders) {
        if self.game_state == CSGOGameState::BombPlanted {
            self.game_state = CSGOGameState::RoundEnded;
            self.counter_terrorist_score += 1;
            self.bomb_time_remaining = None;
            
            // Play defuse success sound
            task_senders.sound.try_send(SoundCommand::SuccessBeep).ok();
        }
    }
    
    fn end_round(&mut self, terrorists_win: bool, task_senders: &TaskSenders) {
        self.game_state = CSGOGameState::RoundEnded;
        self.bomb_time_remaining = None;
        
        if terrorists_win {
            self.terrorist_score += 1;
            task_senders.sound.try_send(SoundCommand::DefeatSound).ok();
        } else {
            self.counter_terrorist_score += 1;
            task_senders.sound.try_send(SoundCommand::VictorySound).ok();
        }
    }
    
    fn reset_game(&mut self) {
        self.game_state = CSGOGameState::Waiting;
        self.round_number = 0;
        self.terrorist_score = 0;
        self.counter_terrorist_score = 0;
        self.round_time_remaining = 120;
        self.bomb_time_remaining = None;
    }
    
    fn is_match_over(&self) -> bool {
        let half_rounds = self.max_rounds / 2;
        self.terrorist_score > half_rounds || self.counter_terrorist_score > half_rounds
    }
}

impl View for CSGOView {
    fn handle_input(&mut self, event: InputEvent, task_senders: &TaskSenders) -> Option<NavigationAction> {
        match event {
            InputEvent::KeypadEvent(key) => {
                match key {
                    '1' => { // '1' key - Start/Reset round
                        match self.game_state {
                            CSGOGameState::Waiting | CSGOGameState::RoundEnded => {
                                if self.is_match_over() {
                                    self.reset_game();
                                } else {
                                    self.start_round(task_senders);
                                }
                            },
                            _ => {}
                        }
                        None
                    },
                    '3' => { // '3' key - Plant bomb
                        self.plant_bomb(task_senders);
                        None
                    },
                    '7' => { // '7' key - Defuse bomb
                        self.defuse_bomb(task_senders);
                        None
                    },
                    '9' => { // '9' key - Terrorists win round
                        if self.game_state == CSGOGameState::RoundActive || 
                           self.game_state == CSGOGameState::BombPlanted {
                            self.end_round(true, task_senders);
                        }
                        None
                    },
                    '6' => { // '6' key - Counter-terrorists win round
                        if self.game_state == CSGOGameState::RoundActive || 
                           self.game_state == CSGOGameState::BombPlanted {
                            self.end_round(false, task_senders);
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
                // NFC could be used for bomb planting/defusing
                match self.game_state {
                    CSGOGameState::RoundActive => {
                        self.plant_bomb(task_senders);
                    },
                    CSGOGameState::BombPlanted => {
                        self.defuse_bomb(task_senders);
                    },
                    _ => {}
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
                Constraint::Length(3), // Score
                Constraint::Length(3), // Round info
                Constraint::Length(3), // Game state
                Constraint::Min(3),    // Timer/Bomb timer
                Constraint::Length(4), // Instructions
            ])
            .split(area);
        
        // Title
        let title = Paragraph::new("CSGO - Search & Destroy")
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);
        
        // Score
        let score_text = format!("T: {} | CT: {}", self.terrorist_score, self.counter_terrorist_score);
        let score = Paragraph::new(score_text)
            .style(Style::default().fg(Color::White))
            .block(Block::default().title("Score").borders(Borders::ALL));
        frame.render_widget(score, chunks[1]);
        
        // Round info
        let round_text = format!("Round: {}/{}", self.round_number, self.max_rounds);
        let round_info = Paragraph::new(round_text)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().title("Round").borders(Borders::ALL));
        frame.render_widget(round_info, chunks[2]);
        
        // Game state
        let state_text = match self.game_state {
            CSGOGameState::Waiting => "Waiting to start...",
            CSGOGameState::RoundActive => "Round in progress",
            CSGOGameState::BombPlanted => "BOMB PLANTED!",
            CSGOGameState::RoundEnded => {
                if self.is_match_over() {
                    if self.terrorist_score > self.counter_terrorist_score {
                        "TERRORISTS WIN!"
                    } else {
                        "COUNTER-TERRORISTS WIN!"
                    }
                } else {
                    "Round ended"
                }
            },
        };
        let state_color = match self.game_state {
            CSGOGameState::BombPlanted => Color::Red,
            CSGOGameState::RoundEnded => Color::Green,
            _ => Color::White,
        };
        let state = Paragraph::new(state_text)
            .style(Style::default().fg(state_color).add_modifier(Modifier::BOLD))
            .block(Block::default().title("Status").borders(Borders::ALL));
        frame.render_widget(state, chunks[3]);
        
        // Timer
        if let Some(bomb_time) = self.bomb_time_remaining {
            let bomb_gauge = Gauge::default()
                .block(Block::default().title("BOMB TIMER").borders(Borders::ALL))
                .gauge_style(Style::default().fg(Color::Red))
                .ratio(bomb_time as f64 / 40.0);
            frame.render_widget(bomb_gauge, chunks[4]);
        } else if self.game_state == CSGOGameState::RoundActive {
            let round_gauge = Gauge::default()
                .block(Block::default().title("Round Timer").borders(Borders::ALL))
                .gauge_style(Style::default().fg(Color::Blue))
                .ratio(self.round_time_remaining as f64 / 120.0);
            frame.render_widget(round_gauge, chunks[4]);
        }
        
        // Instructions
        let instructions = match self.game_state {
            CSGOGameState::Waiting => "1: Start Round | 0: Back to Menu",
            CSGOGameState::RoundActive => "3: Plant Bomb | 6: CT Win | 9: T Win | 0: Back",
            CSGOGameState::BombPlanted => "7: Defuse | 6: CT Win | 9: T Win | 0: Back",
            CSGOGameState::RoundEnded => "1: Next Round | 0: Back to Menu",
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
        ViewType::CSGO
    }
}
