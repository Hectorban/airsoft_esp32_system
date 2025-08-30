use embassy_sync::channel::{Channel, Receiver};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use crate::devices::neopixel::NeoPixelStrip;
use smart_leds::RGB8;

#[derive(Debug, Clone, Copy)]
pub enum LightsCommand {
    SetAllColor { r: u8, g: u8, b: u8 },
    TurnOff,
    Flash { r: u8, g: u8, b: u8, duration_ms: u32 },
}

pub const LIGHTS_QUEUE_SIZE: usize = 8;
pub type LightsChannel = Channel<NoopRawMutex, LightsCommand, { LIGHTS_QUEUE_SIZE }>;

// TODO wrap these things in a result
#[embassy_executor::task]
pub async fn lights_task(
    receiver: Receiver<'static, NoopRawMutex, LightsCommand, { LIGHTS_QUEUE_SIZE }>,
    mut led_strip: NeoPixelStrip<0, 217>,
) {
    // Turn off LEDs initially
    let _ = led_strip.off_all();

    loop {
        let command = receiver.receive().await;
        match command {
                LightsCommand::SetAllColor { r, g, b } => {
                    let _ = led_strip.set_all_color(RGB8::new(r, g, b));
                },
                LightsCommand::TurnOff => {
                    let _ = led_strip.off_all();
                },
                LightsCommand::Flash { r, g, b, duration_ms } => {
                    let _ = led_strip.set_all_color(RGB8::new(r, g, b));
                    embassy_time::Timer::after(embassy_time::Duration::from_millis(duration_ms as u64)).await;
                    let _ = led_strip.off_all();
                },
            }
    }
}
