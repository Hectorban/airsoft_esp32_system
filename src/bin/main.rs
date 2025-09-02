#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use airsoft_v2::app::App;
use airsoft_v2::devices::neopixel::NeoPixelStrip;
use airsoft_v2::events::{InputEvent, TaskSenders};
use airsoft_v2::tasks::input::keypad::KeypadActor;
use airsoft_v2::tasks::input::nfc::NfcActor;
use airsoft_v2::tasks::output::lights::{LightsActor, LightsCommand};
use airsoft_v2::tasks::output::sound::{SoundActor, SoundCommand};
use airsoft_v2::tasks::rng::{RngActor, RngRequest};
use airsoft_v2::tasks::ticker::TickerActor;
use airsoft_v2::tasks::web::{self, WebApp};
use airsoft_v2::tasks::wifi::{dhcp_server, start_wifi};
use airsoft_v2::{devices::keypad, game_state, mk_static};
use ector::mutex::NoopRawMutex as EctorNoopRawMutex;
use ector::{ActorContext, actor};

use airsoft_v2::tasks::input::nfc::EmbassyTimer;
use alloc::boxed::Box;
use core::cell::RefCell;
use defmt::{error, info};
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::{Mutex as BlockingMutex, raw::NoopRawMutex};
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::i2c::master::I2c;
use esp_hal::ledc::{self, LSGlobalClkSource, Ledc, timer};
use esp_hal::rmt::Rmt;
use esp_hal::spi;
use esp_hal::spi::master::Spi;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{Async, i2c};
use esp_hal_buzzer::Buzzer;
use esp_hal_smartled::{SmartLedsAdapter, buffer_size, smart_led_buffer};
use esp_println as _;
use esp_wifi::EspWifiController;
use mousefood::{EmbeddedBackend, EmbeddedBackendConfig};
use pn532::{
    Pn532,
    spi::{NoIRQ, SPIInterface},
};
use ratatui::widgets::{Block, Paragraph, Wrap};
use ratatui::{Frame, Terminal, style::*};
use ssd1306::prelude::I2CInterface as SsdI2CInterface;
use ssd1306::{Ssd1306, prelude::*};
use static_cell::StaticCell;

use embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice;

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

const NUM_LEDS: usize = 9;
const BUFFER_SIZE: usize = buffer_size(NUM_LEDS);

pub type I2cType = I2c<'static, esp_hal::Blocking>;

pub type OledDisplayType<'a> = Ssd1306<
    SsdI2CInterface<I2cDevice<'a, NoopRawMutex, I2cType>>,
    DisplaySize128x64,
    ssd1306::mode::BufferedGraphicsMode<DisplaySize128x64>,
>;

const OLED_ADDRESS: u8 = 0x3C; // Standard SSD1306 I2C address
const KEYPAD_ADDRESS: u8 = 0x20; // or 0x21-0x27

static I2C_BUS: StaticCell<BlockingMutex<NoopRawMutex, RefCell<I2cType>>> = StaticCell::new();
static SPI_BUS: StaticCell<Mutex<NoopRawMutex, Spi<'static, Async>>> = StaticCell::new();
static DISPLAY: StaticCell<OledDisplayType<'static>> = StaticCell::new();

