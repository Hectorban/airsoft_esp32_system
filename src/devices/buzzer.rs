use esp_hal_buzzer::{song, ToneValue, notes::*};

pub const STARTUP_SOUND: [ToneValue; 7] = song!(
    180, // Tempo in beats per minute
    [
        (NOTE_FS6, SIXTEENTH_NOTE),
        (NOTE_B5, DOTTED_EIGHTEENTH_NOTE),
        (REST, SIXTEENTH_NOTE),
        (NOTE_FS6, SIXTEENTH_NOTE),
        (NOTE_B5, DOTTED_EIGHTEENTH_NOTE),
        (NOTE_GS5, DOTTED_EIGHTEENTH_NOTE),
        (NOTE_E6, QUARTER_NOTE)
    ]
);