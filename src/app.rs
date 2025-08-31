use anyhow::Result;
use embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embedded_graphics::pixelcolor::BinaryColor;
use esp_hal::i2c::master::I2c;
use mousefood::EmbeddedBackend;
use ratatui::{
    buffer::Buffer, 
    layout::Rect, 
    style::Stylize, 
    text::{Line, Text}, 
    widgets::{Block, Paragraph, Widget}, 
    Frame, 
    symbols::border,
    Terminal
};
use ssd1306::{
    mode::BufferedGraphicsMode, prelude::I2CInterface, size::DisplaySize128x64, Ssd1306,
};
use alloc::{vec, string::ToString};

use crate::events::{EventBus, InputEvent};

extern crate alloc;

pub type I2cType = I2c<'static, esp_hal::Blocking>;

pub type DisplayType<'a> =
    Ssd1306<I2CInterface<I2cDevice<'a, NoopRawMutex, I2cType>>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>;

pub type BackendType<'a> = EmbeddedBackend<'a, DisplayType<'a>, BinaryColor>;

pub type TerminalType<'a> = Terminal<BackendType<'a>>;

pub mod main_menu;
pub mod search_and_destroy;

pub struct App {
    counter: u8,
    exit: bool,
    event_bus: EventBus
}

impl App {
    /// runs the application's main loop until the user quits
    pub async fn run(&mut self, terminal: &mut TerminalType<'static>) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events().await?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    async fn handle_events(&mut self) -> Result<()> {
        match self.event_bus.event_receiver.receive().await {
            InputEvent::KeypadEvent(key) => {
                match key {
                    'A' | 'a' => self.counter += 1,
                    'B' | 'b' => self.counter -= 1,
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" Counter App Tutorial ".bold());
        let instructions = Line::from(vec![
            " Decrement ".into(),
            "<Left>".blue().bold(),
            " Increment ".into(),
            "<Right>".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let counter_text = Text::from(vec![Line::from(vec![
            "Value: ".into(),
            self.counter.to_string().yellow(),
        ])]);

        Paragraph::new(counter_text)
            .centered()
            .block(block)
            .render(area, buf);
    }
}
