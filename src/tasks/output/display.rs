use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::{Channel, Receiver};
use embassy_time::{Delay, Duration, Timer};
use esp_hal::i2c::master::I2c;
use esp_hal::Async;
use hd44780_driver::bus::I2CBus;
use hd44780_driver::charset::{CharsetUniversal, Fallback};
use hd44780_driver::memory_map::StandardMemoryMap;
use hd44780_driver::non_blocking::HD44780;
use crate::tasks::output::display_diff::DisplayDiffer;

extern crate alloc;
use alloc::string::String;
use hd44780_driver::Direction;

#[derive(Debug, Clone)]
pub enum DisplayCommand {
    WriteText { line1: String, line2: String },
    Clear,
    SetCursor { row: u8, col: u8 },
    WriteAt { text: String, row: u8, col: u8 },
    ScrollText { text: String, col: i32, times: u32, direction: ScrollDirection }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScrollDirection {
    Left,
    Right,
}

pub const DISPLAY_QUEUE_SIZE: usize = 16;
pub type DisplayChannel = Channel<NoopRawMutex, DisplayCommand, { DISPLAY_QUEUE_SIZE }>;

#[embassy_executor::task]
pub async fn display_task(
    receiver: Receiver<'static, NoopRawMutex, DisplayCommand, { DISPLAY_QUEUE_SIZE }>,
    mut display: HD44780<
        I2CBus<I2cDevice<'static, NoopRawMutex, I2c<'static, Async>>>,
        StandardMemoryMap<16, 2>,
        Fallback<CharsetUniversal, 32>,
    >,
) {
    display.clear(&mut Delay).await.unwrap();
    let mut differ = DisplayDiffer::new();

    // Todo manage errors and restart the display
    loop {
        let command = receiver.receive().await;
        
        // Use the diffing engine to check if we need to update the display
        if let Some(filtered_command) = differ.diff_command(&command) {
            match filtered_command {
            DisplayCommand::WriteText { line1, line2 } => {
                display.clear(&mut Delay).await.unwrap();
                display.write_str(&line1, &mut Delay).await.unwrap();
                display.set_cursor_pos(40, &mut Delay).await.unwrap(); // Second line
                display.write_str(&line2, &mut Delay).await.unwrap();
            }
            DisplayCommand::Clear => {
                display.clear(&mut Delay).await.unwrap();
            }
            DisplayCommand::SetCursor { row, col } => {
                let pos = if row == 0 { col } else { 40 + col };
                display.set_cursor_pos(pos, &mut Delay).await.unwrap();
            }
            DisplayCommand::WriteAt { text, row, col } => {
                let pos = if row == 0 { col } else { 40 + col };
                display.set_cursor_pos(pos, &mut Delay).await.unwrap();
                display.write_str(&text, &mut Delay).await.unwrap();
            }
            DisplayCommand::ScrollText { text, col: _col, times, direction } => {
                display.clear(&mut Delay).await.unwrap();
                display.reset(&mut Delay).await.unwrap();
                display.write_str(&text, &mut Delay).await.unwrap();

                for _ in 0..times * 4 {
                    if direction == ScrollDirection::Left {
                        display.shift_display(Direction::Left, &mut Delay).await.unwrap();
                        Timer::after(Duration::from_millis(100)).await;
                    } else {
                        display.shift_display(Direction::Right, &mut Delay).await.unwrap();
                        Timer::after(Duration::from_millis(100)).await;
                    }
                }
            }
            }
        }
        // If differ.diff_command returns None, the command is filtered out (no display update needed)
    }
}
