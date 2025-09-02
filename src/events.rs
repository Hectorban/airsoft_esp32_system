extern crate alloc;

use crate::tasks::output::{lights, sound};
use crate::tasks::rng::RngRequest;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use alloc::boxed::Box;
use ector::Address;

pub const EVENT_QUEUE_SIZE: usize = 32;
pub const CLOCK_QUEUE_SIZE: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEvent {
    KeypadEvent(char),
    CardDetected,
    GameTick,
    None
}

pub type EventChannel = Channel<NoopRawMutex, InputEvent, { EVENT_QUEUE_SIZE }>;

pub enum Command {
    Lights(lights::LightsCommand),
    Sound(sound::SoundCommand),
    Noop
}

pub struct TaskSenders {
    pub lights: Address<lights::LightsCommand>,
    pub sound: Address<sound::SoundCommand>,
    pub rng: Address<RngRequest>,
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
