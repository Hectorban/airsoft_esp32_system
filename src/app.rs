use embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embedded_graphics::pixelcolor::BinaryColor;
use esp_hal::i2c::master::I2c;
use mousefood::EmbeddedBackend;
use ratatui::{
    Frame, Terminal,
    buffer::Buffer,
    layout::Rect,
    widgets::Widget,
};
use ssd1306::{
    Ssd1306, mode::BufferedGraphicsMode, prelude::I2CInterface, size::DisplaySize128x64,
};

use crate::{
    events::{InputEvent, TaskSenders},
    tasks::output::sound::SoundCommand,
    views::{Router, NavigationAction},
};
use ector::{Actor, DynamicAddress, Inbox};

extern crate alloc;

pub type I2cType = I2c<'static, esp_hal::Blocking>;

pub type DisplayType<'a> = Ssd1306<
    I2CInterface<I2cDevice<'a, NoopRawMutex, I2cType>>,
    DisplaySize128x64,
    BufferedGraphicsMode<DisplaySize128x64>,
>;

pub type BackendType<'a> = EmbeddedBackend<'a, DisplayType<'a>, BinaryColor>;

pub type TerminalType<'a> = Terminal<BackendType<'a>>;

// pub mod components; // Removed - using views module instead

pub struct App {
    router: Router,
    task_senders: TaskSenders,
    terminal: Option<TerminalType<'static>>,
}

impl App {
    pub fn new(task_senders: TaskSenders) -> Self {
        let router = Router::new(&task_senders);
        Self {
            router,
            task_senders,
            terminal: None,
        }
    }

    pub fn set_terminal(&mut self, terminal: TerminalType<'static>) {
        self.terminal = Some(terminal);
    }

    fn draw(&self, frame: &mut Frame) {
        self.router.render(frame, frame.area());
    }

    fn handle_input_event(&mut self, event: InputEvent) -> Option<NavigationAction> {
        self.router.handle_input(event, &self.task_senders)
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
                let _ = terminal.draw(|frame| {
                    self.router.render(frame, frame.area());
                });
            }

            // Handle incoming messages
            let event = inbox.next().await;
            if let Some(nav_action) = self.handle_input_event(event) {
                // Handle any navigation actions if needed
                // The router handles navigation internally, but we could
                // handle app-level actions like Exit here
                match nav_action {
                    NavigationAction::Exit => {
                        // Could handle app exit here if needed
                    },
                    _ => {}
                }
            }
        }
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // The router handles all rendering now
        // This Widget impl is kept for compatibility but shouldn't be used
        // Use the draw() method instead which calls router.render()
    }
}
