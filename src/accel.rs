//! Opening the microphone on the path the system already prefers — and letting
//! a caller leave that path deliberately, never by accident.
//!
//! # Why the default is already the right one
//!
//! There is no accelerator to choose here, and that is worth saying plainly
//! rather than inventing one. Capture is not compute: the audio arrives at
//! exactly real time no matter what hardware is underneath, and the only thing
//! that can be wrong is the *path* it travels.
//!
//! The fast path is the device's own default configuration, which is what this
//! crate opens:
//!
//! - **The native host.** `cpal` talks to CoreAudio on macOS, WASAPI on Windows,
//!   and ALSA (and so PulseAudio or PipeWire, which present themselves through
//!   it) on Linux. No PortAudio, no `ffmpeg`, no extra install.
//! - **The device's own rate and format.** Ask for 16 kHz from a microphone that
//!   runs at 48 kHz and the operating system inserts a resampler you did not
//!   choose, in a layer you cannot see, before you get a single sample. Opening
//!   the device's default rate skips that layer entirely; resampling afterwards,
//!   if you need to, is at least yours and visible.
//! - **The driver's own buffer size.** A smaller buffer is lower latency and
//!   more wake-ups; a larger one is the reverse. The driver's default is the
//!   compromise the platform chose for the general case, and beating it
//!   requires knowing something specific about the workload.
//!
//! # So what is configurable
//!
//! Exactly the three things above, because "best for this machine" is not
//! "best for every use". A live transcription path wants a small fixed buffer;
//! a long unattended recording wants a large one. Those are legitimate, and
//! they are explicit.
//!
//! What this module refuses to do is paper over an impossible request. Asking
//! for a sample rate the device does not support used to mean the operating
//! system quietly resampled; here it is an error that names what the device
//! actually offers. A hidden resampler on the capture path is how a recording
//! ends up subtly worse than its source with nothing in the logs to say so.

use std::fmt;

/// Environment variable naming the audio host to use (`"coreaudio"`, `"wasapi"`,
/// `"alsa"`, `"jack"`, ...). Matched case-insensitively against the hosts this
/// build of `cpal` can reach.
pub const HOST_ENV: &str = "CAPTURE_HELPER_HOST";

/// Environment variable forcing a capture sample rate, in hertz.
pub const SAMPLE_RATE_ENV: &str = "CAPTURE_HELPER_SAMPLE_RATE";

/// Environment variable forcing the capture buffer size, in frames.
pub const BUFFER_FRAMES_ENV: &str = "CAPTURE_HELPER_BUFFER_FRAMES";

/// What a caller asks for, over and above "whatever this machine prefers".
///
/// Every field is `None` by default, which means the native path described at
/// the top of this module.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CaptureOptions {
    /// Audio host to open, or `None` for the platform's own.
    pub host: Option<String>,
    /// Input device name, or `None` for the host's default input.
    pub device: Option<String>,
    /// Capture sample rate in hertz, or `None` for the device's own.
    ///
    /// A rate the device cannot produce is an error, not a silent resample.
    pub sample_rate: Option<u32>,
    /// Capture buffer size in frames, or `None` for the driver's own.
    pub buffer_frames: Option<u32>,
}

impl CaptureOptions {
    /// The options, with anything unset filled in from the environment.
    ///
    /// A field set in code always wins: the environment is for pinning a choice
    /// on one machine, not for overruling a caller who was specific.
    pub fn with_env(self) -> Self {
        self.with_lookup(|key| std::env::var(key).ok())
    }

