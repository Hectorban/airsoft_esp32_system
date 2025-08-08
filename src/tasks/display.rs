use embassy_sync::channel::{Channel, Receiver};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::Delay;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use esp_hal::i2c::master::I2c;
use esp_hal::Async;
use hd44780_driver::bus::I2CBus;
use hd44780_driver::charset::{CharsetUniversal, Fallback};
use hd44780_driver::non_blocking::HD44780;
use hd44780_driver::memory_map::StandardMemoryMap;

extern crate alloc;
use alloc::string::String;

#[derive(Debug, Clone)]
pub enum DisplayCommand {
    WriteText { line1: String, line2: String },
    Clear,
    SetCursor { row: u8, col: u8 },
    WriteAt { text: String, row: u8, col: u8 },
}

pub const DISPLAY_QUEUE_SIZE: usize = 16;
pub type DisplayChannel = Channel<NoopRawMutex, DisplayCommand, { DISPLAY_QUEUE_SIZE }>;

#[embassy_executor::task]
pub async fn display_task(
    receiver: Receiver<'static, NoopRawMutex, DisplayCommand, { DISPLAY_QUEUE_SIZE }>,
    mut display: HD44780<I2CBus<I2cDevice<'static, NoopRawMutex, I2c<'static, Async>>>, StandardMemoryMap<16, 2>, Fallback<CharsetUniversal, 32>>
) {
    display.clear(&mut Delay).await.unwrap();
    
    loop {
        let command = receiver.receive().await;
        match command {
                DisplayCommand::WriteText { line1, line2 } => {
                    let _ = display.clear(&mut Delay).await;
                    let _ = display.write_str(&line1, &mut Delay).await;
                    let _ = display.set_cursor_pos(40, &mut Delay).await; // Second line
                    let _ = display.write_str(&line2, &mut Delay).await;
                },
                DisplayCommand::Clear => {
                    let _ = display.clear(&mut Delay).await;
                },
                DisplayCommand::SetCursor { row, col } => {
                    let pos = if row == 0 { col } else { 40 + col };
                    let _ = display.set_cursor_pos(pos, &mut Delay).await;
                },
                DisplayCommand::WriteAt { text, row, col } => {
                    let pos = if row == 0 { col } else { 40 + col };
                    let _ = display.set_cursor_pos(pos, &mut Delay).await;
                    let _ = display.write_str(&text, &mut Delay).await;
                },
            }
    }
}
