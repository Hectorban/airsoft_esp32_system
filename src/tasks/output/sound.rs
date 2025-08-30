use embassy_sync::channel::{Channel, Receiver};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::Duration;
use esp_hal_buzzer::{notes::*, song, ToneValue};

#[derive(Clone)]
pub enum SoundCommand {
    PlayTone { frequency: u32, duration_ms: u32 },
    PlaySong { song: &'static [ToneValue] },
    Beep,
    DoubleBeep,
    ErrorBeep,
    SuccessBeep,
    VictorySound,
    DefeatSound,
    Stop,
}

pub const SOUND_QUEUE_SIZE: usize = 16;
pub type SoundChannel = Channel<NoopRawMutex, SoundCommand, { SOUND_QUEUE_SIZE }>;

pub const VICTORY_SONG: [ToneValue; 23] = song!(
    200,
    [
        (NOTE_G4, SIXTEENTH_NOTE),
        (NOTE_C5, SIXTEENTH_NOTE),
        (NOTE_E5, SIXTEENTH_NOTE),
        (NOTE_G5, SIXTEENTH_NOTE),
        (NOTE_C6, SIXTEENTH_NOTE),
        (NOTE_E6, SIXTEENTH_NOTE),
        (NOTE_G6, HALF_NOTE),
        (NOTE_E6, HALF_NOTE),
        (NOTE_GS4, SIXTEENTH_NOTE),
        (NOTE_C5, SIXTEENTH_NOTE),
        (NOTE_DS5, SIXTEENTH_NOTE),
        (NOTE_GS5, SIXTEENTH_NOTE),
        (NOTE_C6, SIXTEENTH_NOTE),
        (NOTE_DS6, SIXTEENTH_NOTE),
        (NOTE_GS6, HALF_NOTE),
        (NOTE_DS6, HALF_NOTE),
        (NOTE_AS4, SIXTEENTH_NOTE),
        (NOTE_D5, SIXTEENTH_NOTE),
        (NOTE_F5, SIXTEENTH_NOTE),
        (NOTE_AS5, SIXTEENTH_NOTE),
        (NOTE_D6, SIXTEENTH_NOTE),
        (NOTE_F6, SIXTEENTH_NOTE),
        (NOTE_AS6, HALF_NOTE)
    ]
);

pub const DEFEAT_SONG: [ToneValue; 7] = song!(
    120,
    [
        (NOTE_C4, EIGHTEENTH_NOTE),
        (NOTE_G3, EIGHTEENTH_NOTE),
        (NOTE_E3, EIGHTEENTH_NOTE),
        (NOTE_A3, QUARTER_NOTE),
        (NOTE_B3, QUARTER_NOTE),
        (NOTE_A3, QUARTER_NOTE),
        (NOTE_GS3, HALF_NOTE)
    ]
);


#[embassy_executor::task]
pub async fn sound_task(
    mut buzzer: esp_hal_buzzer::Buzzer<'static>,
    receiver: Receiver<'static, NoopRawMutex, SoundCommand, SOUND_QUEUE_SIZE>,
) {
    // Todo Background BGM?
    loop {
        let command = receiver.receive().await;
        match command {
            SoundCommand::PlayTone { frequency, duration_ms } => {
                let _ = buzzer.play_tones([frequency], [duration_ms]);
            },
            SoundCommand::PlaySong { song } => {
                // Convert song to individual tone calls since play_song expects owned array
                for tone in song.iter() {
                    let _ = buzzer.play_tones([tone.frequency], [tone.duration]);
                    embassy_time::Timer::after(Duration::from_millis(10)).await; // Small gap between tones
                }
            },
            SoundCommand::VictorySound => {
                let _ = buzzer.play_song(VICTORY_SONG);
            }
            SoundCommand::DefeatSound => {
                let _ = buzzer.play_song(DEFEAT_SONG);
            }
            SoundCommand::Beep => {
                let _ = buzzer.play_tones([1000], [100]);
            },
            SoundCommand::DoubleBeep => {
                let _ = buzzer.play_tones([1000], [100]);
                embassy_time::Timer::after(Duration::from_millis(50)).await;
                let _ = buzzer.play_tones([1000], [100]);
            },
            SoundCommand::ErrorBeep => {
                let _ = buzzer.play_tones([400], [500]);
            },
            SoundCommand::SuccessBeep => {
                let _ = buzzer.play_tones([800], [100]);
                embassy_time::Timer::after(Duration::from_millis(50)).await;
                let _ = buzzer.play_tones([1000], [100]);
                embassy_time::Timer::after(Duration::from_millis(50)).await;
                let _ = buzzer.play_tones([1200], [200]);
            },
            SoundCommand::Stop => {
                // Stop current sound by playing silence - use a very short silent tone
                let _ = buzzer.play_tones([0], [1]);
            },
        }
    }
}