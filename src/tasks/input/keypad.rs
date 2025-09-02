use ector::{Actor, ActorAddress, DynamicAddress, Inbox};
use defmt::info;
use embassy_time::{Duration, Timer};

use crate::{devices::keypad, events::InputEvent};

pub struct KeypadActor {
    keypad: keypad::I2cKeypad,
    app_address: DynamicAddress<InputEvent>,
}

impl KeypadActor {
    pub fn new(
        keypad: keypad::I2cKeypad,
        app_address: DynamicAddress<InputEvent>,
    ) -> Self {
        Self {
            keypad,
            app_address,
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
                self.app_address.notify(InputEvent::KeypadEvent(key)).await;
            }
            Timer::after(Duration::from_millis(50)).await;
        }
    }
}
