extern crate alloc;

use alloc::string::String;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct GameState {
    pub is_in_game: bool,
    pub current_game: String,
    pub game_data: GameData,
}

#[derive(Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum GameData {
    MainMenu {
        selection: String,
        has_selected: bool,
    },
    SearchAndDestroy {
        time_left: u32,
        stage: String,
        code_length: u8,
        wants_game_tick: bool,
    },
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            is_in_game: false,
            current_game: String::from("main_menu"),
            game_data: GameData::MainMenu {
                selection: String::from("search_and_destroy"),
                has_selected: false,
            },
        }
    }
}

use embassy_sync::once_lock::OnceLock;

static GAME_STATE_MUTEX: OnceLock<Mutex<NoopRawMutex, GameState>> = OnceLock::new();

pub fn init_game_state() {
    GAME_STATE_MUTEX
        .init(Mutex::new(GameState::default()))
        .unwrap();
}

pub async fn update_main_menu_state(selection: &str, has_selected: bool) {
    let mutex = GAME_STATE_MUTEX.get().await;
    let mut game_state = mutex.lock().await;
    game_state.is_in_game = false;
    game_state.current_game = String::from("main_menu");
    game_state.game_data = GameData::MainMenu {
        selection: String::from(selection),
        has_selected,
    };
}

pub async fn update_search_and_destroy_state(
    time_left: u32,
    stage: &str,
    code_length: u8,
    wants_game_tick: bool,
) {
    let mutex = GAME_STATE_MUTEX.get().await;
    let mut game_state = mutex.lock().await;
    game_state.is_in_game = true;
    game_state.current_game = String::from("search_and_destroy");
    game_state.game_data = GameData::SearchAndDestroy {
        time_left,
        stage: String::from(stage),
        code_length,
        wants_game_tick,
    };
}

pub async fn get_current_state() -> GameState {
    let mutex = GAME_STATE_MUTEX.get().await;
    let game_state = mutex.lock().await;
    game_state.clone()
}
