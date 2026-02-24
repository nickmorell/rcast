use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use std::fs::File;
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
}

impl AudioPlayer {
    pub fn new() -> Self {
        Self {
            player: Arc::new(Mutex::new(None)),
            current_episode_id: Arc::new(Mutex::new(None)),
            playback_speed: Arc::new(Mutex::new(1.0)),
            state: Arc::new(Mutex::new(PlaybackState::Stopped)),
            duration: Arc::new(Mutex::new(Duration::from_secs(0))),
        }
    }

    pub fn play(&self, audio_path: &str, episode_id: i32) -> Result<(), String> {
        let file = File::open(audio_path).map_err(|e| e.to_string())?;
        let source = Decoder::try_from(file).map_err(|e| e.to_string())?;
        let secs = source.total_duration().unwrap();
        let mut player_guard = self.player.lock().unwrap();

        if let Some((old_player, _)) = player_guard.take() {
            old_player.stop();
        }

        let stream = DeviceSinkBuilder::open_default_sink().map_err(|e| e.to_string())?;
        let new_player = Player::connect_new(&stream.mixer());
        let speed = *self.playback_speed.lock().unwrap();
        new_player.append(source.speed(speed));
        new_player.play();

        *player_guard = Some((new_player, stream));
        *self.current_episode_id.lock().unwrap() = Some(episode_id);
        *self.state.lock().unwrap() = PlaybackState::Playing;
        *self.duration.lock().unwrap() = Duration::from_secs(secs.as_secs());

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
