use ratatui::{
    Frame,
    layout::{Rect, Layout, Direction, Constraint},
    style::{Style, Color, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Gauge, List, ListItem},
};
use alloc::format;
use alloc::vec::Vec;
use alloc::vec;
use alloc::string::ToString;

use crate::{
    events::{InputEvent, TaskSenders},
    tasks::output::sound::SoundCommand,
};

use super::{View, ViewType, NavigationAction};

#[derive(Debug, Clone, PartialEq)]
pub enum SearchAndDestroyPhase {
    Configuration,
    WaitingToStart,
    RoundActive,
    BombPlanted,
    RoundEnded,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SearchAndDestroyGameState {
    Waiting,
    RoundActive,
    BombPlanted,
    RoundEnded,
}

pub struct SearchAndDestroyView {
    phase: SearchAndDestroyPhase,
    game_state: SearchAndDestroyGameState,
    round_number: u32,
    terrorist_score: u32,
    counter_terrorist_score: u32,
    round_time_remaining: u32, // in seconds
    bomb_time_remaining: Option<u32>, // Some when bomb is planted
    max_rounds: u32,
    // Configuration options
    selected_config_index: usize,
    round_time_minutes: u32,
    bomb_timer_seconds: u32,
}

impl Default for SearchAndDestroyView {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchAndDestroyView {
    pub fn new() -> Self {
        Self {
            phase: SearchAndDestroyPhase::Configuration,
            game_state: SearchAndDestroyGameState::Waiting,
            round_number: 0,
            terrorist_score: 0,
            counter_terrorist_score: 0,
            round_time_remaining: 120, // 2 minutes default
            bomb_time_remaining: None,
            max_rounds: 16,
            selected_config_index: 0,
            round_time_minutes: 2,
            bomb_timer_seconds: 40,
        }
    }

    fn start_configuration(&mut self) {
        self.phase = SearchAndDestroyPhase::Configuration;
        self.selected_config_index = 0;
    }

    fn apply_configuration(&mut self) {
        self.round_time_remaining = self.round_time_minutes * 60;
        self.phase = SearchAndDestroyPhase::WaitingToStart;
    }

    fn move_config_selection_up(&mut self) {
        if self.selected_config_index > 0 {
            self.selected_config_index -= 1;
        } else {
            self.selected_config_index = 2; // 3 config options: 0, 1, 2
        }
    }
    
    fn move_config_selection_down(&mut self) {
        if self.selected_config_index < 2 {
            self.selected_config_index += 1;
        } else {
            self.selected_config_index = 0;
        }
    }

    fn adjust_config_value(&mut self, increase: bool) {
        match self.selected_config_index {
            0 => { // Max rounds
                if increase && self.max_rounds < 30 {
                    self.max_rounds += 2;
                } else if !increase && self.max_rounds > 2 {
                    self.max_rounds -= 2;
                }
            },
            1 => { // Round time
                if increase && self.round_time_minutes < 10 {
                    self.round_time_minutes += 1;
                } else if !increase && self.round_time_minutes > 1 {
                    self.round_time_minutes -= 1;
                }
            },
            2 => { // Bomb timer
                if increase && self.bomb_timer_seconds < 60 {
                    self.bomb_timer_seconds += 5;
                } else if !increase && self.bomb_timer_seconds > 15 {
                    self.bomb_timer_seconds -= 5;
                }
            },
            _ => {}
        }
    }
    
    fn start_round(&mut self, task_senders: &TaskSenders) {
        self.game_state = SearchAndDestroyGameState::RoundActive;
        self.phase = SearchAndDestroyPhase::RoundActive;
        self.round_number += 1;
        self.round_time_remaining = self.round_time_minutes * 60;
        self.bomb_time_remaining = None;
        
        // Play round start sound
        let _ = task_senders.sound.try_send(SoundCommand::PlayTone { frequency: 1000, duration_ms: 200 });
    }
    
    pub fn plant_bomb(&mut self, task_senders: &TaskSenders) {
        self.game_state = SearchAndDestroyGameState::BombPlanted;
        self.phase = SearchAndDestroyPhase::BombPlanted;
        self.bomb_time_remaining = Some(self.bomb_timer_seconds);
        
        // Play bomb plant sound
        let _ = task_senders.sound.try_send(SoundCommand::PlayTone { frequency: 500, duration_ms: 500 });
    }
    
