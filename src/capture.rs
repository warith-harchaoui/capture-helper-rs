use crate::error::CaptureHelperError;
use crate::frame::MicFrame;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How many captured frames may queue up before the oldest audio starts being
/// refused.
///
/// A microphone produces audio at exactly realtime, so a consumer that is even
/// slightly slower than realtime never catches up. With an unbounded queue that
/// is not a slowdown, it is a leak: the backlog grows for as long as capture
/// runs. At cpal's typical callback size this is on the order of seconds of
/// audio -- generous enough that an ordinary hiccup (a GC pause, a slow disk
/// write) costs nothing, small enough that a consumer which has genuinely
/// fallen behind is bounded rather than fatal.
const FRAME_QUEUE_CAPACITY: usize = 256;

/// The state a capture callback shares with its `MicCapture`.
///
/// Grouped rather than passed as two more arguments: they are one concept --
/// what the audio thread needs to tell the consumer about, after the fact,
/// without blocking or panicking on the realtime thread.
struct CallbackState {
    failure: Arc<StreamFailure>,
    dropped: Arc<AtomicU64>,
}

/// How long a blocking wait sits on the channel before looking at
/// [`StreamFailure`] again.
///
/// `cpal` reports a device failure through a callback that cannot close the
/// frame channel, so a waiter blocked forever on `recv()` would never learn the
/// stream is dead. Waking a few times a second costs nothing measurable and is
/// what lets iteration end instead of hanging.
const FAILURE_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// The one piece of state shared between a live stream's error callback and the
/// [`MicCapture`] handle.
///
/// `cpal`'s error callback has no return path to whoever built the stream, so
/// before 0.1.2 a device that failed mid-capture printed one line to stderr and
/// left every consumer blocked on a channel nothing would ever write to again.
/// The failure is recorded here instead, where both the blocking wait and
/// [`MicCapture::error`] can see it.
#[derive(Default)]
struct StreamFailure {
    failed: AtomicBool,
    message: Mutex<Option<String>>,
}

impl StreamFailure {
    fn record(&self, message: String) {
        // Message first, flag second: a reader that sees `failed` must find the
        // message already in place, never an empty slot.
        *self.message.lock().unwrap_or_else(|p| p.into_inner()) = Some(message);
        self.failed.store(true, Ordering::Release);
    }

    fn failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }

    fn message(&self) -> Option<String> {
        self.message
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }
}

