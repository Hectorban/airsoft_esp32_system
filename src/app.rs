use anyhow::Result;
use embedded_graphics::prelude::{DrawTarget, PixelColor};
use mousefood::EmbeddedBackend;
use ratatouille::{Frame, Terminal};

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
                    I2CInterface<I2cDevice<'_, NoopRawMutex, I2c<'static, esp_hal::Blocking>>>,
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
