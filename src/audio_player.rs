use crate::trim_silence::TrimSilenceSource;
use crate::types::TrimSilenceMode;
use bytes::Bytes;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use std::fs::File;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Clone)]
pub struct AudioPlayer {
    player: Arc<Mutex<Option<(Player, MixerDeviceSink)>>>,
    current_episode_id: Arc<Mutex<Option<i32>>>,
    playback_speed: Arc<Mutex<f32>>,
    state: Arc<Mutex<PlaybackState>>,
    duration: Arc<Mutex<Duration>>,
    trim_silence_mode: Arc<Mutex<TrimSilenceMode>>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        Self {
            player: Arc::new(Mutex::new(None)),
            current_episode_id: Arc::new(Mutex::new(None)),
            playback_speed: Arc::new(Mutex::new(1.0)),
            state: Arc::new(Mutex::new(PlaybackState::Stopped)),
            duration: Arc::new(Mutex::new(Duration::from_secs(0))),
            trim_silence_mode: Arc::new(Mutex::new(TrimSilenceMode::Off)),
        }
    }

    pub fn set_trim_silence_mode(&self, mode: TrimSilenceMode) {
        *self.trim_silence_mode.lock().unwrap() = mode;
    }

    pub fn get_trim_silence_mode(&self) -> TrimSilenceMode {
        *self.trim_silence_mode.lock().unwrap()
    }

    pub fn play_from_file(&self, path: &str, episode_id: i32) -> Result<(), String> {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let source = Decoder::new(file).map_err(|e| e.to_string())?;

        let duration = source.total_duration().unwrap_or_else(|| {
            std::fs::read(path)
                .ok()
                .and_then(|b| estimate_duration(&b))
                .unwrap_or(Duration::ZERO)
        });

        let boxed: Box<dyn Source<Item = f32> + Send + 'static> = Box::new(source);
        self.start_source(boxed, episode_id, duration)
    }

    pub fn play_from_memory(&self, bytes: Bytes, episode_id: i32) -> Result<(), String> {
        let cursor = Cursor::new(bytes.clone());
        let source = Decoder::new(cursor).map_err(|e| e.to_string())?;

        let duration = source
            .total_duration()
            .unwrap_or_else(|| estimate_duration(&bytes).unwrap_or(Duration::ZERO));

        let boxed: Box<dyn Source<Item = f32> + Send + 'static> = Box::new(source);
        self.start_source(boxed, episode_id, duration)
    }

    fn start_source(
        &self,
        source: Box<dyn Source<Item = f32> + Send + 'static>,
        episode_id: i32,
        duration: Duration,
    ) -> Result<(), String> {
        let mut player_guard = self.player.lock().unwrap();

        if let Some((old_player, _)) = player_guard.take() {
            old_player.stop();
        }

        let mut stream = DeviceSinkBuilder::open_default_sink().map_err(|e| e.to_string())?;
        stream.log_on_drop(false);
        let new_player = Player::connect_new(&stream.mixer());
        let speed = *self.playback_speed.lock().unwrap();
        let trim_mode = *self.trim_silence_mode.lock().unwrap();

        match trim_mode {
            TrimSilenceMode::Off => {
                new_player.append(source.speed(speed));
            }
            mode => {
                new_player.append(TrimSilenceSource::new(source, mode).speed(speed));
            }
        }
        new_player.play();

        *player_guard = Some((new_player, stream));
        *self.current_episode_id.lock().unwrap() = Some(episode_id);
        *self.state.lock().unwrap() = PlaybackState::Playing;
        *self.duration.lock().unwrap() = duration;

        Ok(())
    }

    pub fn pause(&self) {
        if let Some((player, _)) = self.player.lock().unwrap().as_ref() {
            player.pause();
            *self.state.lock().unwrap() = PlaybackState::Paused;
        }
    }

    pub fn resume(&self) {
        if let Some((player, _)) = self.player.lock().unwrap().as_ref() {
            player.play();
            *self.state.lock().unwrap() = PlaybackState::Playing;
        }
    }

    pub fn stop(&self) {
        if let Some((player, _)) = self.player.lock().unwrap().take() {
            player.stop();
        }
        *self.current_episode_id.lock().unwrap() = None;
        *self.state.lock().unwrap() = PlaybackState::Stopped;
    }

    pub fn set_volume(&self, volume: f32) {
        if let Some((player, _)) = self.player.lock().unwrap().as_ref() {
            player.set_volume(volume / 100.0);
        }
    }

    pub fn set_speed(&self, speed: f32) {
        *self.playback_speed.lock().unwrap() = speed;
        if let Some((player, _)) = self.player.lock().unwrap().as_ref() {
            player.set_speed(speed);
        }
    }

    pub fn seek(&self, position: Duration) {
        if let Some((player, _)) = self.player.lock().unwrap().as_ref() {
            player.try_seek(position).ok();
        }
    }

    pub fn skip_forward(&self, seconds: i32) {
        if let Some((player, _)) = self.player.lock().unwrap().as_ref() {
            let current = player.get_pos();
            let new_pos = current + Duration::from_secs(seconds as u64);
            player.try_seek(new_pos).ok();
        }
    }

    pub fn skip_backward(&self, seconds: i32) {
        if let Some((player, _)) = self.player.lock().unwrap().as_ref() {
            let current = player.get_pos();
            let new_pos = current.saturating_sub(Duration::from_secs(seconds as u64));
            player.try_seek(new_pos).ok();
        }
    }

    pub fn get_position(&self) -> Duration {
        if let Some((player, _)) = self.player.lock().unwrap().as_ref() {
            player.get_pos()
        } else {
            Duration::from_secs(0)
        }
    }

    pub fn get_state(&self) -> PlaybackState {
        *self.state.lock().unwrap()
    }

    pub fn get_current_episode_id(&self) -> Option<i32> {
        *self.current_episode_id.lock().unwrap()
    }

    pub fn get_speed(&self) -> f32 {
        *self.playback_speed.lock().unwrap()
    }

    pub fn get_duration(&self) -> Duration {
        *self.duration.lock().unwrap()
    }

    pub fn is_finished(&self) -> bool {
        if let Some((player, _)) = self.player.lock().unwrap().as_ref() {
            player.empty() && *self.state.lock().unwrap() == PlaybackState::Playing
        } else {
            false
        }
    }
}

