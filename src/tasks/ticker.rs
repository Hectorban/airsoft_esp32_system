use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Sender};
use embassy_time::{Duration, Timer};
use ector::{Actor, DynamicAddress, Inbox};

use crate::events::{InputEvent, EVENT_QUEUE_SIZE};

pub struct TickerActor {
    event_sender: Sender<'static, NoopRawMutex, InputEvent, { EVENT_QUEUE_SIZE }>,
}

impl TickerActor {
    pub fn new(
        event_sender: Sender<'static, NoopRawMutex, InputEvent, { EVENT_QUEUE_SIZE }>,
    ) -> Self {
        Self { event_sender }
    }
}

impl Actor for TickerActor {
    type Message = !;

    async fn on_mount<M>(&mut self, _: DynamicAddress<Self::Message>, _inbox: M) -> !
    where
        M: Inbox<Self::Message>,
    {
        loop {
            let _ = self.event_sender.send(InputEvent::GameTick).await;
            Timer::after(Duration::from_secs(1)).await;
        }
    }
}
