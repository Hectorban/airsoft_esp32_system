use embassy_time::{Duration, Timer};
use esp_wifi::ble::controller::BleConnector;
use defmt::info;
use crate::game_state;

// Custom UUID for our airsoft game service
// Using a custom 128-bit UUID: 12345678-1234-5678-9abc-123456789abc
const AIRSOFT_SERVICE_UUID: [u8; 16] = [
    0x12, 0x34, 0x56, 0x78, 0x12, 0x34, 0x56, 0x78,
    0x9a, 0xbc, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc,
];

// Characteristic UUID for game state (incrementing the last byte)
const GAME_STATE_CHAR_UUID: [u8; 16] = [
    0x12, 0x34, 0x56, 0x78, 0x12, 0x34, 0x56, 0x78,
    0x9a, 0xbc, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbd,
];

pub const BLE_TASK_POOL_SIZE: usize = 1;

#[embassy_executor::task(pool_size = BLE_TASK_POOL_SIZE)]
pub async fn ble_task(
    _id: usize,
    _connector: BleConnector<'static>,
) -> ! {
    info!("Starting BLE task");

    // For now, create a simple task that periodically logs the game state
    // This replaces the web server functionality by providing visibility into the game state
    // In the future, this can be extended to use trouble-host for full BLE GATT services
    
    let mut last_game_state = game_state::GameState::default();
    
    loop {
        Timer::after(Duration::from_secs(2)).await;
        
        let current_state = game_state::get_current_state().await;
        if !game_states_equal(&last_game_state, &current_state) {
            info!("Game state changed: in_game={}, current_game={}", 
                current_state.is_in_game, 
                current_state.current_game.as_str()
            );
            
            // Log specific game data based on the variant
            match &current_state.game_data {
                game_state::GameData::MainMenu { selection, has_selected } => {
                    info!("MainMenu: selection={}, has_selected={}", 
                        selection.as_str(), has_selected);
                }
                game_state::GameData::SearchAndDestroy { time_left, stage, code_length, wants_game_tick } => {
                    info!("SearchAndDestroy: time_left={}, stage={}, code_length={}, wants_game_tick={}", 
                        time_left, stage.as_str(), code_length, wants_game_tick);
                }
            }
            
            last_game_state = current_state;
        }
    }
}

fn game_states_equal(a: &game_state::GameState, b: &game_state::GameState) -> bool {
    a.is_in_game == b.is_in_game 
        && a.current_game == b.current_game
        && match (&a.game_data, &b.game_data) {
            (
                game_state::GameData::MainMenu { selection: sel_a, has_selected: has_a },
                game_state::GameData::MainMenu { selection: sel_b, has_selected: has_b }
            ) => sel_a == sel_b && has_a == has_b,
            (
                game_state::GameData::SearchAndDestroy { 
                    time_left: time_a, 
                    stage: stage_a, 
                    code_length: code_a, 
                    wants_game_tick: tick_a 
                },
                game_state::GameData::SearchAndDestroy { 
                    time_left: time_b, 
                    stage: stage_b, 
                    code_length: code_b, 
                    wants_game_tick: tick_b 
                }
            ) => time_a == time_b && stage_a == stage_b && code_a == code_b && tick_a == tick_b,
            _ => false,
        }
}
