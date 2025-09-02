use alloc::{string::ToString, vec};
use embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embedded_graphics::pixelcolor::BinaryColor;
use esp_hal::i2c::master::I2c;
use mousefood::EmbeddedBackend;
use ratatui::{
    Frame, Terminal,
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget, Wrap},
};
use ssd1306::{
    Ssd1306, mode::BufferedGraphicsMode, prelude::I2CInterface, size::DisplaySize128x64,
};

use crate::{
    events::{InputEvent, TaskSenders},
    tasks::output::sound::SoundCommand,
};
use ector::{Actor, ActorAddress, ActorRequest, DynamicAddress, Inbox};

extern crate alloc;

pub type I2cType = I2c<'static, esp_hal::Blocking>;

pub type DisplayType<'a> = Ssd1306<
    I2CInterface<I2cDevice<'a, NoopRawMutex, I2cType>>,
    DisplaySize128x64,
    BufferedGraphicsMode<DisplaySize128x64>,
>;

pub type BackendType<'a> = EmbeddedBackend<'a, DisplayType<'a>, BinaryColor>;

pub type TerminalType<'a> = Terminal<BackendType<'a>>;

pub mod components;

pub struct App {
    counter: u32,
    task_senders: TaskSenders,
    terminal: Option<TerminalType<'static>>,
}

impl App {
    pub fn new(task_senders: TaskSenders) -> Self {
        Self {
            counter: 0,
            task_senders,
            terminal: None,
        }
    }

    pub fn set_terminal(&mut self, terminal: TerminalType<'static>) {
        self.terminal = Some(terminal);
    }

    async fn get_random_u32(&self) -> u32 {
        self.task_senders.rng.request(()).await
    }

    async fn play_sound(&self, sound: SoundCommand) {
        self.task_senders.sound.notify(sound).await;
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_input_event(&mut self, event: InputEvent) {
        match event {
            InputEvent::KeypadEvent(key) => match key {
                'a' => self.counter += 1,
                'b' => self.counter -= 1,
                _ => {}
            },
            _ => {}
        }
    }
}

impl Actor for App {
    type Message = InputEvent;

    async fn on_mount<M>(&mut self, _: DynamicAddress<Self::Message>, mut inbox: M) -> !
    where
        M: Inbox<Self::Message>,
    {
        loop {
            // Draw the UI first
            if let Some(terminal) = self.terminal.as_mut() {
                let counter = self.counter; // Copy counter to avoid borrow issues
                let _ = terminal.draw(|frame| {
                    // Create a temporary widget for rendering
                    let widget = AppWidget { counter };
                    frame.render_widget(&widget, frame.area());
                });
            }

            // Handle incoming messages
            let event = inbox.next().await;
            self.handle_input_event(event);
        }
    }
}

struct AppWidget {
    counter: u32,
}

impl Widget for &AppWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" Airsoft ".bold());
        let instructions = Line::from(vec![
            " ↓ ".into(),
            "<A> ".blue().bold(),
            " ↑ ".into(),
            "<B>".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.right_aligned())
            .border_set(border::THICK);

        let counter_text = Text::from(vec![Line::from(vec![
            "Value: ".into(),
            self.counter.to_string().yellow(),
        ])]);

        Paragraph::new(counter_text)
            .block(block)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let widget = AppWidget {
            counter: self.counter,
        };
        widget.render(area, buf);
    }
}
