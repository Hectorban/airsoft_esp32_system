use embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::{Duration, Instant};
use esp_hal::{i2c::master::I2c as EspI2c, Blocking};
use defmt::info;
use embedded_hal::i2c::I2c;

type I2cType = EspI2c<'static, Blocking>;
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

    const KEYPAD_KEYS_2: [[char; 4]; 4] = [
        ['1', '2', '3', 'a'],
        ['4', '5', '6', 'b'],
        ['7', '8', '9', 'c'],
        ['*', '0', '#', 'd'],
    ];

    const KEYPAD_KEYS_3: [[char; 4]; 4] = [
        ['d', 'c', 'b', 'a'],
        ['#', '9', '6', '3'],
        ['0', '8', '5', '2'],
        ['*', '7', '4', '1'],
    ];

    pub fn new(address: u8, i2c: SharedI2cDevice) -> Self {
        Self { 
            address, 
            i2c,
            last_key: None,
            last_press_time: None,
            debounce_duration: Duration::from_millis(100), // 100ms debounce
        }
    }

    pub fn with_debounce_duration(mut self, duration: Duration) -> Self {
        self.debounce_duration = duration;
        self
    }

    pub fn scan(&mut self) -> Option<char> {
        let current_key = self.scan_raw();
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

    fn scan_raw(&mut self) -> Option<char> {
        for col in 0..4 {
            // Set column low, others high
            let col_mask = !(1 << (col + 4));
            if self.i2c.write(self.address, &[col_mask]).is_err() {
                // I2C write failed, skip this column
                continue;
            }

            // Read rows
            let mut buf = [0u8];
            match self.i2c.read(self.address, &mut buf) {
                Ok(_) => {
                    let rows = buf[0] & 0x0F;
                    
                    // Add some basic noise filtering
                    if rows == 0x0F {
                        // All rows high - no key pressed, continue
                        continue;
                    }
                    
                    for row in 0..4 {
                        if rows & (1 << row) == 0 {
                            // Reset all columns high
                            let _ = self.i2c.write(self.address, &[0xF0]);
                            info!("address: {} {}", row, col);
                            return Some(Self::KEYPAD_KEYS[row][col]);
                        }
                    }
                }
                Err(_) => {
                    // I2C read failed, skip this scan cycle
                    continue;
                }
            }
        }

        // Reset all columns high
        let _ = self.i2c.write(self.address, &[0xF0]);
        None
    }
}
