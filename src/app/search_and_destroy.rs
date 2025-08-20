extern crate alloc;

use crate::app::main_menu::MainMenu;
use crate::app::App;
use crate::events::{Command, InputEvent};
use crate::tasks::output::display::DisplayCommand;
use crate::tasks::output::lights::LightsCommand;
use crate::tasks::output::sound::SoundCommand;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::boxed::Box;
use alloc::vec::Vec;
use defmt::info;

pub struct SearchAndDestroy {
    pub time_left: u32,
    code: String,
    pub stage: Stage,
    pub current_code: String,
    pub wants_game_tick: bool,
    last_event: InputEvent,
}

pub enum Stage {
    WaitingForArm,
    Arming,
    Armed,
    Ticking,
    Exploded,
    Disarming,
    Disarmed,
}

impl Default for SearchAndDestroy {
    fn default() -> Self {
        Self {
            time_left: 600, // 2 minutes
            code: String::from("1234"), // TODO: Randomize
            stage: Stage::WaitingForArm,
            current_code: String::new(),
            wants_game_tick: false,
            last_event: InputEvent::None,
        }
    }
}

impl App for SearchAndDestroy {
    fn handle_event(&mut self, event: InputEvent) {
        self.last_event = event;

        match self.stage {
            Stage::WaitingForArm => {
                if let InputEvent::LetterA = event {
                    self.stage = Stage::Arming;
                }
            }
            Stage::Arming => {
                if event != InputEvent::GameTick {
                    if self.current_code.len() < 4 {
                        if let Some(digit) = event.to_str().chars().next() {
                             self.current_code.push(digit);
                        }
                    }

                    if self.current_code.len() == 4 {
                        if self.current_code == self.code {
                            self.stage = Stage::Armed;
                            self.wants_game_tick = true;
                        } else {
                            self.current_code.clear(); // Incorrect code
                        }
                    }
                }
            }
            Stage::Armed => {
                self.stage = Stage::Ticking;
            },
                
            Stage::Ticking => {
                match event {
                    InputEvent::GameTick => {
                        if self.time_left > 0 {
                            self.time_left -= 1;
                        } else {
                            self.stage = Stage::Exploded;
                        }
                    }
                    InputEvent::LetterB => {
                        self.current_code.clear();
                        self.stage = Stage::Disarming;
                    }
                    _ => {}
                }
            }
            Stage::Disarming => {
                if event == InputEvent::CardDetected {
                    self.stage = Stage::Disarmed;
                }

                if event != InputEvent::GameTick {
                    if self.current_code.len() < 4 {
                        if let Some(digit) = event.to_str().chars().next() {
                             self.current_code.push(digit);
                        }
                    }
                }

                if self.current_code.len() == 4 {
                    if self.current_code == self.code {
                        self.stage = Stage::Disarmed;
                        self.wants_game_tick = false;
                    } else {
                        self.current_code.clear(); // Incorrect code
                    }
                }
            }
            _ => {
                // No events handled in other states yet
            }
        }
    }

    fn render(&mut self) -> Vec<Command> {
        let mut commands = vec![];

        match self.stage {
            Stage::WaitingForArm => {
                commands.push(Command::DisplayText(DisplayCommand::WriteText {
                    line1: "Search & Destroy".into(),
                    line2: "Press A to arm".into(),
                }));
            }
            Stage::Arming => {
                commands.push(Command::DisplayText(DisplayCommand::WriteText {
                    line1: "Enter arm code:".into(),
                    line2: format!("{}", self.current_code),
                }));
                
                // TODO merge display and sound
                if self.last_event != InputEvent::GameTick {
                    commands.push(Command::Sound(SoundCommand::Beep));
                }
            }
            Stage::Armed => {
                commands.push(Command::DisplayText(DisplayCommand::WriteText {
                    line1: "Armed!".into(),
                    line2: "".into(),
                }));
                commands.push(Command::Sound(SoundCommand::DoubleBeep));
                commands.push(Command::Lights(LightsCommand::Flash {
                    r: 255,
                    g: 0,
                    b: 0,
                    duration_ms: 50,
                }));
            }
            Stage::Ticking => {
                commands.push(Command::DisplayText(DisplayCommand::WriteText { 
                    line1: format!("Time: {:02}:{:02}", self.time_left / 60, self.time_left % 60), 
                    line2: String::from("Press B to disarm") 
                }));
                commands.push(Command::Sound(SoundCommand::Beep));

                commands.push(Command::Lights(LightsCommand::Flash {
                    r: 255,
                    g: 0,
                    b: 0,
                    duration_ms: 50,
                }));
            }
            Stage::Disarming => {
                commands.push(Command::DisplayText(DisplayCommand::WriteText {
                    line1: "Enter disarm code:".into(),
                    line2: format!("{}", self.current_code),
                }));

                if self.last_event != InputEvent::GameTick {
                    commands.push(Command::Sound(SoundCommand::Beep));
                }
            }
            Stage::Disarmed => {
                commands.push(Command::DisplayText(DisplayCommand::WriteText {
                    line1: "Bomb has been".into(),
                    line2: "defused!".into(),
                }));

                commands.push(Command::Sound(SoundCommand::VictorySound));
                commands.push(Command::ChangeApp(Box::new(MainMenu::default())))
            }
            Stage::Exploded => {
                commands.push(Command::DisplayText(DisplayCommand::WriteText {
                    line1: "Bomb has exploded".into(),
                    line2: "Game Over".into(),
                }));

                commands.push(Command::Sound(SoundCommand::DefeatSound));
                commands.push(Command::ChangeApp(Box::new(MainMenu::default())))
            }
        }

        commands
    }
    
    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
}
