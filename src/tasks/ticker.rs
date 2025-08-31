use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Sender};
use embassy_time::{Duration, Timer};

use crate::events::{InputEvent, EVENT_QUEUE_SIZE};

#[embassy_executor::task]
pub async fn tick_task(
    event_sender: Sender<'static, NoopRawMutex, InputEvent, {EVENT_QUEUE_SIZE}>,
) {
    loop {
        let _ = event_sender.send(InputEvent::GameTick).await;
        Timer::after(Duration::from_secs(1)).await;
    }
}
