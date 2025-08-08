use crate::events::{GameEvent, TaskSenders};
use crate::tasks::{LightsCommand, SoundCommand};
use esp_hal::rng::Rng;
use defmt::info;

extern crate alloc;
use alloc::string::String;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameState {
    MainMenu,
    SearchAndDestroy(SearchAndDestroyState),
    Configuration,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchAndDestroyState {
    Idle,
    ArmingCodeEntry { digits_entered: u8, code: [u8; 4] },
    Armed { time_left_seconds: u32, code: [u8; 4] },
    DisarmingCodeEntry { digits_entered: u8, code: [u8; 4], entered_code: [u8; 4] },
    Defused,
    Exploded,
}

#[derive(Debug, Clone, Copy)]
pub struct MenuState {
    pub selected_index: usize,
    pub items: &'static [&'static str],
}

impl MenuState {
    pub fn new(items: &'static [&'static str]) -> Self {
        Self {
            selected_index: 0,
            items,
        }
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected_index < self.items.len() - 1 {
            self.selected_index += 1;
        }
    }

    pub fn get_selected(&self) -> &'static str {
        self.items[self.selected_index]
    }

    pub fn can_move_up(&self) -> bool {
        self.selected_index > 0
    }

    pub fn can_move_down(&self) -> bool {
        self.selected_index < self.items.len() - 1
    }
}

const MAIN_MENU_ITEMS: &[&str] = &["Search & Destroy", "Configuration"];
const BOMB_TIMER_SECONDS: u32 = 120; // 2 minutes

pub struct GameManager {
    pub state: GameState,
    pub menu: MenuState,
    rng: Rng,
}

impl GameManager {
    pub fn new(rng: Rng) -> Self {
        Self {
            state: GameState::MainMenu,
            menu: MenuState::new(MAIN_MENU_ITEMS),
            rng,
        }
    }

    pub async fn handle_event(
        &mut self,
        event: GameEvent,
        task_senders: &TaskSenders,
    ) {
        match self.state {
            GameState::MainMenu => self.handle_main_menu_event(event, task_senders).await,
            GameState::SearchAndDestroy(sd_state) => {
                self.handle_search_destroy_event(event, sd_state, task_senders).await
            }
            GameState::Configuration => self.handle_config_event(event, task_senders).await,
        }
    }

    async fn handle_main_menu_event(
        &mut self,
        event: GameEvent,
        task_senders: &TaskSenders,
    ) {
        match event {
            GameEvent::MenuUp => {
                self.menu.move_up();
                self.render_main_menu(task_senders).await;
                // Feedback
                let _ = task_senders.lights.send(LightsCommand::SetStripColors {
                    strip1: (0, 0, 50),
                    strip2: (0, 0, 0),
                }).await;
                let _ = task_senders.sound.send(SoundCommand::PlayTone {
                    frequency: 800,
                    duration_ms: 100,
                }).await;
            }
            GameEvent::MenuDown => {
                self.menu.move_down();
                self.render_main_menu(task_senders).await;
                // Feedback
                let _ = task_senders.lights.send(LightsCommand::SetStripColors {
                    strip1: (0, 0, 0),
                    strip2: (0, 0, 50),
                }).await;
                let _ = task_senders.sound.send(SoundCommand::PlayTone {
                    frequency: 600,
                    duration_ms: 100,
                }).await;
            }
            GameEvent::MenuSelect => {
                match self.menu.get_selected() {
                    "Search & Destroy" => {
                        self.state = GameState::SearchAndDestroy(SearchAndDestroyState::Idle);
                        self.render_search_destroy(SearchAndDestroyState::Idle, task_senders).await;
                    }
                    "Configuration" => {
                        self.state = GameState::Configuration;
                        self.render_config(task_senders).await;
                    }
                    _ => {}
                }
                // Selection feedback
                let _ = task_senders.lights.send(LightsCommand::SetBothColors {
                    r: 0, g: 50, b: 0,
                }).await;
                let _ = task_senders.sound.send(SoundCommand::PlayTone {
                    frequency: 1000,
                    duration_ms: 200,
                }).await;
            }
            _ => {}
        }
    }

