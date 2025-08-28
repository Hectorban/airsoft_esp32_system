#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use airsoft_v2::app::App;
use airsoft_v2::devices::buzzer::STARTUP_SOUND;
use airsoft_v2::devices::neopixel::NeoPixelStrip;
use airsoft_v2::events::{EventBus, EventChannel, InputEvent, TaskSenders, EVENT_QUEUE_SIZE};
use airsoft_v2::tasks::input::{keypad::keypad_task, nfc::nfc_task};
use airsoft_v2::tasks::internal::game_ticker_task;
use airsoft_v2::tasks::output::lights::LightsCommand;
use airsoft_v2::tasks::output::{
    lights::{lights_task, LightsChannel},
    sound::{sound_task, SoundChannel, SoundCommand},
};
use airsoft_v2::web::{self, WebApp};
use airsoft_v2::wifi::{dhcp_server, start_wifi};
use airsoft_v2::{devices::keypad, game_state, mk_static};

use alloc::boxed::Box;
use alloc::string::ToString;
use defmt::{error, info};
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embassy_time::{Delay, Duration, Timer};
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle};
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::i2c::master::I2c;
use esp_hal::ledc::{self, timer, LSGlobalClkSource, Ledc};
use esp_hal::rmt::Rmt;
use esp_hal::spi;
use esp_hal::spi::master::Spi;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{i2c, Blocking};
use esp_hal_buzzer::Buzzer;
use mousefood::prelude::Rgb565;
use mousefood::{EmbeddedBackend, EmbeddedBackendConfig};
use pn532::{spi::{SPIInterface, NoIRQ}, Pn532};
use airsoft_v2::tasks::input::nfc::EmbassyTimer;
use esp_hal_smartled::{buffer_size, smart_led_buffer, SmartLedsAdapter};
use esp_println as _;
use esp_wifi::EspWifiController;
use ratatui::widgets::{Block, Paragraph, Wrap};
use ratatui::{Frame, Terminal, style::*};
use ssd1306::{prelude::*, Ssd1306, Ssd1306Async};
use ssd1306::prelude::I2CInterface as SsdI2CInterface;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use static_cell::StaticCell;

use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

const NUM_LEDS: usize = 9;
const BUFFER_SIZE: usize = buffer_size(NUM_LEDS);

type I2cType = I2c<'static, esp_hal::Blocking>;

const OLED_ADDRESS: u8 = 0x3C; // Standard SSD1306 I2C address
const KEYPAD_ADDRESS: u8 = 0x20; // or 0x21-0x27

static I2C_BUS: StaticCell<Mutex<NoopRawMutex, I2cType>> = StaticCell::new();
static SPI_BUS: StaticCell<Mutex<NoopRawMutex, Spi<'static, Async>>> = StaticCell::new();
static EVENT_CHANNEL: StaticCell<EventChannel> = StaticCell::new();
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
    //let esp_wifi_ctrl = mk_static!(
    //    EspWifiController<'static>,
    //    esp_wifi::init(timer1.timer0, rng).unwrap()
    //);

    //info!("Attempting to start wifi..");
    //let stack = start_wifi(esp_wifi_ctrl, peripherals.WIFI, rng, &spawner)
    //    .await
    //    .expect("Failed to start wifi");

    //let webapp = WebApp::default();
    //spawner.must_spawn(web::web_task(0, stack, webapp.router, webapp.config));
    //spawner.must_spawn(dhcp_server(stack));
    //info!("Web server started!");

    // TODO Abstract spawning of devices

    info!("Attempting to start I2C bus..");
    let i2c = I2c::new(
        peripherals.I2C0,
        i2c::master::Config::default().with_frequency(Rate::from_khz(100)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO21)
    .with_scl(peripherals.GPIO22);
    let i2c_bus = I2C_BUS.init(Mutex::new(i2c));


    info!("Connecting to neopixel strips..");
    // Initialize NeoPixels
    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
    let buffer1 = smart_led_buffer!(NUM_LEDS);
    let buffer2 = smart_led_buffer!(NUM_LEDS);

    let mut led_strip1 = NeoPixelStrip::<0, BUFFER_SIZE>::new(
        SmartLedsAdapter::new(rmt.channel0, peripherals.GPIO4, buffer1),
        NUM_LEDS,
    );
    let mut led_strip2 = NeoPixelStrip::<1, BUFFER_SIZE>::new(
        SmartLedsAdapter::new(rmt.channel1, peripherals.GPIO2, buffer2),
        NUM_LEDS,
    );

    let _ = led_strip1.off_all();
    let _ = led_strip2.off_all();

    let ledc = mk_static!(Ledc, Ledc::new(peripherals.LEDC));
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

    let lights_channel = LIGHTS_CHANNEL.init(LightsChannel::new());
    spawner.must_spawn(lights_task(
        lights_channel.receiver(),
        led_strip1,
        led_strip2,
    ));

    let buzzer = Buzzer::new(
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
        lights: lights_channel.sender(),
        sound: sound_channel.sender(),
    };

    // Play startup sound and show initial display before spawning tasks
    task_senders
        .sound
        .send(SoundCommand::PlaySong {
            song: &STARTUP_SOUND,
        })
        .await;

    task_senders
        .lights
        .send(LightsCommand::Flash {
            r: 255,
            g: 255,
            b: 255,
            duration_ms: 100,
        })
        .await;

    // give time for the animation to finish
    Timer::after(Duration::from_secs(4)).await;

    // Connect to inputs
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

    let mut pn532 = Pn532::new(
        SPIInterface {
            spi: spi_device,
            irq: None::<NoIRQ>,
        },
        EmbassyTimer,
    );

    // Spawn input tasks
    spawner.must_spawn(keypad_task(keypad, event_bus.event_sender));
    spawner.must_spawn(nfc_task(pn532, event_bus.event_sender));

    // Spawn game ticker task
    spawner.must_spawn(game_ticker_task(event_bus.event_sender));

    info!("All Side tasks spawned!");

    // Initialize shared game state for web AP
    game_state::init_game_state();
    info!("Game state initialized!");

    // Keep main task alive
    info!("Initiating main task loop");

    let mut display= loop {
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

    let config = EmbeddedBackendConfig {
        flush_callback: Box::new(
            move |d| {
                d.flush().unwrap();
            },
        ),
        ..Default::default()
    };

    let backend = EmbeddedBackend::new(&mut display, config);
    let mut terminal = Terminal::new(backend).unwrap();

    loop {
        terminal.draw(draw).unwrap();
    }
}

fn draw(frame: &mut Frame) {
    let text = "Ratatui on embedded devices!";
    let paragraph = Paragraph::new(text.dark_gray()).wrap(Wrap { trim: true });
    let bordered_block = Block::bordered()
        .border_style(Style::new().yellow())
        .title("Mousefood");
    frame.render_widget(paragraph.block(bordered_block), frame.area());
}
