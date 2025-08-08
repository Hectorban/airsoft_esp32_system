use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embassy_time::{Duration, Timer, Instant};
use embedded_hal_async::i2c::I2c;
use esp_hal::{i2c::master::I2c as EspI2c, Async};

type I2cType = EspI2c<'static, Async>;
type SharedI2cDevice = I2cDevice<'static, NoopRawMutex, I2cType>;

pub struct I2cKeypad {
    address: u8,
    i2c: SharedI2cDevice,
    last_key: Option<char>,
    last_press_time: Option<Instant>,
    debounce_duration: Duration,
}

impl I2cKeypad {
    const KEYPAD_KEYS: [[char; 4]; 4] = [
        ['d', '#', '0', '*'],
        ['c', '9', '8', '7'],
        ['b', '6', '5', '4'],
        ['a', '3', '2', '1'],
    ];

    pub fn new(address: u8, i2c: SharedI2cDevice) -> Self {
        Self { 
            address, 
            i2c,
            last_key: None,
            last_press_time: None,
            debounce_duration: Duration::from_millis(50), // 50ms debounce
        }
    }

    pub fn with_debounce_duration(mut self, duration: Duration) -> Self {
        self.debounce_duration = duration;
        self
    }

    pub async fn scan(&mut self) -> Option<char> {
        let current_key = self.scan_raw().await;
        let now = Instant::now();

        match (current_key, self.last_key) {
            // No key pressed
            (None, _) => {
                self.last_key = None;
                self.last_press_time = None;
                None
            }
            // Same key as before - check debounce
            (Some(key), Some(last_key)) if key == last_key => {
                if let Some(last_time) = self.last_press_time {
                    if now.duration_since(last_time) >= self.debounce_duration {
                        // Key held long enough, but don't repeat
                        None
                    } else {
                        // Still in debounce period
                        None
                    }
                } else {
                    // This shouldn't happen, but handle gracefully
                    None
                }
            }
            // New key pressed
            (Some(key), _) => {
                self.last_key = Some(key);
                self.last_press_time = Some(now);
                Some(key)
            }
        }
    }

    async fn scan_raw(&mut self) -> Option<char> {
        for col in 0..4 {
            // Set column low, others high
            let col_mask = !(1 << (col + 4));
            let _ = self.i2c.write(self.address, &[col_mask]).await;
            Timer::after(Duration::from_micros(10)).await;

            // Read rows
            let mut buf = [0u8];
            let read_ok = self.i2c.read(self.address, &mut buf).await.is_ok();

            if read_ok {
                let rows = buf[0] & 0x0F;

                for row in 0..4 {
                    if rows & (1 << row) == 0 {
                        // Reset all columns high
                        let _ = self.i2c.write(self.address, &[0xF0]).await;
                        return Some(Self::KEYPAD_KEYS[row][col]);
                    }
                }
            }
        }

        // Reset all columns high
        let _ = self.i2c.write(self.address, &[0xF0]).await;
        None
    }
}
