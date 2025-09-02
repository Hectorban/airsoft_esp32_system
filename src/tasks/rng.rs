use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Channel, Receiver, Sender},
};
use esp_hal::rng::Rng;

pub const RNG_QUEUE_SIZE: usize = 4;
const REPLY_QUEUE_SIZE: usize = 1;

// The channel to send the response back on.
pub type RngResponseChannel = Channel<NoopRawMutex, u32, REPLY_QUEUE_SIZE>;

// The sender part of the response channel.
pub type RngResponseSender = Sender<'static, NoopRawMutex, u32, REPLY_QUEUE_SIZE>;

pub enum RngCommand {
    GetU32 { reply: RngResponseSender },
}

pub type RngChannel = Channel<NoopRawMutex, RngCommand, { RNG_QUEUE_SIZE }>;

pub async fn rng_task(
    mut rng: Rng,
    receiver: Receiver<'static, NoopRawMutex, RngCommand, { RNG_QUEUE_SIZE }>,
) {
    loop {
        match receiver.receive().await {
            RngCommand::GetU32 { reply } => {
                let random_value = rng.random();
                reply.send(random_value).await;
            }
        }
    }
}
