use defmt::{error, info};
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::{Duration, Timer};
use ector::{Actor, ActorAddress, DynamicAddress, Inbox};
use esp_hal::gpio::Output;
use esp_hal::spi::master::Spi;
use esp_hal::Async;
use pn532::{spi::SPIInterface, Pn532, Request, requests::SAMMode};
use core::future::Future;

use crate::events::InputEvent;

// Timer wrapper that implements CountDown for PN532
pub struct EmbassyTimer;

impl pn532::CountDown for EmbassyTimer {
    type Time = embassy_time::Duration;
    
    fn start<T>(&mut self, _timeout: T)
    where
        T: Into<Self::Time>,
    {
        // Embassy timer doesn't need explicit start - we'll use it in until_timeout
    }
    
    fn until_timeout<F: Future>(
        &self,
        fut: F,
    ) -> impl core::future::Future<Output = Result<F::Output, embassy_time::TimeoutError>> {
        // Use embassy_time::with_timeout to wrap the future with a timeout
        // For now, use a default timeout of 1 second - in a real implementation
        // you might want to store the timeout from the start() method
        embassy_time::with_timeout(Duration::from_secs(1), fut)
    }
}

type Pn532Type = Pn532<SPIInterface<SpiDevice<'static, NoopRawMutex, Spi<'static, Async>, Output<'static>>>, EmbassyTimer, 32>;

pub struct NfcActor {
    pn532: Pn532Type,
    app_address: DynamicAddress<InputEvent>,
}

impl NfcActor {
    pub fn new(
        pn532: Pn532Type,
        app_address: DynamicAddress<InputEvent>,
    ) -> Self {
        Self { pn532, app_address }
    }
}

impl Actor for NfcActor {
    type Message = !;

    async fn on_mount<M>(&mut self, _: DynamicAddress<Self::Message>, _inbox: M) -> !
    where
        M: Inbox<Self::Message>,
    {
        // Initialize the PN532
        info!("Initializing PN532...");
        
        // Configure SAM (Secure Access Module)
        if let Err(e) = self.pn532.process(&Request::sam_configuration(SAMMode::Normal, false), 0, Duration::from_millis(50)).await {
            error!("Failed to configure PN532 SAM: {:?}", defmt::Debug2Format(&e));
            // In an actor, we cannot return, so we loop forever.
            loop { Timer::after(Duration::from_secs(1)).await; }
        }
        
        // Get firmware version
        match self.pn532.process(&Request::GET_FIRMWARE_VERSION, 5, Duration::from_millis(50)).await {
            Ok(version) => {
                info!("PN532 Firmware version: {:02x}.{:02x}", version[0], version[1]);
            }
            Err(e) => {
                error!("Failed to get PN532 firmware version: {:?}", defmt::Debug2Format(&e));
                loop { Timer::after(Duration::from_secs(1)).await; }
            }
        }
        
        info!("PN532 initialized successfully");

        loop {
            // Poll for ISO14443A cards with enhanced MIFARE Classic support
            match self.pn532.process(&Request::INLIST_ONE_ISO_A_TARGET, 10, Duration::from_millis(500)).await {
                Ok(response) => {
                    // Check if we have a valid response
                    if !response.is_empty() {
                        let num_targets = response[0];
                        if num_targets > 0 && response.len() >= 6 {
                            info!("Card detected!");
                            let _sel_res = response[4]; // SEL_RES (SAK)
                            let uid_len = response[5] as usize;

                            if response.len() >= 6 + uid_len && uid_len > 0 {
                                let uid = &response[6..6 + uid_len];
                                info!("Card UID: {:?}", uid);

                                // Send event to the game system
                                self.app_address.notify(InputEvent::CardDetected).await;

                                // Prevent rapid re-detection of the same card
                                Timer::after(Duration::from_millis(2000)).await;
                            } else {
                                info!("Invalid UID length or missing UID data");
                            }
                        }
                    }
                }
                Err(_) => {
                    Timer::after(Duration::from_millis(200)).await;
                }
            }
            
            // Small delay to yield to other tasks
            Timer::after(Duration::from_millis(50)).await;
        }
    }
}