    async fn handle_search_destroy_event(
        &mut self,
        event: GameEvent,
        current_state: SearchAndDestroyState,
        task_senders: &TaskSenders,
    ) {
        match (current_state, event) {
            (SearchAndDestroyState::Idle, GameEvent::GameArm) => {
                let code = self.generate_random_code();
                let new_state = SearchAndDestroyState::ArmingCodeEntry {
                    digits_entered: 0,
                    code,
                };
                self.state = GameState::SearchAndDestroy(new_state);
                self.render_search_destroy(new_state, task_senders).await;
            }
            (SearchAndDestroyState::ArmingCodeEntry { digits_entered, code }, GameEvent::CodeDigit(_digit)) => {
                if digits_entered < 4 {
                    let new_digits = digits_entered + 1;
                    if new_digits == 4 {
                        // Check if code matches
                        // For simplicity, we'll assume the entered code is correct for now
                        let armed_state = SearchAndDestroyState::Armed {
                            time_left_seconds: BOMB_TIMER_SECONDS,
                            code,
                        };
                        self.state = GameState::SearchAndDestroy(armed_state);
                        self.render_search_destroy(armed_state, task_senders).await;
                        info!("Bomb armed! Timer started.");
                    } else {
                        let new_state = SearchAndDestroyState::ArmingCodeEntry {
                            digits_entered: new_digits,
                            code,
                        };
                        self.state = GameState::SearchAndDestroy(new_state);
                        self.render_search_destroy(new_state, task_senders).await;
                    }
                }
            }
            (SearchAndDestroyState::Armed { time_left_seconds: _, code }, GameEvent::GameDisarm) => {
                let new_state = SearchAndDestroyState::DisarmingCodeEntry {
                    digits_entered: 0,
                    code,
                    entered_code: [0; 4],
                };
                self.state = GameState::SearchAndDestroy(new_state);
                self.render_search_destroy(new_state, task_senders).await;
            }
            (SearchAndDestroyState::Armed { time_left_seconds, code }, GameEvent::TimerTick) => {
                if time_left_seconds > 0 {
                    let new_time = time_left_seconds - 1;
                    let new_state = SearchAndDestroyState::Armed {
                        time_left_seconds: new_time,
                        code,
                    };
                    self.state = GameState::SearchAndDestroy(new_state);
                    self.render_search_destroy(new_state, task_senders).await;
                    
                    if new_time == 0 {
                        let exploded_state = SearchAndDestroyState::Exploded;
                        self.state = GameState::SearchAndDestroy(exploded_state);
                        self.render_search_destroy(exploded_state, task_senders).await;
                    }
                }
            }
            (SearchAndDestroyState::DisarmingCodeEntry { digits_entered, code, mut entered_code }, GameEvent::CodeDigit(digit)) => {
                if digits_entered < 4 {
                    entered_code[digits_entered as usize] = digit;
                    let new_digits = digits_entered + 1;
                    
                    if new_digits == 4 {
                        // Check if entered code matches the original code
                        if entered_code == code {
                            let defused_state = SearchAndDestroyState::Defused;
                            self.state = GameState::SearchAndDestroy(defused_state);
                            self.render_search_destroy(defused_state, task_senders).await;
                            info!("Bomb defused!");
                        } else {
                            // Wrong code, back to armed state
                            let armed_state = SearchAndDestroyState::Armed {
                                time_left_seconds: BOMB_TIMER_SECONDS, // Reset or keep current time
                                code,
                            };
                            self.state = GameState::SearchAndDestroy(armed_state);
                            self.render_search_destroy(armed_state, task_senders).await;
                        }
                    } else {
                        let new_state = SearchAndDestroyState::DisarmingCodeEntry {
                            digits_entered: new_digits,
                            code,
                            entered_code,
                        };
                        self.state = GameState::SearchAndDestroy(new_state);
                        self.render_search_destroy(new_state, task_senders).await;
                    }
                }
            }
            // Back to main menu from any completed state
            (SearchAndDestroyState::Defused | SearchAndDestroyState::Exploded, GameEvent::MenuSelect) => {
                self.state = GameState::MainMenu;
                self.render_main_menu(task_senders).await;
            }
            _ => {}
        }
    }

