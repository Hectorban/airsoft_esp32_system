use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use esp_hal::Async;
use esp_hal::gpio::Output;
use esp_hal::spi::master::Spi;
use esp_hal_mfrc522::{consts::UidSize, MFRC522};
use defmt::{error, info};

use crate::events::{InputEvent, EVENT_QUEUE_SIZE};

#[embassy_executor::task]
pub async fn nfc_task(
    mut mfrc522: MFRC522<SpiDevice<'static, NoopRawMutex, Spi<'static, Async>, Output<'static>>>,
    event_sender: embassy_sync::channel::Sender<'static, NoopRawMutex, InputEvent, {EVENT_QUEUE_SIZE}>,
) {
    // Initialize the MFRC522
    if let Err(e) = mfrc522.pcd_init().await {
        error!("Failed to initialize MFRC522: {:?}", defmt::Debug2Format(&e));
        return;
    }

    // Run self-test
    if let Err(e) = mfrc522.pcd_selftest().await {
        error!("MFRC522 self-test failed: {:?}", defmt::Debug2Format(&e));
    }

    info!("PCD version: {:?}", defmt::Debug2Format(&mfrc522.pcd_get_version().await));

    if !mfrc522.pcd_is_init().await {
        error!("MFRC522 init failed! Try to power cycle the module!");
        return;
    }

    info!("MFRC522 initialized successfully");

    loop {
        // Check for new card present (non-blocking)
        if mfrc522.picc_is_new_card_present().await.is_ok() {
            info!("Card detected, reading UID...");
            
            // Try to get the card with 4-byte UID
            match mfrc522.get_card(UidSize::Four).await {
                Ok(card) => {
                    let uid_number = card.get_number();
                    info!("Card UID: {}", uid_number);
                    print_uid_number(uid_number);
                    
                    // Send event to the game system
                    event_sender.send(InputEvent::CardDetected).await;
                    
                    // Halt the card to prevent continuous reading
                    let _ = mfrc522.picc_halta().await;
                    
                    // Prevent rapid re-detection of the same card
                    Timer::after(Duration::from_millis(1000)).await;
                }
                Err(e) => {
                    error!("Failed to read card: {:?}", defmt::Debug2Format(&e));
                    let _ = mfrc522.picc_halta().await;
                }
            }
        }
        
        // Small delay to yield to other tasks
        Timer::after(Duration::from_millis(1)).await;
    }
}

fn print_uid_number(uid_number: u128) {
    info!("Card UID: 0x{:016x}\n", defmt::Debug2Format(&uid_number));
}
