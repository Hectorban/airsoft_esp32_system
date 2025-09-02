use embassy_time::{Duration, Timer};
use ector::{Actor, ActorAddress, DynamicAddress, Inbox};

use crate::events::InputEvent;

pub struct TickerActor {
    app_address: DynamicAddress<InputEvent>,
}

impl TickerActor {
    pub fn new(app_address: DynamicAddress<InputEvent>) -> Self {
        Self { app_address }
    }
}

impl Actor for TickerActor {
    type Message = !;

    async fn on_mount<M>(&mut self, _: DynamicAddress<Self::Message>, _inbox: M) -> !
    where
        M: Inbox<Self::Message>,
    {
        loop {
            self.app_address.notify(InputEvent::GameTick).await;
            Timer::after(Duration::from_secs(1)).await;
        }
    }
}
