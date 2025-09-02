use crate::devices::neopixel::NeoPixelStrip;
use ector::{Actor, DynamicAddress, Inbox};
use embassy_time::Timer;
use smart_leds::RGB8;

#[derive(Debug, Clone, Copy)]
pub enum LightsCommand {
    SetAllColor { r: u8, g: u8, b: u8 },
    TurnOff,
    Flash { r: u8, g: u8, b: u8, duration_ms: u32 },
}

pub struct LightsActor<const N: usize> {
    led_strip: NeoPixelStrip<0, N>,
}

impl<const N: usize> LightsActor<N> {
    pub fn new(led_strip: NeoPixelStrip<0, N>) -> Self {
        Self { led_strip }
    }
}

impl<const N: usize> Actor for LightsActor<N> {
    type Message = LightsCommand;

    async fn on_mount<M>(&mut self, _: DynamicAddress<Self::Message>, mut inbox: M) -> !
    where
        M: Inbox<Self::Message>,
    {
        // Turn off LEDs initially
        let _ = self.led_strip.off_all();

        loop {
            let command = inbox.next().await;
            match command {
                LightsCommand::SetAllColor { r, g, b } => {
                    let _ = self.led_strip.set_all_color(RGB8::new(r, g, b));
                }
                LightsCommand::TurnOff => {
                    let _ = self.led_strip.off_all();
                }
                LightsCommand::Flash { r, g, b, duration_ms } => {
                    let _ = self.led_strip.set_all_color(RGB8::new(r, g, b));
                    Timer::after(embassy_time::Duration::from_millis(duration_ms as u64)).await;
                    let _ = self.led_strip.off_all();
                }
            }
        }
    }
}
