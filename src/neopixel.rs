use esp_hal::rmt::{ConstChannelAccess, Tx};
use esp_hal_smartled::{LedAdapterError, SmartLedsAdapter};
use smart_leds::{brightness, SmartLedsWrite, RGB8};

const BRIGHTNESS: u8 = 32;

pub struct NeoPixelStrip<const CHANNEL: u8, const N: usize> {
    adapter: SmartLedsAdapter<ConstChannelAccess<Tx, CHANNEL>, N>,
    num_leds: usize,
}

impl<const CHANNEL: u8, const N: usize> NeoPixelStrip<CHANNEL, N> {
    pub fn new(adapter: SmartLedsAdapter<ConstChannelAccess<Tx, CHANNEL>, N>, num_leds: usize) -> Self {
        Self { adapter, num_leds }
    }

    pub fn set_all_color(&mut self, color: RGB8) -> Result<(), LedAdapterError> {
        let colors = core::iter::repeat(color).take(self.num_leds);
        self.adapter.write(brightness(colors, BRIGHTNESS))
    }

    pub fn off_all(&mut self) -> Result<(), LedAdapterError> {
        let black = RGB8::new(0, 0, 0);
        let colors = core::iter::repeat(black).take(self.num_leds);
        self.adapter.write(brightness(colors, BRIGHTNESS))
    }
}
