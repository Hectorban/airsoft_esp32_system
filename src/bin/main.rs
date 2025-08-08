#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use airsoft_v2::devices::buzzer::STARTUP_SOUND;
use airsoft_v2::devices::neopixel::NeoPixelStrip;
use airsoft_v2::web::{self, WebApp};
use airsoft_v2::wifi::start_wifi;
use airsoft_v2::{devices::keypad, mk_static};
use airsoft_v2::devices::nfc;
use airsoft_v2::events::{EventChannel, EventBus, GameEvent, TaskSenders, EVENT_QUEUE_SIZE};
use airsoft_v2::game::GameManager;
use airsoft_v2::tasks::{
    DisplayChannel, LightsChannel, SoundChannel,
    DisplayCommand, LightsCommand, SoundCommand,
    display_task, lights_task, sound_task
};

use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice as BlockingSpiDevice;
use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;
use embassy_time::{Delay, Duration, Timer};
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::ledc::{self, timer, LSGlobalClkSource, Ledc};
use esp_hal::i2c::master::I2c;
use esp_hal_buzzer::Buzzer;
use mfrc522::Mfrc522;
use core::cell::RefCell;
use esp_hal::clock::CpuClock;
use esp_hal::rmt::Rmt;
use esp_hal::spi::master::Spi;
use esp_hal::spi;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{i2c, Async};
use esp_hal_smartled::{buffer_size, smart_led_buffer, SmartLedsAdapter};
use esp_println as _;
use esp_wifi::EspWifiController;
use hd44780_driver::memory_map::MemoryMap1602;
use hd44780_driver::setup::DisplayOptionsI2C;
use hd44780_driver::non_blocking::HD44780;
use static_cell::StaticCell;    

use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

const NUM_LEDS: usize = 10;
const BUFFER_SIZE: usize = buffer_size(NUM_LEDS);

type I2cType = I2c<'static, esp_hal::Async>;

const LCD_ADDRESS: u8 = 0x27; // or 0x3F
const KEYPAD_ADDRESS: u8 = 0x20; // or 0x21-0x27

static I2C_BUS: StaticCell<Mutex<NoopRawMutex, I2cType>> = StaticCell::new();
static SPI_BUS_BLOCKING: StaticCell<BlockingMutex<NoopRawMutex, RefCell<Spi<'static, Async>>>> = StaticCell::new();
static EVENT_CHANNEL: StaticCell<EventChannel> = StaticCell::new();
static DISPLAY_CHANNEL: StaticCell<DisplayChannel> = StaticCell::new();
static LIGHTS_CHANNEL: StaticCell<LightsChannel> = StaticCell::new();
static SOUND_CHANNEL: StaticCell<SoundChannel> = StaticCell::new();

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
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

    // TODO Abstract spawning of devices

    info!("Attempting to start I2C bus..");
    let i2c = I2c::new(
        peripherals.I2C0,
        i2c::master::Config::default().with_frequency(Rate::from_khz(100)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO21)
    .with_scl(peripherals.GPIO22)
    .into_async();
    let i2c_bus = I2C_BUS.init(Mutex::new(i2c));

    let lcd_i2c = I2cDevice::new(i2c_bus);
    let mut options = DisplayOptionsI2C::new(MemoryMap1602::new()).with_i2c_bus(lcd_i2c, LCD_ADDRESS);

    let display = loop {
        match HD44780::new(options, &mut Delay).await {
            Err((options_back, error)) => {
                error!("Error creating LCD Driver: {:?}", defmt::Debug2Format(&error));
                options = options_back;
                Timer::after(Duration::from_millis(100)).await;
            }
            Ok(display) => break display,
        }
    };

    let display_channel = DISPLAY_CHANNEL.init(DisplayChannel::new());
    spawner.must_spawn(display_task(display_channel.receiver(), display));

    // Initialize NeoPixels
    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
    let buffer1 = smart_led_buffer!(10);
    let buffer2 = smart_led_buffer!(10);

    let mut led_strip1 = NeoPixelStrip::<0, BUFFER_SIZE>::new(
        SmartLedsAdapter::new(rmt.channel0, peripherals.GPIO32, buffer1),
        NUM_LEDS,
    );
    let mut led_strip2 = NeoPixelStrip::<1, BUFFER_SIZE>::new(
        SmartLedsAdapter::new(rmt.channel1, peripherals.GPIO33, buffer2),
        NUM_LEDS,
    );

    let _ = led_strip1.off_all();
    let _ = led_strip2.off_all();

    let ledc = mk_static!(Ledc, Ledc::new(peripherals.LEDC));
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

    let lights_channel = LIGHTS_CHANNEL.init(LightsChannel::new());
    spawner.must_spawn(lights_task(lights_channel.receiver(), led_strip1, led_strip2));

    let mut buzzer = Buzzer::new(
        ledc,
        timer::Number::Timer0,
        ledc::channel::Number::Channel1,
        peripherals.GPIO25,
    );

    let sound_channel = SOUND_CHANNEL.init(SoundChannel::new());
    spawner.must_spawn(sound_task(buzzer, sound_channel.receiver()));

    // Initialize event bus
    let event_channel = EVENT_CHANNEL.init(EventChannel::new());
    let event_bus = EventBus::new(event_channel);
    
    // Create task senders
    let task_senders = TaskSenders {
        display: display_channel.sender(),
        lights: lights_channel.sender(),
        sound: sound_channel.sender(),
    };
    
    // Initialize game manager
    let game_manager = GameManager::new(rng);
    
    // Play startup sound and show initial display before spawning tasks
    let _ = task_senders.sound.send(SoundCommand::PlaySong { song: &STARTUP_SOUND }).await;
    let _ = task_senders.display.send(DisplayCommand::WriteText {
        line1: alloc::string::String::from("Airsoft Master"),
        line2: alloc::string::String::from("↓Search & Destroy"),
    }).await;
    
    // Connect to inputs
    let keypad_i2c = I2cDevice::new(i2c_bus);
    let keypad = keypad::I2cKeypad::new(KEYPAD_ADDRESS, keypad_i2c);

    let spi_bus = Spi::new(
        peripherals.SPI2,
        spi::master::Config::default()
            .with_frequency(Rate::from_mhz(5))
            .with_mode(spi::Mode::_0),
    )
    .unwrap()
    .with_sck(peripherals.GPIO18)
    .with_mosi(peripherals.GPIO23)
    .with_miso(peripherals.GPIO19)
    .into_async();

    let spi_bus_blocking = SPI_BUS_BLOCKING.init(BlockingMutex::new(RefCell::new(spi_bus)));

    let sd_cs = Output::new(peripherals.GPIO5, Level::High, OutputConfig::default());
    let spi_device = BlockingSpiDevice::new(spi_bus, sd_cs);
    let spi_interface = mfrc522::comm::blocking::spi::SpiInterface::new(spi_device);
    let mut mfrc522 = match Mfrc522::new(spi_interface).init() {
        Ok(mfrc522) => mfrc522,
        Err(e) => {
            error!("Failed to initialize MFRC522: {:?}", defmt::Debug2Format(&e));
            return;
        }
    };
    
    // Spawn game tasks
    spawner.must_spawn(input_task(keypad, mfrc522, event_bus.event_sender));
    spawner.must_spawn(game_loop_task(game_manager, event_bus.event_receiver, task_senders));
    spawner.must_spawn(timer_task(event_bus.event_sender));
    
    // Keep main task alive
    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}

