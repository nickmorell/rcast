use rodio::{Decoder, OutputStream, Sink, Source};
use std::fs::File;
use std::io::BufReader;
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
    sink: Arc<Mutex<Option<(Sink, OutputStream)>>>,
    current_episode_id: Arc<Mutex<Option<i32>>>,
    playback_speed: Arc<Mutex<f32>>,
    state: Arc<Mutex<PlaybackState>>,
    duration: Arc<Mutex<Duration>>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        Self {
            sink: Arc::new(Mutex::new(None)),
            current_episode_id: Arc::new(Mutex::new(None)),
            playback_speed: Arc::new(Mutex::new(1.0)),
            state: Arc::new(Mutex::new(PlaybackState::Stopped)),
            duration: Arc::new(Mutex::new(Duration::from_secs(0))),
        }
    }

    pub fn play(
        &self,
        audio_path: &str,
        episode_id: i32,
        duration_secs: i64,
    ) -> Result<(), String> {
        let file = File::open(audio_path).map_err(|e| e.to_string())?;
        let source = Decoder::new(BufReader::new(file)).map_err(|e| e.to_string())?;

        let mut sink_guard = self.sink.lock().unwrap();

        if let Some((old_sink, _)) = sink_guard.take() {
            old_sink.stop();
        }

        let (_stream, stream_handle) = OutputStream::try_default().map_err(|e| e.to_string())?;
        let new_sink = Sink::try_new(&stream_handle).map_err(|e| e.to_string())?;
        let speed = *self.playback_speed.lock().unwrap();
        new_sink.append(source.speed(speed));
        new_sink.play();

        *sink_guard = Some((new_sink, _stream));
        *self.current_episode_id.lock().unwrap() = Some(episode_id);
        *self.state.lock().unwrap() = PlaybackState::Playing;
        *self.duration.lock().unwrap() = Duration::from_secs(duration_secs as u64);

        Ok(())
    }

    pub fn pause(&self) {
        if let Some((sink, _)) = self.sink.lock().unwrap().as_ref() {
            sink.pause();
            *self.state.lock().unwrap() = PlaybackState::Paused;
        }
    }

    pub fn resume(&self) {
        if let Some((sink, _)) = self.sink.lock().unwrap().as_ref() {
            sink.play();
            *self.state.lock().unwrap() = PlaybackState::Playing;
        }
    }

    pub fn stop(&self) {
        if let Some((sink, _)) = self.sink.lock().unwrap().take() {
            sink.stop();
        }
        *self.current_episode_id.lock().unwrap() = None;
        *self.state.lock().unwrap() = PlaybackState::Stopped;
    }

    pub fn set_volume(&self, volume: f32) {
        if let Some((sink, _)) = self.sink.lock().unwrap().as_ref() {
            sink.set_volume(volume / 100.0);
        }
    }

    pub fn set_speed(&self, speed: f32) {
        *self.playback_speed.lock().unwrap() = speed;
        if let Some((sink, _)) = self.sink.lock().unwrap().as_ref() {
            sink.set_speed(speed);
        }
    }

    pub fn seek(&self, position: Duration) {
        if let Some((sink, _)) = self.sink.lock().unwrap().as_ref() {
            sink.try_seek(position).ok();
        }
    }

    pub fn skip_forward(&self, seconds: i32) {
        if let Some((sink, _)) = self.sink.lock().unwrap().as_ref() {
            let current = sink.get_pos();
            let new_pos = current + Duration::from_secs(seconds as u64);
            sink.try_seek(new_pos).ok();
        }
    }

    pub fn skip_backward(&self, seconds: i32) {
        if let Some((sink, _)) = self.sink.lock().unwrap().as_ref() {
            let current = sink.get_pos();
            let new_pos = current.saturating_sub(Duration::from_secs(seconds as u64));
            sink.try_seek(new_pos).ok();
        }
    }

    pub fn get_position(&self) -> Duration {
        if let Some((sink, _)) = self.sink.lock().unwrap().as_ref() {
            sink.get_pos()
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
        if let Some((sink, _)) = self.sink.lock().unwrap().as_ref() {
            sink.empty() && *self.state.lock().unwrap() == PlaybackState::Playing
        } else {
            false
        }
    }
}
