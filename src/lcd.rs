use defmt::error;
use esp_hal::{i2c::master::I2c, Async};
use hd44780_driver::{
    bus::I2CBus,
    charset::{CharsetUniversal, Fallback},
    memory_map::MemoryMap1602,
    setup::DisplayOptionsI2C,
    HD44780, CursorBlink, Display, Cursor
};

type I2cType = I2c<'static, Async>;
pub type LcdDisplay = HD44780<I2CBus<I2cType>, MemoryMap1602, Fallback<CharsetUniversal, 32>>;
type LcdError = hd44780_driver::error::Error<esp_hal::i2c::master::Error>;

/// Extension trait for HD44780 to add convenient methods
pub trait LcdExt {
    /// Write text at a specific position (row: 0-1, col: 0-15 for 16x2 display)
    fn write_at(&mut self, row: u8, col: u8, text: &str) -> Result<(), LcdError>;
    
    /// Display a message on the first line, clearing the display first
    fn display_message(&mut self, message: &str) -> Result<(), LcdError>;
    
    /// Display a two-line message
    fn display_two_lines(&mut self, line1: &str, line2: &str) -> Result<(), LcdError>;
    
    /// Toggle display on/off
    fn toggle_display(&mut self, enabled: bool) -> Result<(), LcdError>;
    
    /// Configure cursor appearance (visible, blinking)
    fn configure_cursor(&mut self, visible: bool, blink: bool) -> Result<(), LcdError>;
}

/// Implement the extension trait for our LCD type
impl LcdExt for LcdDisplay {
    fn write_at(&mut self, row: u8, col: u8, text: &str) -> Result<(), LcdError> {
        // Calculate position for 16x2 display (row 0: pos 0-15, row 1: pos 64-79)
        let pos = if row == 0 { col } else { 64 + col };
        self.set_cursor_pos(pos, &mut embassy_time::Delay)?;
        self.write_str(text, &mut embassy_time::Delay)
    }
    
    fn display_message(&mut self, message: &str) -> Result<(), LcdError> {
        self.reset(&mut embassy_time::Delay)?;
        self.clear(&mut embassy_time::Delay)?;
        self.write_str(message, &mut embassy_time::Delay)
    }
    
    fn display_two_lines(&mut self, line1: &str, line2: &str) -> Result<(), LcdError> {
        self.reset(&mut embassy_time::Delay)?;
        self.clear(&mut embassy_time::Delay)?;
        self.write_at(0, 0, line1)?;
        self.write_at(1, 0, line2)
    }
    
    fn toggle_display(&mut self, enabled: bool) -> Result<(), LcdError> {
        let display_mode = if enabled { Display::On } else { Display::Off };
        self.set_display(display_mode, &mut embassy_time::Delay)
    }
    
    fn configure_cursor(&mut self, visible: bool, blink: bool) -> Result<(), LcdError> {
        let cursor_mode = if visible { Cursor::Visible } else { Cursor::Invisible };
        self.set_cursor_visibility(cursor_mode, &mut embassy_time::Delay)?;
        let blink_mode = if blink { CursorBlink::On } else { CursorBlink::Off };
        self.set_cursor_blink(blink_mode, &mut embassy_time::Delay)
    }
}

/// Helper function to create and initialize an LCD display
pub async fn create_lcd(address: u8, i2c: I2cType) -> LcdDisplay {
    let mut options = DisplayOptionsI2C::new(MemoryMap1602::new()).with_i2c_bus(i2c, address);

    loop {
        match HD44780::new(options, &mut embassy_time::Delay) {
            Err((options_back, error)) => {
                error!("Error creating LCD Driver: {:?}", defmt::Debug2Format(&error));
                options = options_back;
                embassy_time::Timer::after(embassy_time::Duration::from_millis(500)).await;
                // try again
            }
            Ok(display) => break display,
        }
    }
}