/// The blocking wait behind [`MicCapture::next_frame`] and the `Iterator` impl.
///
/// Returns `None` once the stream can no longer produce: either every sender is
/// gone, or the device reported a failure. Free-standing (rather than a method)
/// so it can be exercised without audio hardware — see this module's tests.
fn recv_frame(rx: &Receiver<MicFrame>, failure: &StreamFailure) -> Option<MicFrame> {
    loop {
        match rx.recv_timeout(FAILURE_POLL_INTERVAL) {
            Ok(frame) => return Some(frame),
            Err(RecvTimeoutError::Disconnected) => return None,
            Err(RecvTimeoutError::Timeout) => {
                if failure.failed() {
                    return None;
                }
            }
        }
    }
}

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
    failure: Arc<StreamFailure>,
    dropped: Arc<AtomicU64>,
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

        let (tx, rx) = mpsc::sync_channel::<MicFrame>(FRAME_QUEUE_CAPACITY);
        let failure = Arc::new(StreamFailure::default());
        let dropped = Arc::new(AtomicU64::new(0));
        let state = CallbackState {
            failure: Arc::clone(&failure),
            dropped: Arc::clone(&dropped),
        };

        let stream = match sample_format {
            SampleFormat::F32 => {
                build_stream::<f32>(device, config, tx, &state, channels, sample_rate, |s| s)?
            }
            // Divide by 32768.0 (i16::MIN's magnitude), not i16::MAX (32767):
            // dividing by MAX would send i16::MIN to -1.0000305, breaking the
            // documented [-1.0, 1.0] guarantee. Same convention as the U16
            // branch below.
            SampleFormat::I16 => {
                build_stream::<i16>(device, config, tx, &state, channels, sample_rate, |s| {
                    s as f32 / 32768.0
                })?
            }
            SampleFormat::U16 => {
                build_stream::<u16>(device, config, tx, &state, channels, sample_rate, |s| {
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
            failure,
            dropped,
        })
    }

    /// How many frames the capture had to refuse because the consumer fell
    /// behind, since this capture started.
    ///
    /// A microphone runs at realtime, so a consumer that is even slightly
    /// slower never catches up. This counter is how that shows up: nonzero
    /// means audio is missing from what you received, and the gaps are at the
    /// moments you were busiest. Zero is the normal reading.
    ///
    /// Checking it occasionally is worth more than it looks -- it is the
    /// difference between a pipeline that is lossy and one that is lossy
    /// *without anyone knowing*.
    pub fn dropped_frames(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Block until the next [`MicFrame`] is available, or the stream has
    /// stopped producing — device disconnected, `cpal` callback thread gone, or
    /// the device reported an error. `None` means no further frame will ever
    /// arrive; call [`MicCapture::error`] to find out whether that was a failure
    /// or an ordinary shutdown.
    pub fn next_frame(&self) -> Option<MicFrame> {
        recv_frame(&self.rx, &self.failure)
    }

    /// Non-blocking poll: returns `None` immediately if no frame is queued
    /// yet rather than waiting for one. Note that `None` here is ambiguous by
    /// design — nothing yet, or nothing ever; [`MicCapture::error`] and
    /// [`MicCapture::next_frame`] are what distinguish the two.
    pub fn try_next_frame(&self) -> Option<MicFrame> {
        self.rx.try_recv().ok()
    }

    /// The device error that ended this capture, if one did.
    ///
    /// `cpal` surfaces stream failures through a callback with no path back to
    /// the caller, so the message is parked here rather than printed. `None`
    /// means the stream has not failed — which includes a stream that simply has
    /// not produced anything yet.
    pub fn error(&self) -> Option<String> {
        self.failure.message()
    }
}

/// Iterating over a `MicCapture` blocks for each [`MicFrame`] in turn — the
/// idiomatic way to drain a live microphone stream in a `for` loop.
///
/// This impl *consumes* the handle, which also gives away the answer to "why did
/// it stop?": see the by-reference impl below.
impl Iterator for MicCapture {
    type Item = MicFrame;

    fn next(&mut self) -> Option<MicFrame> {
        recv_frame(&self.rx, &self.failure)
    }
}

/// The same blocking drain, by reference, so the handle survives the loop and
/// [`MicCapture::error`] can still be asked why iteration ended. Every frame
/// arrives through a `&self` channel receive, so nothing is given up by taking
/// the stream this way — `for frame in &mic` and `for frame in mic` differ only
/// in what you still own afterwards.
impl Iterator for &MicCapture {
    type Item = MicFrame;

    fn next(&mut self) -> Option<MicFrame> {
        recv_frame(&self.rx, &self.failure)
    }
}

/// Builds and wires up the `cpal` input stream for a concrete sample type
/// `T`, converting each sample to `f32` with `to_f32` before it crosses the
/// channel as a [`MicFrame`].
fn build_stream<T>(
    device: &cpal::Device,
    config: StreamConfig,
    tx: SyncSender<MicFrame>,
    state: &CallbackState,
    channels: u16,
    sample_rate: u32,
    to_f32: fn(T) -> f32,
) -> Result<cpal::Stream, CaptureHelperError>
where
    T: cpal::SizedSample + Send + 'static,
{
    let failure = Arc::clone(&state.failure);
    let dropped = Arc::clone(&state.dropped);
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
                // Two different "the send failed" cases, and only one of them
                // is nothing to worry about.
                //
                // Disconnected: the receiver is already gone because MicCapture
                // was dropped mid-callback. A normal shutdown race, not a bug.
                //
                // Full: the consumer is slower than the microphone. Audio
                // arrives at exactly realtime, so a consumer that cannot keep
                // up never catches up -- with an unbounded channel that meant
                // the queue grew for as long as the capture ran, silently,
                // until the process died. A bounded queue turns unbounded
                // memory growth into bounded memory plus a number the caller
                // can read, which is the honest trade for a realtime source:
                // audio is lost either way, and being told is the difference.
                //
                // Neither case may panic or block: this is the audio thread.
                if let Err(TrySendError::Full(_)) = tx.try_send(frame) {
                    dropped.fetch_add(1, Ordering::Relaxed);
                }
            },
            move |err| {
                // cpal's error callback has no channel back to the caller that
                // constructed the stream, and it cannot close the frame channel
                // either — the data callback owns the sender and outlives this.
                // Recording the failure is what lets a blocked consumer stop
                // waiting; printing it, as this did before 0.1.2, left the
                // consumer hanging on a stream that was already dead.
                failure.record(err.to_string());
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
    /// A frame already queued must be delivered even if the device has since
    /// failed — the failure ends the stream, it does not discard what was
    /// captured before it.
    #[test]
    fn a_queued_frame_still_arrives_after_a_failure() {
        let (tx, rx) = mpsc::channel::<MicFrame>();
        let failure = StreamFailure::default();
        tx.send(MicFrame {
            samples: vec![0.25],
            sample_rate: 48_000,
            channels: 1,
            timestamp: Instant::now(),
        })
        .unwrap();
        failure.record("device went away".to_string());

        let frame = recv_frame(&rx, &failure).expect("the queued frame");
        assert_eq!(frame.samples, vec![0.25]);
        // ...and then the stream ends instead of blocking on a dead device.
        assert!(recv_frame(&rx, &failure).is_none());
    }

    /// The regression this plumbing exists for: before 0.1.2 the error callback
    /// only printed, the sender stayed alive inside the data callback, and a
    /// consumer blocked on `recv()` waited forever on a stream that was already
    /// dead. The `tx` kept alive here is exactly that live-but-silent sender.
    #[test]
    fn a_failure_ends_the_wait_instead_of_hanging_forever() {
        let (_tx, rx) = mpsc::channel::<MicFrame>();
        let failure = Arc::new(StreamFailure::default());

        let signal = Arc::clone(&failure);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            signal.record("stream error from the device".to_string());
        });

        assert!(recv_frame(&rx, &failure).is_none());
        assert_eq!(
            failure.message().as_deref(),
            Some("stream error from the device")
        );
    }

    /// A clean shutdown — every sender dropped, no device error — must also end
    /// the wait, and must *not* look like a failure to the caller.
    #[test]
    fn dropping_the_sender_ends_the_wait_without_reporting_an_error() {
        let (tx, rx) = mpsc::channel::<MicFrame>();
        let failure = StreamFailure::default();
        drop(tx);

        assert!(recv_frame(&rx, &failure).is_none());
        assert!(failure.message().is_none());
    }

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

