use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use crate::tasks::{DisplayCommand, LightsCommand, SoundCommand};

pub const EVENT_QUEUE_SIZE: usize = 32;

#[derive(Debug, Clone, Copy)]
pub enum GameEvent {
    KeypadPress(char),
    NfcCardDetected,
    TimerTick,
    MenuUp,
    MenuDown,
    MenuSelect,
    GameArm,
    GameDisarm,
    CodeDigit(u8),
    GameTimeout,
}

pub type EventChannel = Channel<NoopRawMutex, GameEvent, { EVENT_QUEUE_SIZE }>;

pub struct TaskSenders {
    pub display: Sender<'static, NoopRawMutex, DisplayCommand, { crate::tasks::display::DISPLAY_QUEUE_SIZE }>,
    pub lights: Sender<'static, NoopRawMutex, LightsCommand, { crate::tasks::lights::LIGHTS_QUEUE_SIZE }>,
    pub sound: Sender<'static, NoopRawMutex, SoundCommand, { crate::tasks::sound::SOUND_QUEUE_SIZE }>,
}

pub struct EventBus {
    pub event_sender: Sender<'static, NoopRawMutex, GameEvent, { EVENT_QUEUE_SIZE }>,
    pub event_receiver: Receiver<'static, NoopRawMutex, GameEvent, { EVENT_QUEUE_SIZE }>,
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