// Input handling task
#[embassy_executor::task]
async fn input_task(
    mut keypad: keypad::I2cKeypad,
    mut mfrc522: Mfrc522,
    event_sender: embassy_sync::channel::Sender<'static, NoopRawMutex, GameEvent, EVENT_QUEUE_SIZE>,
) {
    loop {
        if let Some(key) = keypad.scan().await {
            let event = match key {
                'A' | 'a' => GameEvent::MenuUp,
                'B' | 'b' => GameEvent::MenuDown,
                '4' => GameEvent::MenuSelect,
                '0'..='9' => {
                    if let Some(digit) = key.to_digit(10) {
                        GameEvent::CodeDigit(digit as u8)
                    } else {
                        continue;
                    }
                },
                _ => continue,
            };
            
            // Convert menu events to game events based on context
            let final_event = match event {
                GameEvent::MenuUp => {
                    // In game modes, 'A' might mean arm
                    GameEvent::GameArm
                },
                GameEvent::MenuDown => {
                    // In game modes, 'B' might mean disarm  
                    GameEvent::GameDisarm
                },
                other => other,
            };
            
            let _ = event_sender.send(final_event).await;
        }
        Timer::after(Duration::from_millis(50)).await;
    }
}

// Game loop task
#[embassy_executor::task]
async fn game_loop_task(
    mut game_manager: GameManager,
    event_receiver: embassy_sync::channel::Receiver<'static, NoopRawMutex, GameEvent, EVENT_QUEUE_SIZE>,
    task_senders: TaskSenders,
) {
    // Initial render of main menu
    let _ = task_senders.display.send(DisplayCommand::WriteText {
        line1: alloc::string::String::from("Airsoft Master"),
        line2: alloc::string::String::from("↓Search & Destroy"),
    }).await;
    
    loop {
        let event = event_receiver.receive().await;
        game_manager.handle_event(event, &task_senders).await;
    }
}

// Timer task for game countdown
#[embassy_executor::task]
async fn timer_task(
    event_sender: embassy_sync::channel::Sender<'static, NoopRawMutex, GameEvent, EVENT_QUEUE_SIZE>,
) {
    loop {
        Timer::after(Duration::from_secs(1)).await;
        let _ = event_sender.send(GameEvent::TimerTick).await;
    }
}
