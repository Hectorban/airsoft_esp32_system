use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Sender};
use defmt::info;
use embassy_time::{Duration, Timer};

use crate::{devices::keypad, events::{InputEvent, EVENT_QUEUE_SIZE}};


#[embassy_executor::task]
pub async fn keypad_task(
    mut keypad: keypad::I2cKeypad,
    event_sender: Sender<'static, NoopRawMutex, InputEvent, {EVENT_QUEUE_SIZE}>,
) {
    loop {
        if let Some(key) = keypad.scan() {
            info!("Key pressed: {}", key);
            let _ = event_sender.send(InputEvent::KeypadEvent(key)).await;
        }
        Timer::after(Duration::from_millis(50)).await;
    };
}
