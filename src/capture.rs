use crate::error::CaptureHelperError;
use crate::frame::MicFrame;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Instant;

/// A live handle on a microphone input stream.
///
/// `cpal` delivers audio through a realtime callback, so `MicCapture` bridges
/// that callback to an ordinary `std::sync::mpsc` channel: construction spins
/// up the device stream and returns immediately, and frames arrive as they
/// are captured. Consume them by iterating over `MicCapture` directly
/// (blocking, one [`MicFrame`] per iteration) or via [`MicCapture::try_next_frame`]
/// for a non-blocking poll.
///
/// The stream keeps running for as long as this value is alive; dropping it
/// stops capture.
pub struct MicCapture {
    _stream: cpal::Stream,
    rx: Receiver<MicFrame>,
}

impl MicCapture {
    /// Start capturing from the host's default input device.
    pub fn from_default_device() -> Result<Self, CaptureHelperError> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or(CaptureHelperError::NoInputDevice)?;
        Self::from_device(&device)
    }

    /// Start capturing from the input device whose `cpal` name matches `name`
    /// exactly. Use [`crate::list_input_devices`] to discover available names.
    pub fn from_named_device(name: &str) -> Result<Self, CaptureHelperError> {
        let host = cpal::default_host();
        let mut devices = host
            .input_devices()
            .map_err(|e| CaptureHelperError::DeviceEnumeration(e.to_string()))?;
        let device = devices
            .find(|d| d.to_string() == name)
            .ok_or_else(|| CaptureHelperError::DeviceNotFound(name.to_string()))?;
        Self::from_device(&device)
    }

    fn from_device(device: &cpal::Device) -> Result<Self, CaptureHelperError> {
        let supported_config = device
            .default_input_config()
            .map_err(|e| CaptureHelperError::StreamConfig(e.to_string()))?;
        let sample_format = supported_config.sample_format();
        let channels = supported_config.channels();
        let sample_rate = supported_config.sample_rate();
        let config: StreamConfig = supported_config.into();

        let (tx, rx) = mpsc::channel::<MicFrame>();

        let stream = match sample_format {
            SampleFormat::F32 => {
                build_stream::<f32>(device, config, tx, channels, sample_rate, |s| s)?
            }
            // Divide by 32768.0 (i16::MIN's magnitude), not i16::MAX (32767):
            // dividing by MAX would send i16::MIN to -1.0000305, breaking the
            // documented [-1.0, 1.0] guarantee. Same convention as the U16
            // branch below.
            SampleFormat::I16 => {
                build_stream::<i16>(device, config, tx, channels, sample_rate, |s| {
                    s as f32 / 32768.0
                })?
            }
            SampleFormat::U16 => {
                build_stream::<u16>(device, config, tx, channels, sample_rate, |s| {
                    (s as f32 - 32768.0) / 32768.0
                })?
            }
            other => {
                return Err(CaptureHelperError::UnsupportedSampleFormat(format!(
                    "{other:?}"
                )));
            }
        };

        stream
            .play()
            .map_err(|e| CaptureHelperError::StreamPlay(e.to_string()))?;

        Ok(Self {
            _stream: stream,
            rx,
        })
    }

    /// Block until the next [`MicFrame`] is available, or the stream has
    /// stopped producing (device disconnected, `cpal` callback thread gone).
    pub fn next_frame(&self) -> Option<MicFrame> {
        self.rx.recv().ok()
    }

    /// Non-blocking poll: returns `None` immediately if no frame is queued
    /// yet rather than waiting for one.
    pub fn try_next_frame(&self) -> Option<MicFrame> {
        self.rx.try_recv().ok()
    }
}

/// Iterating over a `MicCapture` blocks for each [`MicFrame`] in turn — the
/// idiomatic way to drain a live microphone stream in a `for` loop.
impl Iterator for MicCapture {
    type Item = MicFrame;

    fn next(&mut self) -> Option<MicFrame> {
        self.rx.recv().ok()
    }
}

/// Builds and wires up the `cpal` input stream for a concrete sample type
/// `T`, converting each sample to `f32` with `to_f32` before it crosses the
/// channel as a [`MicFrame`].
fn build_stream<T>(
    device: &cpal::Device,
    config: StreamConfig,
    tx: Sender<MicFrame>,
    channels: u16,
    sample_rate: u32,
    to_f32: fn(T) -> f32,
) -> Result<cpal::Stream, CaptureHelperError>
where
    T: cpal::SizedSample + Send + 'static,
{
    device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                let samples: Vec<f32> = data.iter().copied().map(to_f32).collect();
                let frame = MicFrame {
                    samples,
                    sample_rate,
                    channels,
                    timestamp: Instant::now(),
                };
                // The receiver may already be gone (MicCapture dropped mid-callback);
                // that is a normal shutdown race, not a bug, so the send error is
                // silently ignored rather than panicking on the audio thread.
                let _ = tx.send(frame);
            },
            |err| {
                // cpal's error callback has no channel back to the caller that
                // constructed the stream; eprintln-and-drop is the standard pattern
                // here (mirrors scribe-reunion's sr-io mic adapter).
                eprintln!("capture-helper-rs: input stream error: {err}");
            },
            None,
        )
        .map_err(|e| CaptureHelperError::StreamBuild(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CaptureHelperError;

    /// The i16 -> f32 conversion used in `from_device` must stay within the
    /// documented [-1.0, 1.0] bound at both extremes, including i16::MIN
    /// (which a naive `/ i16::MAX` conversion overshoots to -1.0000305).
    #[test]
    fn i16_to_f32_conversion_stays_within_bounds() {
        let to_f32 = |s: i16| s as f32 / 32768.0;
        let min = to_f32(i16::MIN);
        let max = to_f32(i16::MAX);
        assert!(
            (-1.0..=1.0).contains(&min),
            "i16::MIN mapped to {min}, outside [-1.0, 1.0]"
        );
        assert!(
            (-1.0..=1.0).contains(&max),
            "i16::MAX mapped to {max}, outside [-1.0, 1.0]"
        );
        assert_eq!(min, -1.0);
    }

    /// Real streaming (`from_default_device`, or `from_named_device` against
    /// a name that actually exists) needs live audio hardware and cannot be
    /// exercised in this headless CI/sandbox environment — no microphone is
    /// attached here, and no test in this crate claims otherwise. See the
    /// README's "Limites" section.
    ///
    /// What *is* verifiable without hardware: requesting a device name that
    /// provably does not exist must fail with a distinct, correct error
    /// rather than panicking — whether that's `DeviceNotFound` (the host
    /// enumerated devices and none matched) or `DeviceEnumeration` (the host
    /// has no audio subsystem at all, possible on some CI images).
    #[test]
    fn from_named_device_fails_cleanly_for_a_bogus_name() {
        let bogus = "definitely-not-a-real-input-device-name-capture-helper-rs-test";
        match MicCapture::from_named_device(bogus) {
            Err(CaptureHelperError::DeviceNotFound(name)) => assert_eq!(name, bogus),
            Err(CaptureHelperError::DeviceEnumeration(_)) => {
                // No audio subsystem available at all on this host; still a
                // clean, typed error rather than a panic.
            }
            Err(other) => panic!("expected DeviceNotFound or DeviceEnumeration, got {other:?}"),
            Ok(_) => panic!("expected an error for a bogus device name, but capture started"),
        }
    }
}