    async fn handle_config_event(
        &mut self,
        event: GameEvent,
        task_senders: &TaskSenders,
    ) {
        if let GameEvent::MenuSelect = event {
            // Back to main menu for now
            self.state = GameState::MainMenu;
            self.render_main_menu(task_senders).await;
        }
    }

    async fn render_main_menu(
        &self,
        task_senders: &TaskSenders,
    ) {
        let line1 = String::from("Airsoft Master");
        let selected = self.menu.get_selected();
        let mut line2 = String::new();
        
        // Add navigation arrows if needed
        if self.menu.can_move_up() {
            line2.push('↑');
        } else {
            line2.push(' ');
        }
        
        line2.push_str(selected);
        
        if self.menu.can_move_down() {
            line2.push('↓');
        }
        
        let _ = task_senders.display.send(crate::tasks::DisplayCommand::WriteText { line1, line2 }).await;
    }

    async fn render_search_destroy(
        &self,
        state: SearchAndDestroyState,
        task_senders: &TaskSenders,
    ) {
        let (line1, line2, led1, led2) = match state {
            SearchAndDestroyState::Idle => (
                String::from("Search&Destroy"),
                String::from("Press A to arm"),
                (0, 0, 0),
                (0, 0, 0),
            ),
            SearchAndDestroyState::ArmingCodeEntry { digits_entered, .. } => {
                let mut line2 = String::from("Code: ");
                for i in 0..4 {
                    if i < digits_entered {
                        line2.push('*');
                    } else {
                        line2.push('_');
                    }
                }
                (
                    String::from("Enter arm code"),
                    line2,
                    (50, 50, 0), // Yellow
                    (50, 50, 0),
                )
            }
            SearchAndDestroyState::Armed { time_left_seconds, .. } => {
                let minutes = time_left_seconds / 60;
                let seconds = time_left_seconds % 60;
                let mut line2 = String::new();
                let _ = write!(line2, "{minutes}:{seconds:02} Press B");
                (
                    String::from("BOMB ARMED!"),
                    line2,
                    (50, 0, 0), // Red
                    (50, 0, 0),
                )
            }
            SearchAndDestroyState::DisarmingCodeEntry { digits_entered, .. } => {
                let mut line2 = String::from("Code: ");
                for i in 0..4 {
                    if i < digits_entered {
                        line2.push('*');
                    } else {
                        line2.push('_');
                    }
                }
                (
                    String::from("Disarm code"),
                    line2,
                    (0, 0, 50), // Blue
                    (0, 0, 50),
                )
            }
            SearchAndDestroyState::Defused => (
                String::from("BOMB DEFUSED!"),
                String::from("Press 4 to menu"),
                (0, 50, 0), // Green
                (0, 50, 0),
            ),
            SearchAndDestroyState::Exploded => (
                String::from("BOOM! EXPLODED"),
                String::from("Press 4 to menu"),
                (50, 0, 0), // Red flashing effect
                (50, 0, 0),
            ),
        };

        let _ = task_senders.display.send(crate::tasks::DisplayCommand::WriteText { line1, line2 }).await;
        let _ = task_senders.lights.send(crate::tasks::LightsCommand::SetStripColors {
            strip1: led1,
            strip2: led2,
        }).await;
    }

    async fn render_config(
        &self,
        task_senders: &TaskSenders,
    ) {
        let line1 = String::from("Configuration");
        let line2 = String::from("Press 4 to back");
        let _ = task_senders.display.send(crate::tasks::DisplayCommand::WriteText { line1, line2 }).await;
    }

    fn generate_random_code(&mut self) -> [u8; 4] {
        let mut code = [0u8; 4];
        for i in 0..4 {
            code[i] = (self.rng.random() % 10) as u8;
        }
        info!("Generated code: {:?}", code);
        code
    }
}

// Helper for formatting
use core::fmt::Write;