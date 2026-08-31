use thiserror::Error;

/// Every fallible operation in this crate crosses this type. Each cpal failure
/// mode gets its own distinct variant instead of a single opaque wrapper, so
/// callers (e.g. scribe-reunion's live-capture path) can match on *why*
/// capture didn't start rather than parsing an error string.
#[derive(Debug, Error)]
pub enum CaptureHelperError {
    /// The host exposes no default input device at all (e.g. a headless
    /// machine, or microphone access denied at the OS level).
    #[error("no audio input device available on this host")]
    NoInputDevice,

    /// A device was requested by name via [`crate::MicCapture::from_named_device`]
    /// but no input device with that exact name was found.
    #[error("no input device found matching name: {0}")]
    DeviceNotFound(String),

    /// The host itself failed to enumerate input devices (distinct from
    /// simply having zero devices, which is not an error).
    #[error("failed to enumerate audio input devices: {0}")]
    DeviceEnumeration(String),

    /// The device was found but its input configuration could not be read.
    #[error("failed to read input device configuration: {0}")]
    StreamConfig(String),

    /// The device's default sample format is not one this crate knows how
    /// to normalize to `f32`.
    #[error("unsupported input sample format: {0}")]
    UnsupportedSampleFormat(String),

    /// `cpal` refused to build the input stream for the resolved config.
    #[error("failed to build audio input stream: {0}")]
    StreamBuild(String),

    /// The stream was built but failed to start playing.
    #[error("failed to start audio input stream: {0}")]
    StreamPlay(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_input_device_has_a_stable_message() {
        assert_eq!(
            CaptureHelperError::NoInputDevice.to_string(),
            "no audio input device available on this host"
        );
    }

    #[test]
    fn device_not_found_message_includes_the_requested_name() {
        let err = CaptureHelperError::DeviceNotFound("Yeti Nano".to_string());
        assert!(err.to_string().contains("Yeti Nano"));
    }

    #[test]
    fn unsupported_sample_format_message_includes_the_format() {
        let err = CaptureHelperError::UnsupportedSampleFormat("I64".to_string());
        assert!(err.to_string().contains("I64"));
    }
}