    pub fn defuse_bomb(&mut self, task_senders: &TaskSenders) {
        if self.game_state == SearchAndDestroyGameState::BombPlanted {
            self.bomb_time_remaining = None;
            self.end_round(false, task_senders); // CT wins
            
            // Play defuse sound
            let _ = task_senders.sound.try_send(SoundCommand::PlayTone { frequency: 1500, duration_ms: 300 });
        }
    }
    
    pub fn end_round(&mut self, terrorists_win: bool, task_senders: &TaskSenders) {
        self.game_state = SearchAndDestroyGameState::RoundEnded;
        self.phase = SearchAndDestroyPhase::RoundEnded;
        self.bomb_time_remaining = None;
        
        if terrorists_win {
            self.terrorist_score += 1;
        } else {
            self.counter_terrorist_score += 1;
        }
        
        // Play round end sound
        let _ = task_senders.sound.try_send(SoundCommand::PlayTone { frequency: 800, duration_ms: 400 });
    }

    fn next_round_or_end(&mut self) {
        if self.is_match_over() {
            self.phase = SearchAndDestroyPhase::Configuration;
        } else {
            self.phase = SearchAndDestroyPhase::WaitingToStart;
            self.game_state = SearchAndDestroyGameState::Waiting;
        }
    }
    
    fn reset_game(&mut self) {
        self.game_state = SearchAndDestroyGameState::Waiting;
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

    fn render_configuration(&self, frame: &mut Frame, area: Rect) {
        let config_items: Vec<ListItem> = vec![
            format!("Max Rounds: {}", self.max_rounds),
            format!("Round Time: {}min", self.round_time_minutes),
            format!("Bomb Timer: {}s", self.bomb_timer_seconds),
        ]
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == self.selected_config_index {
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

    fn render_waiting(&self, frame: &mut Frame, area: Rect) {
        let info_items: Vec<ListItem> = vec![
            format!("T: {} | CT: {}", self.terrorist_score, self.counter_terrorist_score),
            format!("Round: {}/{}", self.round_number, self.max_rounds),
            format!("Time: {}min", self.round_time_minutes),
            "Ready to start...".to_string(),
        ]
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

    fn render_round_active(&self, frame: &mut Frame, area: Rect) {
        // Calculate timer ratio
        let timer_ratio = self.round_time_remaining as f64 / (self.round_time_minutes * 60) as f64;
        let minutes = self.round_time_remaining / 60;
        let seconds = self.round_time_remaining % 60;

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Timer
                Constraint::Min(0),    // Info
            ])
            .split(area);

        // Timer gauge
        let timer_gauge = Gauge::default()
            .block(Block::default().title("Round Timer").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Blue))
            .ratio(timer_ratio)
            .label(format!("{}:{:02}", minutes, seconds));
        frame.render_widget(timer_gauge, chunks[0]);

        // Info list
        let info_items: Vec<ListItem> = vec![
            format!("T: {} | CT: {}", self.terrorist_score, self.counter_terrorist_score),
            format!("Round: {}/{}", self.round_number, self.max_rounds),
            "Round Active".to_string(),
        ]
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

    fn render_bomb_planted(&self, frame: &mut Frame, area: Rect) {
        let bomb_time = self.bomb_time_remaining.unwrap_or(0);
        let timer_ratio = bomb_time as f64 / self.bomb_timer_seconds as f64;

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Bomb timer
                Constraint::Min(0),    // Info
            ])
            .split(area);

        // Bomb timer gauge
        let bomb_gauge = Gauge::default()
            .block(Block::default().title("BOMB TIMER").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Red))
            .ratio(timer_ratio)
            .label(format!("{}s", bomb_time));
        frame.render_widget(bomb_gauge, chunks[0]);

        // Info list
        let info_items: Vec<ListItem> = vec![
            format!("T: {} | CT: {}", self.terrorist_score, self.counter_terrorist_score),
            format!("Round: {}/{}", self.round_number, self.max_rounds),
            "BOMB PLANTED!".to_string(),
        ]
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