// Actor contexts following the circular reference pattern
static APP_CONTEXT: ActorContext<App> = ActorContext::new();

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 64 * 1024);
    //esp_alloc::heap_allocator!(#[unsafe(link_section = ".dram2_uninit")] size: 64 * 1024);

    let timer0 = TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timer0.timer0);
    info!("Embassy initialized!");

    let rng = esp_hal::rng::Rng::new(peripherals.RNG);
    // let timer1 = TimerGroup::new(peripherals.TIMG0);
    // let esp_wifi_ctrl = mk_static!(
    //     EspWifiController<'static>,
    //     esp_wifi::init(timer1.timer0, rng).unwrap()
    // );

    // info!("Attempting to start wifi..");
    // let stack = start_wifi(esp_wifi_ctrl, peripherals.WIFI, rng, &spawner)
    //     .await
    //     .expect("Failed to start wifi");

    // let webapp = WebApp::default();
    // spawner.must_spawn(web::web_task(0, stack, webapp.router, webapp.config));
    // spawner.must_spawn(dhcp_server(stack));

    // info!("Web server started!");

    let i2c = I2c::new(
        peripherals.I2C0,
        i2c::master::Config::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO21)
    .with_scl(peripherals.GPIO22);
    let i2c_bus = I2C_BUS.init(BlockingMutex::new(RefCell::new(i2c)));

    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
    let buffer1 = smart_led_buffer!(NUM_LEDS);

    let mut led_strip = NeoPixelStrip::<0, BUFFER_SIZE>::new(
        SmartLedsAdapter::new(rmt.channel0, peripherals.GPIO4, buffer1),
        NUM_LEDS,
    );

    let lights_addr: ector::DynamicAddress<LightsCommand> = actor!(spawner, lights, LightsActor<BUFFER_SIZE>, LightsActor::new(led_strip), EctorNoopRawMutex).into();

    let ledc = mk_static!(Ledc, Ledc::new(peripherals.LEDC));
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);
    let buzzer = Buzzer::new(
        ledc,
        timer::Number::Timer0,
        ledc::channel::Number::Channel1,
        peripherals.GPIO25,
    );

    let sound_addr: ector::DynamicAddress<SoundCommand> = actor!(spawner, sound, SoundActor, SoundActor::new(buzzer), EctorNoopRawMutex).into();

    let rng_addr: ector::Address<RngRequest, EctorNoopRawMutex> = actor!(spawner, rng_actor, RngActor, RngActor::new(rng), EctorNoopRawMutex).into();

    let task_senders = TaskSenders {
        lights: lights_addr,
        sound: sound_addr,
        rng: rng_addr,
    };

    // Create the App actor address for circular references
    let app_addr = APP_CONTEXT.dyn_address();

    let keypad_i2c = I2cDevice::new(i2c_bus);
    let keypad = keypad::I2cKeypad::new(KEYPAD_ADDRESS, keypad_i2c);

    let spi = Spi::new(
        peripherals.SPI2,
        spi::master::Config::default()
            .with_frequency(Rate::from_mhz(5))
            .with_mode(spi::Mode::_0),
    )
    .unwrap()
    .with_sck(peripherals.GPIO18)
    .with_miso(peripherals.GPIO19)
    .with_mosi(peripherals.GPIO23)
    .into_async();

    let cs = Output::new(peripherals.GPIO5, Level::High, OutputConfig::default());
    let spi_bus = SPI_BUS.init(Mutex::new(spi));
    let spi_device = SpiDevice::new(spi_bus, cs);

    let pn532 = Pn532::new(
        SPIInterface {
            spi: spi_device,
            irq: None::<NoIRQ>,
        },
        EmbassyTimer,
    );

    // Spawn input tasks with direct communication to App
    actor!(
        spawner,
        keypad_actor,
        KeypadActor,
        KeypadActor::new(keypad, app_addr.clone()),
        EctorNoopRawMutex
    );
    actor!(
        spawner,
        nfc,
        NfcActor,
        NfcActor::new(pn532, app_addr.clone()),
        EctorNoopRawMutex
    );
    actor!(
        spawner,
        ticker,
        TickerActor,
        TickerActor::new(app_addr.clone()),
        EctorNoopRawMutex
    );

    game_state::init_game_state();
    info!("Game state initialized!");

    let display_temp = loop {
        let oled_i2c = I2cDevice::new(i2c_bus);
        let interface = SsdI2CInterface::new(oled_i2c, OLED_ADDRESS, 0x40);

        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();

        match display.init() {
            Err(error) => {
                error!(
                    "Error creating OLED Driver: {:?}",
                    defmt::Debug2Format(&error)
                );
                Timer::after(Duration::from_millis(100)).await;
            }
            Ok(()) => break display,
        }
    };
    let display = DISPLAY.init(display_temp);
    airsoft_v2::graphics::boot_animation(display).await;

    let config = EmbeddedBackendConfig {
        flush_callback: Box::new(move |d: &mut OledDisplayType<'_>| {
            d.flush().unwrap();
        }),
        ..Default::default()
    };

    let backend = EmbeddedBackend::new(display, config);
    let terminal = Terminal::new(backend).unwrap();

    info!("Initiating main task loop");

    // Create and mount the App actor following the circular reference pattern
    let mut app = App::new(task_senders);
    app.set_terminal(terminal);

    // Mount the App actor - this starts the main loop
    let app_future = APP_CONTEXT.mount(app);
    app_future.await;
}
