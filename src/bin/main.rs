#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use airsoft_v2::lcd::{self, LcdExt};
use airsoft_v2::web::{self, WebApp};
use airsoft_v2::wifi::start_wifi;
use airsoft_v2::{keypad, mk_static};

use alloc::string::ToString;
use defmt::info;
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::i2c::master::I2c;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{i2c, Async};
use esp_hal_smartled::buffer_size;
use esp_println as _;
use esp_wifi::EspWifiController;
use static_cell::StaticCell;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

const NUM_LEDS: usize = 10;
const BUFFER_SIZE: usize = buffer_size(NUM_LEDS);

type I2cType = I2c<'static, Async>;

static I2C_BUS: StaticCell<Mutex<NoopRawMutex, I2cType>> = StaticCell::new();

const LCD_ADDRESS: u8 = 0x27; // or 0x3F
const KEYPAD_ADDRESS: u8 = 0x20; // or 0x21-0x27

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.5.0

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 64 * 1024);
    // COEX needs more RAM - so we've added some more
    esp_alloc::heap_allocator!(#[unsafe(link_section = ".dram2_uninit")] size: 64 * 1024);

    let timer0 = TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timer0.timer0);

    info!("Embassy initialized!");

    let rng = esp_hal::rng::Rng::new(peripherals.RNG);
    let timer1 = TimerGroup::new(peripherals.TIMG0);
    let esp_wifi_ctrl = mk_static!(
        EspWifiController<'static>,
        esp_wifi::init(timer1.timer0, rng).unwrap()
    );

    // let transport = BleConnector::new(esp_wifi_ctrl, peripherals.BT);
    // let _ble_controller = ExternalController::<_, 20>::new(transport);

    info!("Attempting to start wifi..");
    let stack = start_wifi(esp_wifi_ctrl, peripherals.WIFI, rng, &spawner).await;
    let webapp = WebApp::default();

    for id in 0..web::WEB_TASK_POOL_SIZE {
        spawner.must_spawn(web::web_task(id, stack, webapp.router, webapp.config));
    }

    // Initialize I2C
    let i2c = I2c::new(
        peripherals.I2C0,
        i2c::master::Config::default().with_frequency(Rate::from_khz(100)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO21)
    .with_scl(peripherals.GPIO22)
    .into_async();

    let mut lcd = lcd::create_lcd(LCD_ADDRESS, i2c).await;
    lcd.display_message("Ready!").unwrap();
    let mut keypad = keypad::I2cKeypad::new(KEYPAD_ADDRESS, i2c);

    loop {
        if let Some(key) = keypad.scan().await {
            info!("Key pressed: {}", key);
            lcd.display_two_lines("Key", &key.to_string()).unwrap();
        }
        Timer::after(Duration::from_millis(100)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-rc.0/examples/src/bin
}
