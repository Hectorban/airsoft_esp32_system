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
        if let Some(key) = keypad.scan().await {
            info!("Key pressed: {}", key);
            let event = match key {
                'A' | 'a' => InputEvent::LetterA,
                'B' | 'b' => InputEvent::LetterB,
                'C' | 'c' => InputEvent::LetterC,
                'D' | 'd' => InputEvent::LetterD,
                '0' => InputEvent::Number0,
                '1' => InputEvent::Number1,
                '2' => InputEvent::Number2,
                '3' => InputEvent::Number3,
                '4' => InputEvent::Number4,
                '5' => InputEvent::Number5,
                '6' => InputEvent::Number6,
                '7' => InputEvent::Number7,
                '8' => InputEvent::Number8,
                '9' => InputEvent::Number9,
                '#' => InputEvent::Hashtag,
                '*' => InputEvent::Asterisk,
                _ => {
                    Timer::after(Duration::from_millis(50)).await;
                    continue
                },
            };

            let _ = event_sender.send(event).await;
        }
        Timer::after(Duration::from_millis(50)).await;
    };
}
