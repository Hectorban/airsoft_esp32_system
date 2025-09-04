use ratatui::{
    Frame,
    layout::{Rect},
};

use crate::{
    events::{InputEvent, TaskSenders},
    tasks::output::sound::SoundCommand,
};

use super::{View, ViewType, NavigationAction};

mod configuration;
mod waiting;
mod active;
mod planted;
mod ended;

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
    pub(super) phase: SearchAndDestroyPhase,
    pub(super) game_state: SearchAndDestroyGameState,
    pub(super) round_number: u32,
    pub(super) terrorist_score: u32,
    pub(super) counter_terrorist_score: u32,
    pub(super) round_time_remaining: u32, // in seconds
    pub(super) bomb_time_remaining: Option<u32>, // Some when bomb is planted
    pub(super) max_rounds: u32,
    // Configuration options
    pub(super) selected_config_index: usize,
    pub(super) round_time_minutes: u32,
    pub(super) bomb_timer_seconds: u32,
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

    pub(super) fn start_configuration(&mut self) {
        self.phase = SearchAndDestroyPhase::Configuration;
        self.selected_config_index = 0;
    }

    pub(super) fn apply_configuration(&mut self) {
        self.round_time_remaining = self.round_time_minutes * 60;
        self.phase = SearchAndDestroyPhase::WaitingToStart;
    }

    pub(super) fn move_config_selection_up(&mut self) {
        if self.selected_config_index > 0 {
            self.selected_config_index -= 1;
        } else {
            self.selected_config_index = 2; // 3 config options: 0, 1, 2
        }
    }
    
    pub(super) fn move_config_selection_down(&mut self) {
        if self.selected_config_index < 2 {
            self.selected_config_index += 1;
        } else {
            self.selected_config_index = 0;
        }
    }

    pub(super) fn adjust_config_value(&mut self, increase: bool) {
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
    
    pub(super) fn start_round(&mut self, task_senders: &TaskSenders) {
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
    
    pub(super) fn is_match_over(&self) -> bool {
        let half_rounds = self.max_rounds / 2;
        self.terrorist_score > half_rounds || self.counter_terrorist_score > half_rounds
    }
}

impl View for SearchAndDestroyView {
    fn handle_input(&mut self, event: InputEvent, task_senders: &TaskSenders) -> Option<NavigationAction> {
        match self.phase {
            SearchAndDestroyPhase::Configuration => configuration::handle_input(self, event),
            SearchAndDestroyPhase::WaitingToStart => waiting::handle_input(self, event, task_senders),
            SearchAndDestroyPhase::RoundActive => active::handle_input(self, event, task_senders),
            SearchAndDestroyPhase::BombPlanted => planted::handle_input(self, event, task_senders),
            SearchAndDestroyPhase::RoundEnded => ended::handle_input(self, event),
        }
    }
    
    fn render(&self, frame: &mut Frame, area: Rect) {
        match self.phase {
            SearchAndDestroyPhase::Configuration => configuration::render(self, frame, area),
            SearchAndDestroyPhase::WaitingToStart => waiting::render(self, frame, area),
            SearchAndDestroyPhase::RoundActive => active::render(self, frame, area),
            SearchAndDestroyPhase::BombPlanted => planted::render(self, frame, area),
            SearchAndDestroyPhase::RoundEnded => ended::render(self, frame, area),
        }
    }
    
    fn on_enter(&mut self, task_senders: &TaskSenders) {
        task_senders.sound.try_send(SoundCommand::Beep).ok();
    }
    
    fn view_type(&self) -> ViewType {
        ViewType::SearchAndDestroy
    }
}
