//! The microphone where there is no ALSA: the default input device through cpal (CoreAudio on
//! macOS, WASAPI on Windows), its channels mixed to mono and resampled to the rate asked for, read
//! the way `alsa::Mic` is read: blocking for the next stretch of 16-bit samples.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat};
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

/// Samples waiting to be read, at the asked rate, and whether the device has failed.
#[derive(Default)]
struct Queue {
    samples: VecDeque<i16>,
    failed: Option<String>,
}

/// Mono samples at the device's rate turned into samples at `want`, by linear interpolation,
/// carrying the fractional position and the last sample between calls.
struct Resampler {
    step: f64,
    at: f64,
    last: f32,
}

impl Resampler {
    fn run(&mut self, input: &[f32], out: &mut Vec<i16>) {
        for &x in input {
            // each output instant between the last input sample and this one
            while self.at <= 1.0 {
                let v = self.last + (x - self.last) * self.at as f32;
                out.push((v.clamp(-1.0, 1.0) * 32767.0) as i16);
                self.at += self.step;
            }
            self.at -= 1.0;
            self.last = x;
        }
    }
}

pub struct Mic {
    queue: Arc<(Mutex<Queue>, Condvar)>,
    _stream: cpal::Stream,
}

impl Mic {
    /// The default input device, recording at `rate` (mono, 16-bit) from now on.
    pub fn open(rate: u32) -> Result<Mic, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("no microphone: the system has no default input device")?;
        let config = device
            .default_input_config()
            .map_err(|e| format!("the microphone's settings: {e}"))?;
        let channels = usize::from(config.channels()).max(1);
        let from = config.sample_rate() as f64;
        let queue = Arc::new((Mutex::new(Queue::default()), Condvar::new()));
        let resampler = Arc::new(Mutex::new(Resampler {
            step: from / f64::from(rate),
            at: 0.0,
            last: 0.0,
        }));
        let failed = queue.clone();
        let on_error = move |e: cpal::Error| {
            let (q, ready) = &*failed;
            if let Ok(mut q) = q.lock() {
                q.failed = Some(format!("the microphone stopped: {e}"));
            }
            ready.notify_all();
        };
        let stream = match config.sample_format() {
            SampleFormat::F32 => build::<f32>(&device, config.into(), channels, &queue, &resampler, on_error),
            SampleFormat::I16 => build::<i16>(&device, config.into(), channels, &queue, &resampler, on_error),
            SampleFormat::I32 => build::<i32>(&device, config.into(), channels, &queue, &resampler, on_error),
            SampleFormat::U16 => build::<u16>(&device, config.into(), channels, &queue, &resampler, on_error),
            SampleFormat::I8 => build::<i8>(&device, config.into(), channels, &queue, &resampler, on_error),
            f => {
                return Err(format!(
                    "the microphone's sample format {f} is not one this reads"
                ));
            }
        }
        .map_err(|e| format!("cannot open the microphone: {e}"))?;
        stream
            .play()
            .map_err(|e| format!("cannot start the microphone: {e}"))?;
        Ok(Mic {
            queue,
            _stream: stream,
        })
    }

    /// The next samples, as many as are ready up to `buf`'s length: waits for at least one.
    pub fn read(&mut self, buf: &mut [i16]) -> Result<usize, String> {
        let (q, ready) = &*self.queue;
        let mut q = q
            .lock()
            .map_err(|_| "the microphone's queue failed".to_string())?;
        loop {
            if let Some(e) = &q.failed {
                return Err(e.clone());
            }
            if !q.samples.is_empty() {
                let n = buf.len().min(q.samples.len());
                for (b, s) in buf.iter_mut().zip(q.samples.drain(..n)) {
                    *b = s;
                }
                return Ok(n);
            }
            q = ready
                .wait_timeout(q, Duration::from_secs(2))
                .map_err(|_| "the microphone's queue failed".to_string())?
                .0;
        }
    }
}

fn build<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    queue: &Arc<(Mutex<Queue>, Condvar)>,
    resampler: &Arc<Mutex<Resampler>>,
    on_error: impl FnMut(cpal::Error) + Send + 'static,
) -> Result<cpal::Stream, cpal::Error>
where
    T: Sample + cpal::SizedSample + Send + 'static,
    f32: FromSample<T>,
{
    let (queue, resampler) = (queue.clone(), resampler.clone());
    let mut mono = Vec::new();
    let mut out = Vec::new();
    device.build_input_stream(
        config,
        move |data: &[T], _: &_| {
            mono.clear();
            mono.extend(
                data.chunks(channels).map(|frame| {
                    frame.iter().map(|&s| f32::from_sample(s)).sum::<f32>() / frame.len() as f32
                }),
            );
            out.clear();
            if let Ok(mut r) = resampler.lock() {
                r.run(&mono, &mut out);
            }
            let (q, ready) = &*queue;
            if let Ok(mut q) = q.lock() {
                q.samples.extend(&out);
                // never more than ten seconds behind: a reader that stopped loses the oldest
                let cap = 160_000;
                if q.samples.len() > cap {
                    let extra = q.samples.len() - cap;
                    q.samples.drain(..extra);
                }
            }
            ready.notify_all();
        },
        on_error,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::Resampler;

    #[test]
    fn three_to_one_keeps_a_third_of_the_samples_and_the_level() {
        let mut r = Resampler {
            step: 3.0,
            at: 0.0,
            last: 0.0,
        };
        let mut out = vec![];
        r.run(&vec![0.5f32; 48_000], &mut out);
        assert!((15_999..=16_001).contains(&out.len()), "{}", out.len());
        assert!(out[10..].iter().all(|&s| s == (0.5f32 * 32767.0) as i16));
    }
}