    /// The merge rule itself, over any source of values.
    ///
    /// Split from [`CaptureOptions::with_env`] so the precedence can be tested
    /// without mutating the process environment — which this crate could not do
    /// anyway (it denies `unsafe`, and `set_var` is unsafe), and which would
    /// make the tests order-dependent under the default parallel runner.
    pub fn with_lookup(mut self, lookup: impl Fn(&str) -> Option<String>) -> Self {
        let value = |key: &str| {
            lookup(key)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        if self.host.is_none() {
            self.host = value(HOST_ENV);
        }
        if self.sample_rate.is_none() {
            self.sample_rate = value(SAMPLE_RATE_ENV).and_then(|v| v.parse().ok());
        }
        if self.buffer_frames.is_none() {
            self.buffer_frames = value(BUFFER_FRAMES_ENV).and_then(|v| v.parse().ok());
        }
        self
    }
}

/// One contiguous span of sample rates a device advertises.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateRange {
    /// Lowest rate in the span, in hertz.
    pub min: u32,
    /// Highest rate in the span, in hertz.
    pub max: u32,
}

impl RateRange {
    /// Whether `rate` falls inside this span.
    pub fn covers(&self, rate: u32) -> bool {
        rate >= self.min && rate <= self.max
    }
}

/// A rate the device cannot produce, carrying what it *can* so the message is
/// actionable rather than a refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedRate {
    /// What was asked for.
    pub requested: u32,
    /// What the device advertises instead.
    pub supported: Vec<RateRange>,
}

impl fmt::Display for UnsupportedRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the device cannot capture at {} Hz", self.requested)?;
        if self.supported.is_empty() {
            return write!(f, " and advertises no rate at all");
        }
        write!(f, "; it offers ")?;
        for (i, r) in self.supported.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            if r.min == r.max {
                write!(f, "{} Hz", r.min)?;
            } else {
                write!(f, "{}-{} Hz", r.min, r.max)?;
            }
        }
        Ok(())
    }
}

/// The rate to open the device at: the requested one when the device can
/// actually produce it, the device's own otherwise.
///
/// The error case is the point of the function. Returning the device default
/// after an impossible request would be a silent downgrade, and the caller would
/// find out by listening.
pub fn choose_rate(
    requested: Option<u32>,
    device_default: u32,
    supported: &[RateRange],
) -> Result<u32, UnsupportedRate> {
    let Some(rate) = requested else {
        return Ok(device_default);
    };
    if rate == device_default || supported.iter().any(|r| r.covers(rate)) {
        return Ok(rate);
    }
    Err(UnsupportedRate {
        requested: rate,
        supported: supported.to_vec(),
    })
}

/// Picks a host by name from the ones this build can reach, case-insensitively.
///
/// `None` asks for the platform's own host, which is the right answer unless you
/// know otherwise — the alternatives (JACK, ASIO) exist for latency work and are
/// compile-time features, so on most builds this list has one entry.
pub fn choose_host<'a>(
    requested: Option<&str>,
    available: &[&'a str],
) -> Result<Option<&'a str>, UnknownHost> {
    let Some(name) = requested.map(str::trim).filter(|n| !n.is_empty()) else {
        return Ok(None);
    };
    available
        .iter()
        .find(|h| h.eq_ignore_ascii_case(name))
        .map(|h| Some(*h))
        .ok_or_else(|| UnknownHost {
            requested: name.to_string(),
            available: available.iter().map(|h| (*h).to_string()).collect(),
        })
}

/// A host name that this build of `cpal` has no backend for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownHost {
    /// What was asked for.
    pub requested: String,
    /// What this build can reach.
    pub available: Vec<String>,
}

impl fmt::Display for UnknownHost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "no audio host named `{}`; this build reaches {}",
            self.requested,
            if self.available.is_empty() {
                "none".to_string()
            } else {
                self.available.join(", ")
            }
        )
    }
}

