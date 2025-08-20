#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use airsoft_v2::app::main_menu::MainMenu;
use airsoft_v2::app::search_and_destroy::SearchAndDestroy;
use airsoft_v2::app::App;
use airsoft_v2::devices::buzzer::STARTUP_SOUND;
use airsoft_v2::devices::neopixel::NeoPixelStrip;
use airsoft_v2::events::Command;
use airsoft_v2::events::{EventBus, EventChannel, InputEvent, TaskSenders, EVENT_QUEUE_SIZE};
use airsoft_v2::tasks::input::{keypad::keypad_task, nfc::nfc_task};
use airsoft_v2::tasks::internal::game_ticker_task;
use airsoft_v2::tasks::output::display::ScrollDirection;
use airsoft_v2::tasks::output::lights::LightsCommand;
use airsoft_v2::tasks::output::{
    display::{display_task, DisplayChannel, DisplayCommand},
    lights::{lights_task, LightsChannel},
    sound::{sound_task, SoundChannel, SoundCommand},
};
use airsoft_v2::web::{self, WebApp};
use airsoft_v2::wifi::{dhcp_server, start_wifi};
use airsoft_v2::{devices::keypad, mk_static, game_state};

use alloc::boxed::Box;
use alloc::string::ToString;
use defmt::{error, info};
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embassy_time::{Delay, Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::i2c::master::I2c;
use esp_hal::ledc::{self, timer, LSGlobalClkSource, Ledc};
use esp_hal::rmt::Rmt;
use esp_hal::spi;
use esp_hal::spi::master::Spi;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{i2c, Async};
use esp_hal_buzzer::Buzzer;
use esp_hal_mfrc522::MFRC522;
use esp_hal_smartled::{buffer_size, smart_led_buffer, SmartLedsAdapter};
use esp_println as _;
use esp_wifi::EspWifiController;
use hd44780_driver::memory_map::MemoryMap1602;
use hd44780_driver::non_blocking::HD44780;
use hd44780_driver::setup::DisplayOptionsI2C;
use static_cell::StaticCell;

use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

const NUM_LEDS: usize = 9;
const BUFFER_SIZE: usize = buffer_size(NUM_LEDS);

type I2cType = I2c<'static, esp_hal::Async>;

const LCD_ADDRESS: u8 = 0x27; // or 0x3F
const KEYPAD_ADDRESS: u8 = 0x20; // or 0x21-0x27

static I2C_BUS: StaticCell<Mutex<NoopRawMutex, I2cType>> = StaticCell::new();
static SPI_BUS: StaticCell<Mutex<NoopRawMutex, Spi<'static, Async>>> = StaticCell::new();
static EVENT_CHANNEL: StaticCell<EventChannel> = StaticCell::new();
static LIGHTS_CHANNEL: StaticCell<LightsChannel> = StaticCell::new();
static SOUND_CHANNEL: StaticCell<SoundChannel> = StaticCell::new();
static DISPLAY_CHANNEL: StaticCell<DisplayChannel> = StaticCell::new();

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

    info!("Attempting to start wifi..");
    let stack = start_wifi(esp_wifi_ctrl, peripherals.WIFI, rng, &spawner)
        .await
        .expect("Failed to start wifi");


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
    let mut options =
        DisplayOptionsI2C::new(MemoryMap1602::new()).with_i2c_bus(lcd_i2c, LCD_ADDRESS);

    let display = loop {
        match HD44780::new(options, &mut Delay).await {
            Err((options_back, error)) => {
                error!(
                    "Error creating LCD Driver: {:?}",
                    defmt::Debug2Format(&error)
                );
                options = options_back;
                Timer::after(Duration::from_millis(100)).await;
            }
            Ok(display) => break display,
        }
    };

    let display_channel = DISPLAY_CHANNEL.init(DisplayChannel::new());
    spawner.must_spawn(display_task(display_channel.receiver(), display));

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
        display: display_channel.sender(),
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
    task_senders
        .display
        .send(DisplayCommand::ScrollText {
            text: "Airsoft".to_string(),
            col: 0,
            times: 2,
            direction: ScrollDirection::Right,
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
    .with_mosi(peripherals.GPIO23)
    .with_miso(peripherals.GPIO19)
    .into_async();

    let cs = Output::new(peripherals.GPIO5, Level::High, OutputConfig::default());
    let spi_bus = SPI_BUS.init(Mutex::new(spi));
    let spi_device = SpiDevice::new(spi_bus, cs);

    let mfrc522 = MFRC522::new(spi_device, || embassy_time::Instant::now().as_ticks());

    // Spawn input tasks
    spawner.must_spawn(keypad_task(keypad, event_bus.event_sender));
    spawner.must_spawn(nfc_task(mfrc522, event_bus.event_sender));

    // Spawn game ticker task
    spawner.must_spawn(game_ticker_task(event_bus.event_sender));

    info!("All Side tasks spawned!");
    
    // Initialize shared game state for web API
    game_state::init_game_state();
    info!("Game state initialized!");
    
    // Start web server and DHCP server after game state is initialized
    let webapp = WebApp::default();
    spawner.must_spawn(web::web_task(0, stack, webapp.router, webapp.config));
    spawner.must_spawn(dhcp_server(stack));
    info!("Web server started!");
    
    // Keep main task alive
    info!("Initiating main task loop");

    let mut current_app: Box<dyn App> = Box::new(MainMenu::default());

    loop {
        // TODO we can make this follow a state machine pattern or component pattern to make it more extensible
        // We only need to figure out sound and future output devices like servos, etc

        // Update game state for web API based on current app
        // This is a bit of a hack since we need to check the concrete type
        if let Some(main_menu) = current_app.as_any().downcast_ref::<MainMenu>() {
            let selection = match main_menu.current_selection {
                airsoft_v2::app::main_menu::MainMenuSelection::SearchAndDestroy => "search_and_destroy",
                airsoft_v2::app::main_menu::MainMenuSelection::TeamDeathMatch => "team_death_match",
                airsoft_v2::app::main_menu::MainMenuSelection::Domination => "domination",
                airsoft_v2::app::main_menu::MainMenuSelection::Cashout => "cashout",
                airsoft_v2::app::main_menu::MainMenuSelection::Config => "config",
            };
            game_state::update_main_menu_state(selection, main_menu.has_selected).await;
        } else if let Some(sad) = current_app.as_any().downcast_ref::<SearchAndDestroy>() {
            let stage = match sad.stage {
                airsoft_v2::app::search_and_destroy::Stage::WaitingForArm => "waiting_for_arm",
                airsoft_v2::app::search_and_destroy::Stage::Arming => "arming",
                airsoft_v2::app::search_and_destroy::Stage::Armed => "armed",
                airsoft_v2::app::search_and_destroy::Stage::Ticking => "ticking",
                airsoft_v2::app::search_and_destroy::Stage::Exploded => "exploded",
                airsoft_v2::app::search_and_destroy::Stage::Disarming => "disarming",
                airsoft_v2::app::search_and_destroy::Stage::Disarmed => "disarmed",
            };
            game_state::update_search_and_destroy_state(
                sad.time_left,
                stage,
                sad.current_code.len() as u8,
                sad.wants_game_tick,
            ).await;
        }

        // Dont trigger game tick events if the app doesn't need them
        let commands = current_app.render();
        for command in commands {
            match command {
                Command::DisplayText(display_command) => {
                    task_senders.display.send(display_command).await;
                }
                Command::Lights(lights_command) => {
                    task_senders.lights.send(lights_command).await;
                }
                Command::Sound(sound_command) => {
                    task_senders.sound.send(sound_command).await;
                }
                Command::ChangeApp(app) => {
                    current_app = app;
                    // Send a None event to the app to trigger the first render
                    // This is a bit of a hack, but it works
                    event_bus.event_sender.send(InputEvent::None).await;
                }
                Command::Noop => {}
            }
        }

        let event = event_bus.event_receiver.receive().await;
        current_app.handle_event(event);
    }
}
