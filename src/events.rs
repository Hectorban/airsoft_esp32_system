extern crate alloc;

use crate::app::App;
use crate::tasks::output::{lights, sound};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use alloc::boxed::Box;

pub const EVENT_QUEUE_SIZE: usize = 32;
pub const CLOCK_QUEUE_SIZE: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEvent {
    Number1,
    Number2,
    Number3,
    Number4,
    Number5,
    Number6,
    Number7,
    Number8,
    Number9,
    Number0,
    Hashtag,
    Asterisk,
    LetterA,
    LetterB,
    LetterC,
    LetterD,
    CardDetected,
    GameTick,
    None
}

impl InputEvent {
    pub fn from_char(c: char) -> Option<InputEvent> {
        match c {
            '1' => Some(InputEvent::Number1),
            '2' => Some(InputEvent::Number2),
            '3' => Some(InputEvent::Number3),
            '4' => Some(InputEvent::Number4),
            '5' => Some(InputEvent::Number5),
            '6' => Some(InputEvent::Number6),
            '7' => Some(InputEvent::Number7),
            '8' => Some(InputEvent::Number8),
            '9' => Some(InputEvent::Number9),
            '0' => Some(InputEvent::Number0),
            '#' => Some(InputEvent::Hashtag),
            '*' => Some(InputEvent::Asterisk),
            'A' => Some(InputEvent::LetterA),
            'B' => Some(InputEvent::LetterB),
            'C' => Some(InputEvent::LetterC),
            'D' => Some(InputEvent::LetterD),
            _ => None,
        }
    }
    
    pub fn from_str(s: &str) -> Option<InputEvent> {
        s.chars().next().and_then(InputEvent::from_char)
    }

    pub fn to_str(&self) -> &str {
        match self {
            InputEvent::Number1 => "1",
            InputEvent::Number2 => "2",
            InputEvent::Number3 => "3",
            InputEvent::Number4 => "4",
            InputEvent::Number5 => "5",
            InputEvent::Number6 => "6",
            InputEvent::Number7 => "7",
            InputEvent::Number8 => "8",
            InputEvent::Number9 => "9",
            InputEvent::Number0 => "0",
            InputEvent::Hashtag => "#",
            InputEvent::Asterisk => "*",
            InputEvent::LetterA => "A",
            InputEvent::LetterB => "B",
            InputEvent::LetterC => "C",
            InputEvent::LetterD => "D",
            InputEvent::CardDetected => "CardDetected",
            InputEvent::GameTick => "GameTick",
            InputEvent::None => "None",
        }
    }
}

pub type EventChannel = Channel<NoopRawMutex, InputEvent, { EVENT_QUEUE_SIZE }>;

pub enum Command {
    Lights(lights::LightsCommand),
    Sound(sound::SoundCommand),
    Noop
}

pub struct TaskSenders {
    pub lights: Sender<'static, NoopRawMutex, lights::LightsCommand, { lights::LIGHTS_QUEUE_SIZE }>,
    pub sound: Sender<'static, NoopRawMutex, sound::SoundCommand, { sound::SOUND_QUEUE_SIZE }>,
}

pub struct EventBus {
    pub event_sender: Sender<'static, NoopRawMutex, InputEvent, { EVENT_QUEUE_SIZE }>,
    pub event_receiver: Receiver<'static, NoopRawMutex, InputEvent, { EVENT_QUEUE_SIZE }>,
}

impl EventBus {
    pub fn new(event_channel: &'static EventChannel) -> Self {
        let event_sender = event_channel.sender();
        let event_receiver = event_channel.receiver();

        Self {
            event_sender,
            event_receiver,
        }
    }
}