fn estimate_duration(bytes: &[u8]) -> Option<Duration> {
    if bytes.len() < 4 {
        return None;
    }

    let start = if bytes.starts_with(b"ID3") {
        if bytes.len() < 10 {
            return None;
        }
        let sz = ((bytes[6] as usize) << 21)
            | ((bytes[7] as usize) << 14)
            | ((bytes[8] as usize) << 7)
            | (bytes[9] as usize);
        10 + sz
    } else {
        0
    };

    let search = &bytes[start.min(bytes.len())..];
    let frame_start = search
        .windows(2)
        .position(|w| w[0] == 0xFF && (w[1] & 0xE0) == 0xE0)?;

    if frame_start + 3 >= search.len() {
        return None;
    }

    let header = &search[frame_start..frame_start + 4];
    let mpeg_version = (header[1] >> 3) & 0x03;
    let bitrate_index = (header[2] >> 4) as usize;

    let bitrate_table: [[u32; 16]; 2] = [
        [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0],
        [0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0],
    ];

    let version_idx = if mpeg_version == 3 { 1 } else { 0 };
    let bitrate_kbps = bitrate_table[version_idx][bitrate_index];

    if bitrate_kbps == 0 {
        return None;
    }

    let bytes_per_sec = (bitrate_kbps * 1000 / 8) as usize;
    let secs = bytes.len() / bytes_per_sec;

    Some(Duration::from_secs(secs as u64))
}
