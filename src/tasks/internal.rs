use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Sender};

use crate::events::{InputEvent, EVENT_QUEUE_SIZE};

#[embassy_executor::task]
pub async fn game_ticker_task(
    event_sender: Sender<'static, NoopRawMutex, InputEvent, {EVENT_QUEUE_SIZE}>,
)  {
    loop {
        event_sender.send(InputEvent::GameTick).await;
        embassy_time::Timer::after(embassy_time::Duration::from_secs(1)).await;
    }
}
    