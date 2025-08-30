use defmt::{error, info};
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::Output;
use esp_hal::spi::master::Spi;
use esp_hal::Async;
use pn532::{spi::SPIInterface, Pn532, Request, requests::SAMMode};
use core::future::Future;

use crate::events::{InputEvent, EVENT_QUEUE_SIZE};

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

#[embassy_executor::task]
pub async fn nfc_task(
    mut pn532: Pn532Type,
    event_sender: embassy_sync::channel::Sender<
        'static,
        NoopRawMutex,
        InputEvent,
        { EVENT_QUEUE_SIZE },
    >,
) {
    // Initialize the PN532
    info!("Initializing PN532...");
    
    // Configure SAM (Secure Access Module)
    if let Err(e) = pn532.process(&Request::sam_configuration(SAMMode::Normal, false), 0, Duration::from_millis(50)).await {
        error!("Failed to configure PN532 SAM: {:?}", defmt::Debug2Format(&e));
        return;
    }
    
    // Get firmware version
    match pn532.process(&Request::GET_FIRMWARE_VERSION, 5, Duration::from_millis(50)).await {
        Ok(version) => {
            info!("PN532 Firmware version: {:02x}.{:02x}", version[0], version[1]);
        }
        Err(e) => {
            error!("Failed to get PN532 firmware version: {:?}", defmt::Debug2Format(&e));
            return;
        }
    }
    
    info!("PN532 initialized successfully");

    loop {
        // Poll for ISO14443A cards with enhanced MIFARE Classic support
        match pn532.process(&Request::INLIST_ONE_ISO_A_TARGET, 10, Duration::from_millis(500)).await {
            Ok(response) => {
                info!("Raw response length: {}, data: {:?}", response.len(), defmt::Debug2Format(&response));
                
                // Check if we have a valid response
                if !response.is_empty() {
                    let num_targets = response[0];
                    info!("Number of targets found: {}", num_targets);
                    
                    if num_targets > 0 && response.len() >= 4 {
                        info!("Card detected!");
                        
                        // Parse target information - format varies by card type
                        if response.len() >= 6 {
                            let tg = response[1]; // Target number
                            let sens_res = [response[2], response[3]]; // SENS_RES (ATQA)
                            let sel_res = response[4]; // SEL_RES (SAK)
                            let uid_len = response[5] as usize;
                            
                            info!("Target: {}, SENS_RES: {:02x}{:02x}, SEL_RES: {:02x}, UID len: {}", 
                                  tg, sens_res[0], sens_res[1], sel_res, uid_len);
                            
                            // Extract UID if present
                            if response.len() >= 6 + uid_len && uid_len > 0 {
                                let uid = &response[6..6 + uid_len];
                                info!("Card UID: {:?}", uid);
                                
                                // Check if this is a MIFARE Classic card (common SAK values: 0x08, 0x09, 0x18, 0x19)
                                match sel_res {
                                    0x08 => info!("MIFARE Classic 1K detected"),
                                    0x09 => info!("MIFARE Classic Mini detected"),
                                    0x18 => info!("MIFARE Classic 4K detected"),
                                    0x19 => info!("MIFARE Classic 2K detected"),
                                    0x20 => info!("MIFARE Plus/DESFire detected"),
                                    _ => info!("Unknown card type, SAK: 0x{:02x}", sel_res),
                                }
                                
                                // Send event to the game system
                                event_sender.send(InputEvent::CardDetected).await;
                                
                                // Prevent rapid re-detection of the same card
                                Timer::after(Duration::from_millis(2000)).await;
                            } else {
                                info!("Invalid UID length or missing UID data");
                            }
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


