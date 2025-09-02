use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Sender};
use ector::{Actor, DynamicAddress, Inbox};
use defmt::info;
use embassy_time::{Duration, Timer};

use crate::{devices::keypad, events::{InputEvent, EVENT_QUEUE_SIZE}};

pub struct KeypadActor {
    keypad: keypad::I2cKeypad,
    event_sender: Sender<'static, NoopRawMutex, InputEvent, { EVENT_QUEUE_SIZE }>,
}

impl KeypadActor {
    pub fn new(
        keypad: keypad::I2cKeypad,
        event_sender: Sender<'static, NoopRawMutex, InputEvent, { EVENT_QUEUE_SIZE }>,
    ) -> Self {
        Self {
            keypad,
            event_sender,
        }
    }
}

impl Actor for KeypadActor {
    type Message = !;

    async fn on_mount<M>(&mut self, _: DynamicAddress<Self::Message>, _inbox: M) -> !
    where
        M: Inbox<Self::Message>,
    {
        loop {
            if let Some(key) = self.keypad.scan() {
                info!("Key pressed: {}", key);
                let _ = self.event_sender.send(InputEvent::KeypadEvent(key)).await;
            }
            Timer::after(Duration::from_millis(50)).await;
        }
    }
}