/// Clamps a requested buffer size into what the driver advertises, or `None`
/// to leave the driver's own default in place.
///
/// Clamping rather than refusing, unlike the sample rate, because a buffer
/// outside the advertised span is a *performance* preference the driver can
/// legitimately overrule — nothing about the audio changes — whereas a wrong
/// sample rate changes the data.
pub fn choose_buffer_frames(requested: Option<u32>, advertised: Option<(u32, u32)>) -> Option<u32> {
    let frames = requested?;
    match advertised {
        Some((min, max)) => Some(frames.clamp(min, max)),
        None => Some(frames),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_request_means_the_devices_own_rate() {
        let ranges = [RateRange {
            min: 44_100,
            max: 48_000,
        }];
        assert_eq!(choose_rate(None, 48_000, &ranges), Ok(48_000));
    }

    #[test]
    fn a_supported_rate_is_honoured() {
        let ranges = [RateRange {
            min: 8_000,
            max: 96_000,
        }];
        assert_eq!(choose_rate(Some(16_000), 48_000, &ranges), Ok(16_000));
    }

    #[test]
    fn an_impossible_rate_is_refused_rather_than_resampled() {
        // The whole reason this function exists: the operating system would
        // happily insert a resampler here, and nothing would say so.
        let ranges = [RateRange {
            min: 44_100,
            max: 48_000,
        }];
        let err = choose_rate(Some(16_000), 48_000, &ranges).unwrap_err();
        assert_eq!(err.requested, 16_000);
        assert!(err.to_string().contains("44100-48000 Hz"), "{err}");
    }

    #[test]
    fn the_device_default_is_trusted_over_an_incomplete_advertisement() {
        // Some drivers advertise nothing useful through `supported_input_configs`
        // yet run perfectly at their default. Refusing that rate because it is
        // missing from an empty list would make the device unusable.
        assert_eq!(choose_rate(Some(48_000), 48_000, &[]), Ok(48_000));
    }

    #[test]
    fn a_host_is_matched_case_insensitively() {
        let hosts = ["CoreAudio"];
        assert_eq!(
            choose_host(Some("coreaudio"), &hosts),
            Ok(Some("CoreAudio"))
        );
        assert_eq!(choose_host(None, &hosts), Ok(None));
        assert_eq!(choose_host(Some("  "), &hosts), Ok(None));
    }

    #[test]
    fn an_unknown_host_says_what_there_is() {
        let hosts = ["ALSA"];
        let err = choose_host(Some("jack"), &hosts).unwrap_err();
        assert!(err.to_string().contains("ALSA"), "{err}");
    }

    #[test]
    fn a_buffer_request_is_clamped_not_refused() {
        // A buffer preference the driver cannot honour changes timing, not data,
        // so it is bent to fit instead of failing the capture.
        assert_eq!(choose_buffer_frames(Some(64), Some((128, 4096))), Some(128));
        assert_eq!(
            choose_buffer_frames(Some(8192), Some((128, 4096))),
            Some(4096)
        );
        assert_eq!(choose_buffer_frames(Some(512), None), Some(512));
        assert_eq!(choose_buffer_frames(None, Some((128, 4096))), None);
    }

    #[test]
    fn a_field_set_in_code_beats_the_environment() {
        let env = |key: &str| (key == SAMPLE_RATE_ENV).then(|| "44100".to_string());

        let explicit = CaptureOptions {
            sample_rate: Some(16_000),
            ..Default::default()
        }
        .with_lookup(env);
        assert_eq!(explicit.sample_rate, Some(16_000));

        // ... and fills the field in when the caller had no opinion.
        let from_env = CaptureOptions::default().with_lookup(env);
        assert_eq!(from_env.sample_rate, Some(44_100));
    }

    #[test]
    fn an_unparseable_environment_value_is_ignored_not_fatal() {
        // A typo in a shell profile must not stop the microphone from opening.
        let env = |key: &str| (key == BUFFER_FRAMES_ENV).then(|| "beaucoup".to_string());
        assert_eq!(
            CaptureOptions::default().with_lookup(env).buffer_frames,
            None
        );
    }

    #[test]
    fn whitespace_in_the_environment_is_not_a_value() {
        let env = |key: &str| (key == HOST_ENV).then(|| "   ".to_string());
        assert_eq!(CaptureOptions::default().with_lookup(env).host, None);
    }
}
