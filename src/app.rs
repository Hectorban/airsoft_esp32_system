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
use static_cell::StaticCell;

use crate::{
    events::{EventBus, InputEvent, TaskSenders},
    tasks::rng::{RngCommand, RngResponseChannel},
};

extern crate alloc;

pub type I2cType = I2c<'static, esp_hal::Blocking>;

pub type DisplayType<'a> =
    Ssd1306<I2CInterface<I2cDevice<'a, NoopRawMutex, I2cType>>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>;

pub type BackendType<'a> = EmbeddedBackend<'a, DisplayType<'a>, BinaryColor>;

pub type TerminalType<'a> = Terminal<BackendType<'a>>;

pub mod components;

pub struct App {
    random_number: u32,
    exit: bool,
    event_bus: EventBus,
    task_senders: TaskSenders,
}

impl App {
    pub fn new(event_bus: EventBus, task_senders: TaskSenders) -> Self {
        Self {
            random_number: 0,
            exit: false,
            event_bus,
            task_senders,
        }
    }

    /// runs the application's main loop until the user quits
    pub async fn run(&mut self, terminal: &mut TerminalType<'static>) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events().await?;
        }
        Ok(())
    }

    async fn get_random_u32(&self) -> u32 {
        static REPLY_CHANNEL: StaticCell<RngResponseChannel> = StaticCell::new();
        let reply_channel = REPLY_CHANNEL.init(RngResponseChannel::new());
        let cmd = RngCommand::GetU32 {
            reply: reply_channel.sender(),
        };
        self.task_senders.rng.send(cmd).await;
        reply_channel.receive().await
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    async fn handle_events(&mut self) -> Result<()> {
        match self.event_bus.event_receiver.receive().await {
            InputEvent::KeypadEvent(key) => match key {
                'A' | 'a' => self.random_number = self.get_random_u32().await,
                'D' | 'd' => self.exit = true,
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" RNG App ".bold());
        let instructions = Line::from(vec![
            " Get Random ".into(),
            "<A> ".blue().bold(),
            " Quit ".into(),
            "<D>".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.right_aligned())
            .border_set(border::THICK);

        let counter_text = Text::from(vec![Line::from(vec![
            "Value: ".into(),
            self.random_number.to_string().yellow(),
        ])]);

        Paragraph::new(counter_text)
            .centered()
            .block(block)
            .render(area, buf);
    }
}
