use anyhow::Result;
use embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embedded_graphics::prelude::{DrawTarget, PixelColor};
use esp_hal::i2c::master::I2c;
use mousefood::EmbeddedBackend;
use ratatui::{Frame, Terminal};
use ssd1306::{prelude::I2CInterface, size::DisplaySize128x64, Ssd1306};

pub type I2cType = I2c<'static, esp_hal::Blocking>;

extern crate alloc;
pub mod main_menu;
pub mod search_and_destroy;

#[derive(Debug, Default)]
pub struct App {
    counter: u8,
    exit: bool,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run<'display, D, C>(
        &mut self,
        terminal: &mut Terminal<
            EmbeddedBackend<
                '_,
                Ssd1306<
                I2CInterface<I2cDevice<'_, NoopRawMutex, I2cType>>,
                DisplaySize128x64,
                ssd1306::mode::BufferedGraphicsMode<DisplaySize128x64>,
            >,
                embedded_graphics::pixelcolor::BinaryColor,
            >,
        >,
    ) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        todo!()
    }

    fn handle_events(&mut self) -> Result<()> {
        todo!()
    }
}