    fn render_round_ended(&self, frame: &mut Frame, area: Rect) {
        let (winner_msg, winner_color) = if self.is_match_over() {
            if self.terrorist_score > self.counter_terrorist_score {
                ("TERRORISTS WIN MATCH!", Color::Red)
            } else {
                ("CT WIN MATCH!", Color::Blue)
            }
        } else {
            ("Round Complete", Color::Green)
        };

        let info_items: Vec<ListItem> = vec![
            format!("T: {} | CT: {}", self.terrorist_score, self.counter_terrorist_score),
            format!("Round: {}/{}", self.round_number, self.max_rounds),
            winner_msg.to_string(),
        ]
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

        let instructions = if self.is_match_over() {
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
}

impl View for SearchAndDestroyView {
    fn handle_input(&mut self, event: InputEvent, task_senders: &TaskSenders) -> Option<NavigationAction> {
        match event {
            InputEvent::KeypadEvent(key) => {
                match self.phase {
                    SearchAndDestroyPhase::Configuration => {
                        match key {
                            'a' | 'A' => { // Move up
                                self.move_config_selection_up();
                                None
                            },
                            'd' | 'D' => { // Move down
                                self.move_config_selection_down();
                                None
                            },
                            'b' | 'B' => { // Adjust value (will cycle through + and -)
                                // For now, just increase. We can add decrease with different key later
                                self.adjust_config_value(true);
                                None
                            },
                            '1' => { // Start game
                                self.apply_configuration();
                                None
                            },
                            '0' => { // Back to main menu
                                Some(NavigationAction::Back)
                            },
                            _ => None,
                        }
                    },
                    SearchAndDestroyPhase::WaitingToStart => {
                        match key {
                            '1' => { // Start round
                                self.start_round(task_senders);
                                None
                            },
                            '0' => { // Back to main menu
                                Some(NavigationAction::Back)
                            },
                            _ => None,
                        }
                    },
                    SearchAndDestroyPhase::RoundActive => {
                        match key {
                            '3' => { // Plant bomb
                                self.plant_bomb(task_senders);
                                None
                            },
                            '9' => { // Terrorists win round
                                self.end_round(true, task_senders);
                                None
                            },
                            '6' => { // Counter-terrorists win round
                                self.end_round(false, task_senders);
                                None
                            },
                            '0' => { // Back to main menu
                                Some(NavigationAction::Back)
                            },
                            _ => None,
                        }
                    },
                    SearchAndDestroyPhase::BombPlanted => {
                        match key {
                            '7' => { // Defuse bomb
                                self.defuse_bomb(task_senders);
                                None
                            },
                            '9' => { // Terrorists win round
                                self.end_round(true, task_senders);
                                None
                            },
                            '6' => { // Counter-terrorists win round
                                self.end_round(false, task_senders);
                                None
                            },
                            '0' => { // Back to main menu
                                Some(NavigationAction::Back)
                            },
                            _ => None,
                        }
                    },
                    SearchAndDestroyPhase::RoundEnded => {
                        match key {
                            '1' => { // Next round or restart
                                if self.is_match_over() {
                                    self.reset_game();
                                    self.phase = SearchAndDestroyPhase::Configuration;
                                } else {
                                    self.next_round_or_end();
                                }
                                None
                            },
                            '0' => { // Back to main menu
                                Some(NavigationAction::Back)
                            },
                            _ => None,
                        }
                    },
                }
            },
            InputEvent::CardDetected => {
                // NFC could be used for bomb planting/defusing
                match self.phase {
                    SearchAndDestroyPhase::RoundActive => {
                        self.plant_bomb(task_senders);
                    },
                    SearchAndDestroyPhase::BombPlanted => {
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
        match self.phase {
            SearchAndDestroyPhase::Configuration => self.render_configuration(frame, area),
            SearchAndDestroyPhase::WaitingToStart => self.render_waiting(frame, area),
            SearchAndDestroyPhase::RoundActive => self.render_round_active(frame, area),
            SearchAndDestroyPhase::BombPlanted => self.render_bomb_planted(frame, area),
            SearchAndDestroyPhase::RoundEnded => self.render_round_ended(frame, area),
        }
    }
    
    fn on_enter(&mut self, task_senders: &TaskSenders) {
        task_senders.sound.try_send(SoundCommand::Beep).ok();
    }
    
    fn view_type(&self) -> ViewType {
        ViewType::SearchAndDestroy
    }
}
