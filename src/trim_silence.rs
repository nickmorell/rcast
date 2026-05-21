use crate::types::TrimSilenceMode;
use rodio::Source;
use std::num::NonZero;
use std::time::Duration;

const SILENCE_THRESHOLD: f32 = 0.01;
const MIN_SILENCE_SAMPLES: u64 = 13230; // ~300ms at 44100 Hz mono

/// Wraps any `rodio::Source` to remove or speed through silent sections.
///
/// - `SmartSpeed`: during silence, outputs every other sample (effective 2× speed).
/// - `SkipSilence`: during silence, drops all silent samples entirely.
///
/// Silence is detected via an exponential moving average of absolute amplitude.
pub struct TrimSilenceSource {
    inner: Box<dyn Source<Item = f32> + Send + 'static>,
    mode: TrimSilenceMode,
    ema: f32,
    silent_samples: u64,
    smart_skip: bool,
    channels: NonZero<u16>,
    sample_rate: NonZero<u32>,
}

impl TrimSilenceSource {
    pub fn new(
        inner: Box<dyn Source<Item = f32> + Send + 'static>,
        mode: TrimSilenceMode,
    ) -> Self {
        let channels = inner.channels();
        let sample_rate = inner.sample_rate();
        Self {
            inner,
            mode,
            ema: 0.0,
            silent_samples: 0,
            smart_skip: false,
            channels,
            sample_rate,
        }
    }
}

impl Iterator for TrimSilenceSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        loop {
            let sample = self.inner.next()?;

            // Exponential moving average of absolute amplitude (fast attack).
            self.ema = self.ema * 0.995 + sample.abs() * 0.005;

            let is_silent = self.ema < SILENCE_THRESHOLD;

            if is_silent {
                self.silent_samples += 1;
                // Pass through a short lead-in before applying the effect.
                if self.silent_samples < MIN_SILENCE_SAMPLES {
                    return Some(sample);
                }
                match self.mode {
                    TrimSilenceMode::Off => return Some(sample),
                    TrimSilenceMode::SkipSilence => {
                        // Drop this sample; pull the next one.
                        continue;
                    }
                    TrimSilenceMode::SmartSpeed => {
                        // Output every other sample → 2× speed during silence.
                        self.smart_skip = !self.smart_skip;
                        if self.smart_skip {
                            continue;
                        }
                        return Some(sample);
                    }
                }
            } else {
                self.silent_samples = 0;
                return Some(sample);
            }
        }
    }
}

impl Source for TrimSilenceSource {
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }

    fn channels(&self) -> NonZero<u16> {
        self.channels
    }

    fn sample_rate(&self) -> NonZero<u32> {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        // Duration is unknown after trimming.
        None
    }
}
