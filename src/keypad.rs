use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embassy_time::{Duration, Timer};
use esp_hal::{i2c::master::I2c, Async};

type I2cType = I2c<'static, Async>;

pub struct I2cKeypad<'a> {
    address: u8,
    i2c: &'a Mutex<NoopRawMutex, I2cType>,
}

impl<'a> I2cKeypad<'a> {
    const KEYPAD_KEYS: [[char; 4]; 4] = [
        ['d', '#', '0', '*'],
        ['c', '9', '8', '7'],
        ['b', '6', '5', '4'],
        ['a', '3', '2', '1'],
    ];

    pub fn new(address: u8, i2c: &'a Mutex<NoopRawMutex, I2cType>) -> Self {
        Self { address, i2c }
    }

    pub async fn scan(&mut self) -> Option<char> {
        for col in 0..4 {
            // Set column low, others high
            let col_mask = !(1 << (col + 4));
            {
                let mut i2c = self.i2c.lock().await;
                let _ = i2c.write(self.address, &[col_mask]);
            }
            Timer::after(Duration::from_micros(10)).await;

            // Read rows
            let mut buf = [0u8];
            let read_ok = {
                let mut i2c = self.i2c.lock().await;
                i2c.read(self.address, &mut buf).is_ok()
            };

            if read_ok {
                let rows = buf[0] & 0x0F;

                for row in 0..4 {
                    if rows & (1 << row) == 0 {
                        // Reset all columns high
                        {
                            let mut i2c = self.i2c.lock().await;
                            let _ = i2c.write(self.address, &[0xF0]);
                        }
                        return Some(Self::KEYPAD_KEYS[row][col]);
                    }
                }
            }
        }

        // Reset all columns high
        {
            let mut i2c = self.i2c.lock().await;
            let _ = i2c.write(self.address, &[0xF0]);
        }
        None
    }
}
