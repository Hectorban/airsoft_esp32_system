use embassy_sync::channel::{Channel, Receiver};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use crate::devices::neopixel::NeoPixelStrip;
use smart_leds::RGB8;

#[derive(Debug, Clone, Copy)]
pub enum LightsCommand {
    SetStrip1Color { r: u8, g: u8, b: u8 },
    SetStrip2Color { r: u8, g: u8, b: u8 },
    SetBothColors { r: u8, g: u8, b: u8 },
    SetStripColors { strip1: (u8, u8, u8), strip2: (u8, u8, u8) },
    TurnOff,
    Flash { r: u8, g: u8, b: u8, duration_ms: u32 },
}

pub const LIGHTS_QUEUE_SIZE: usize = 16;
pub type LightsChannel = Channel<NoopRawMutex, LightsCommand, { LIGHTS_QUEUE_SIZE }>;

#[embassy_executor::task]
pub async fn lights_task(
    receiver: Receiver<'static, NoopRawMutex, LightsCommand, { LIGHTS_QUEUE_SIZE }>,
    mut led_strip1: NeoPixelStrip<0, 217>,
    mut led_strip2: NeoPixelStrip<1, 217>,
) {
    // Turn off LEDs initially
    let _ = led_strip1.off_all();
    let _ = led_strip2.off_all();
    
    // Todo Background BGM?
    loop {
        let command = receiver.receive().await;
        match command {
                LightsCommand::SetStrip1Color { r, g, b } => {
                    let _ = led_strip1.set_all_color(RGB8::new(r, g, b));
                },
                LightsCommand::SetStrip2Color { r, g, b } => {
                    let _ = led_strip2.set_all_color(RGB8::new(r, g, b));
                },
                LightsCommand::SetBothColors { r, g, b } => {
                    let _ = led_strip1.set_all_color(RGB8::new(r, g, b));
                    let _ = led_strip2.set_all_color(RGB8::new(r, g, b));
                },
                LightsCommand::SetStripColors { strip1, strip2 } => {
                    let _ = led_strip1.set_all_color(RGB8::new(strip1.0, strip1.1, strip1.2));
                    let _ = led_strip2.set_all_color(RGB8::new(strip2.0, strip2.1, strip2.2));
                },
                LightsCommand::TurnOff => {
                    let _ = led_strip1.off_all();
                    let _ = led_strip2.off_all();
                },
                LightsCommand::Flash { r, g, b, duration_ms } => {
                    let _ = led_strip1.set_all_color(RGB8::new(r, g, b));
                    let _ = led_strip2.set_all_color(RGB8::new(r, g, b));
                    embassy_time::Timer::after(embassy_time::Duration::from_millis(duration_ms as u64)).await;
                    let _ = led_strip1.off_all();
                    let _ = led_strip2.off_all();
                },
            }
    }
}
