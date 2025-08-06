use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embassy_time::{Duration, Timer};
use esp_hal::{i2c::master::I2c, Async};

const LCD_BACKLIGHT: u8 = 0x08;
const LCD_ENABLE: u8 = 0x04;
const LCD_CMD: u8 = 0x00;
const LCD_CHAR: u8 = 0x01;

type I2cType = I2c<'static, Async>;

pub struct I2cLcd<'a> {
    address: u8,
    i2c: &'a Mutex<NoopRawMutex, I2cType>,
}

impl<'a> I2cLcd<'a> {
    pub fn new(address: u8, i2c: &'a Mutex<NoopRawMutex, I2cType>) -> Self {
        Self { address, i2c }
    }

    pub async fn init(&mut self) {
        Timer::after(Duration::from_millis(50)).await;

        // Initialize in 4-bit mode
        self.write_nibble(0x03, false).await;
        Timer::after(Duration::from_millis(5)).await;
        self.write_nibble(0x03, false).await;
        Timer::after(Duration::from_micros(150)).await;
        self.write_nibble(0x03, false).await;
        self.write_nibble(0x02, false).await;

        // Function set: 4-bit, 2 lines, 5x8 dots
        self.write_byte(0x28, false).await;
        // Display on, cursor off, blink off
        self.write_byte(0x0C, false).await;
        // Clear display
        self.write_byte(0x01, false).await;
        Timer::after(Duration::from_millis(2)).await;
        // Entry mode set
        self.write_byte(0x06, false).await;
    }

    async fn write_nibble(&mut self, nibble: u8, rs: bool) {
        let data = (nibble << 4) | LCD_BACKLIGHT | if rs { LCD_CHAR } else { LCD_CMD };
        {
            let mut i2c = self.i2c.lock().await;
            let _ = i2c.write(self.address, &[data | LCD_ENABLE]);
        }
        Timer::after(Duration::from_micros(1)).await;
        {
            let mut i2c = self.i2c.lock().await;
            let _ = i2c.write(self.address, &[data & !LCD_ENABLE]);
        }
        Timer::after(Duration::from_micros(50)).await;
    }

    async fn write_byte(&mut self, byte: u8, rs: bool) {
        self.write_nibble(byte >> 4, rs).await;
        self.write_nibble(byte & 0x0F, rs).await;
    }

    pub async fn clear(&mut self) {
        self.write_byte(0x01, false).await;
        Timer::after(Duration::from_millis(2)).await;
    }

    pub async fn set_cursor(&mut self, col: u8, row: u8) {
        let row_offsets = [0x00, 0x40];
        self.write_byte(0x80 | (col + row_offsets[row as usize]), false).await;
    }

    pub async fn print(&mut self, text: &str) {
        for ch in text.chars() {
            self.write_byte(ch as u8, true).await;
        }
    }

    // Helper method to print at a specific position
    pub async fn print_at(&mut self, col: u8, row: u8, text: &str) {
        self.set_cursor(col, row).await;
        self.print(text).await;
    }

    // Helper method to clear a line and print new text
    pub async fn clear_line_and_print(&mut self, row: u8, text: &str) {
        self.set_cursor(0, row).await;
        // Clear the line by writing spaces
        self.print("                ").await; // 16 spaces for 16x2 LCD
        self.set_cursor(0, row).await;
        self.print(text).await;
    }
}