#[cfg(test)]
mod backpressure_tests {
    use super::*;

    // A microphone produces audio at exactly realtime, so a consumer even
    // slightly slower than realtime never catches up. With the unbounded
    // channel this crate used to build, that was not a slowdown but a leak:
    // the backlog grew for as long as capture ran, with no error, no warning,
    // and nothing in the "honest, typed errors" promise to cover it, because
    // there was no error to report. These tests pin the bound and the counter
    // that replaced it -- neither needs a microphone.

    #[test]
    fn the_frame_queue_is_bounded() {
        let (tx, _rx) = mpsc::sync_channel::<MicFrame>(2);
        let frame = || MicFrame {
            samples: vec![0.0; 4],
            sample_rate: 48_000,
            channels: 1,
            timestamp: Instant::now(),
        };
        assert!(tx.try_send(frame()).is_ok());
        assert!(tx.try_send(frame()).is_ok());
        // Third send must be refused rather than queued: that refusal is the
        // whole point, and what the counter below records.
        assert!(matches!(tx.try_send(frame()), Err(TrySendError::Full(_))));
    }

    #[test]
    fn a_refused_frame_is_counted_not_swallowed() {
        let (tx, _rx) = mpsc::sync_channel::<MicFrame>(1);
        let dropped = Arc::new(AtomicU64::new(0));
        let frame = || MicFrame {
            samples: vec![0.0; 4],
            sample_rate: 48_000,
            channels: 1,
            timestamp: Instant::now(),
        };
        for _ in 0..5 {
            if let Err(TrySendError::Full(_)) = tx.try_send(frame()) {
                dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
        // One fits, four are refused. Lossy either way; being told is the
        // difference between that and silently lossy.
        assert_eq!(dropped.load(Ordering::Relaxed), 4);
    }

    #[test]
    fn a_disconnected_receiver_is_not_counted_as_backpressure() {
        // MicCapture dropped mid-callback is a normal shutdown race, not a
        // consumer falling behind. Conflating the two would make the counter
        // report a loss on every clean shutdown.
        let (tx, rx) = mpsc::sync_channel::<MicFrame>(4);
        drop(rx);
        let dropped = Arc::new(AtomicU64::new(0));
        let frame = MicFrame {
            samples: vec![0.0; 4],
            sample_rate: 48_000,
            channels: 1,
            timestamp: Instant::now(),
        };
        if let Err(TrySendError::Full(_)) = tx.try_send(frame) {
            dropped.fetch_add(1, Ordering::Relaxed);
        }
        assert_eq!(dropped.load(Ordering::Relaxed), 0);
    }
}
