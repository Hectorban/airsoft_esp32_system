use crate::tasks::output::{lights, sound};
use crate::tasks::rng::RngRequest;
use alloc::vec::Vec;
use ector::mutex::NoopRawMutex as EctorNoopRawMutex;
use ector::Address;

/// Input events sent directly to the App actor
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    KeypadEvent(char),
    CardDetected(Vec<u8>),
    GameTick,
    None,
}

/// Collection of actor addresses for sending messages to output tasks
pub struct TaskSenders {
    pub lights: Address<lights::LightsCommand, EctorNoopRawMutex>,
    pub sound: Address<sound::SoundCommand, EctorNoopRawMutex>,
    pub rng: Address<RngRequest, EctorNoopRawMutex>, // Keep Address for request-response
}
