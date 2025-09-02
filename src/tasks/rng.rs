use ector::{Actor, DynamicAddress, Inbox, Request};
use esp_hal::rng::Rng;

pub type RngRequest = Request<(), u32>;

pub struct RngActor {
    rng: Rng,
}

impl RngActor {
    pub fn new(rng: Rng) -> Self {
        Self { rng }
    }
}

impl Actor for RngActor {
    type Message = RngRequest;

    async fn on_mount<M>(&mut self, _: DynamicAddress<Self::Message>, mut inbox: M) -> !
    where
        M: Inbox<Self::Message>,
    {
        loop {
            let req = inbox.next().await;
            let random_value = self.rng.random();
            req.reply(random_value).await;
        }
    }
}
