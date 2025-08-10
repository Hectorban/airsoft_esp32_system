use crate::{app::{search_and_destroy::SearchAndDestroy, App}, events::{Command, InputEvent}, tasks::output::{display::DisplayCommand, lights::LightsCommand, sound::SoundCommand}};
use alloc::vec::Vec;
use alloc::string::String;
use alloc::{vec, boxed::Box};

extern crate alloc;

pub struct MainMenu {
    current_selection: MainMenuSelection,
    has_selected: bool,
    last_event: InputEvent,
}

enum MainMenuSelection {
    SearchAndDestroy,
    TeamDeathMatch,
    Domination,
    Cashout,
    Config,
}   

impl MainMenuSelection {
    fn next(&mut self) {
        match self {
            MainMenuSelection::SearchAndDestroy => *self = MainMenuSelection::TeamDeathMatch,
            MainMenuSelection::TeamDeathMatch => *self = MainMenuSelection::Domination,
            MainMenuSelection::Domination => *self = MainMenuSelection::Cashout,
            MainMenuSelection::Cashout => *self = MainMenuSelection::Config,
            MainMenuSelection::Config => *self = MainMenuSelection::SearchAndDestroy,
        }
    }

    fn prev(&mut self) {
        match self {
            MainMenuSelection::Config => *self = MainMenuSelection::Cashout,
            MainMenuSelection::Cashout => *self = MainMenuSelection::Domination,
            MainMenuSelection::Domination => *self = MainMenuSelection::TeamDeathMatch,
            MainMenuSelection::TeamDeathMatch => *self = MainMenuSelection::SearchAndDestroy,
            MainMenuSelection::SearchAndDestroy => *self = MainMenuSelection::Config,
        }
    }

    fn handle_select_render(&mut self) -> Command {
        match self {
            MainMenuSelection::Config => Command::DisplayText(DisplayCommand::WriteText {
                        line1: String::from("Not implemented"),
                        line2: String::from("")
                    }),
            MainMenuSelection::Cashout => Command::DisplayText(DisplayCommand::WriteText {
                        line1: String::from("Not implemented"),
                        line2: String::from("")
                    }),
            MainMenuSelection::Domination => Command::DisplayText(DisplayCommand::WriteText {
                        line1: String::from("Not implemented"),
                        line2: String::from("")
                    }),
            MainMenuSelection::TeamDeathMatch => Command::DisplayText(DisplayCommand::WriteText {
                        line1: String::from("Not implemented"),
                        line2: String::from("")
                    }),
            MainMenuSelection::SearchAndDestroy => Command::ChangeApp(Box::new(SearchAndDestroy::default())),
        }
    }
}

impl MainMenu {
    fn valid_last_event(&mut self) -> bool {
        match self.last_event {
            InputEvent::LetterA => true,
            InputEvent::LetterB => true,
            InputEvent::Number4 => true,
            _ => false,
        }
    }
}

impl MainMenu {
    fn handle_menu_up(&mut self) {
        self.current_selection.prev();
    }

    fn handle_menu_down(&mut self) {
        self.current_selection.next();
    }
}

impl Default for MainMenu {
    fn default() -> Self {
        Self {
            current_selection: MainMenuSelection::SearchAndDestroy,
            last_event: InputEvent::None,
            has_selected: false,
        }
    }
}

impl App for MainMenu {
    fn handle_event(&mut self, event: InputEvent) {
        self.last_event = event;
        match event {
            InputEvent::LetterA => self.handle_menu_up(),
            InputEvent::LetterB => self.handle_menu_down(),
            InputEvent::Number4 => self.has_selected = true,
            _ => {}
        }
    }

    fn render(&mut self) -> Vec<Command> {
        let mut commands = vec![];

        if self.has_selected {
            commands.push(self.current_selection.handle_select_render());
        } else {
            match self.current_selection {
                MainMenuSelection::SearchAndDestroy => {
                    commands.push(Command::DisplayText(DisplayCommand::WriteText {
                        line1: String::from("Search&Destroy <-"),
                        line2: String::from("TDMatch")
                    }));
                }
                MainMenuSelection::TeamDeathMatch => {
                    commands.push(Command::DisplayText(DisplayCommand::WriteText {
                        line1: String::from("TDMatch <-"),
                        line2: String::from("Domination")
                    }));
                }
                MainMenuSelection::Domination => {
                    commands.push(Command::DisplayText(DisplayCommand::WriteText {
                        line1: String::from("Domination <-"),
                        line2: String::from("Cashout")
                    }));
                }
                MainMenuSelection::Cashout => {
                    commands.push(Command::DisplayText(DisplayCommand::WriteText {
                        line1: String::from("Cashout <-"),
                        line2: String::from("Config")
                    }));
                }
                MainMenuSelection::Config => {
                    commands.push(Command::DisplayText(DisplayCommand::WriteText {
                        line1: String::from("Config <-"),
                        line2: String::from("")
                    }));
                }
            }   
        }

        // Feedback
        if self.valid_last_event() && self.last_event != InputEvent::GameTick {
            commands.push(Command::Sound(SoundCommand::SuccessBeep));

            commands.push(Command::Lights(LightsCommand::Flash {
                r: 255,
                g: 255,
                b: 255,
                duration_ms: 50,
            }));
        } else if self.last_event != InputEvent::GameTick {
            commands.push(Command::Sound(SoundCommand::ErrorBeep));
            commands.push(Command::Lights(LightsCommand::Flash {
                r: 255,
                g: 0,
                b: 0,
                duration_ms: 50,
            }));
        }

        commands
    }
}
    
