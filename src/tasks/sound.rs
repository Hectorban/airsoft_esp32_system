use embassy_sync::channel::{Channel, Receiver};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::{Duration, Timer};
use esp_hal_buzzer::{Buzzer, ToneValue};

#[derive(Clone)]
pub enum SoundCommand {
    PlayTone { frequency: u32, duration_ms: u32 },
    PlaySong { song: &'static [ToneValue] },
    Beep,
    DoubleBeep,
    ErrorBeep,
    SuccessBeep,
    Stop,
}

pub const SOUND_QUEUE_SIZE: usize = 16;
pub type SoundChannel = Channel<NoopRawMutex, SoundCommand, { SOUND_QUEUE_SIZE }>;

#[embassy_executor::task]
pub async fn sound_task(
    mut buzzer: esp_hal_buzzer::Buzzer<'static>,
    receiver: Receiver<'static, NoopRawMutex, SoundCommand, SOUND_QUEUE_SIZE>,
) {
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