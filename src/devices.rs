use crate::error::CaptureHelperError;
use cpal::traits::HostTrait;

/// Enumerate the names of every audio input device the default host exposes.
///
/// An empty `Vec` is a valid, honest answer (no input devices attached,
/// which is the normal state of a headless CI runner) — it is only an `Err`
/// when the host itself fails to enumerate devices at all.
///
/// Device names come from `cpal`'s `Display` impl on `Device` (the
/// cross-backend way to get a human-readable name in cpal 0.18), not from a
/// dedicated `name()` method.
pub fn list_input_devices() -> Result<Vec<String>, CaptureHelperError> {
    let host = cpal::default_host();
    let devices = host
        .input_devices()
        .map_err(|e| CaptureHelperError::DeviceEnumeration(e.to_string()))?;

    Ok(devices.map(|d| d.to_string()).collect())
}

/// The name of the host's default input device, or `None` if the host has none.
///
/// An `Option` rather than a `Result`: "no default input device" is the ordinary
/// state of a headless machine, not a failure. It is the same condition
/// [`crate::MicCapture::from_default_device`] turns into
/// [`crate::CaptureHelperError::NoInputDevice`] — this is the way to ask which
/// microphone that would be, without opening it.
///
/// **For display, not for round-tripping.** The name is not guaranteed to appear
/// in [`list_input_devices`], so do not feed it back into
/// [`crate::MicCapture::from_named_device`]. ALSA is the counter-example that
/// settled this: on a headless Linux box its default device reports itself as
/// `"Default Audio Device"` while enumeration lists the very same device as
/// `"Discard all samples (playback) or generate zero samples (capture)"`. To
/// open the default device, use [`crate::MicCapture::from_default_device`],
/// which never goes through a name at all.
pub fn default_input_device_name() -> Option<String> {
    cpal::default_host()
        .default_input_device()
        .map(|d| d.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `list_input_devices` must never panic and must always return a `Vec`,
    /// even in this headless CI/sandbox environment where no physical
    /// microphone is attached — the list may legitimately be empty here.
    ///
    /// What this test *cannot* verify: that the returned names are correct,
    /// or that a real microphone actually shows up on a machine that has
    /// one. That needs a human running this on real hardware; see the crate
    /// README for the explicit "not tested in CI" boundary.
    #[test]
    fn list_input_devices_does_not_panic_and_returns_a_list() {
        let result = list_input_devices();
        assert!(
            result.is_ok(),
            "list_input_devices should succeed (possibly with an empty list) \
             even with zero input devices attached: {result:?}"
        );
    }

    /// Asking which device is the default must never panic, on a machine with a
    /// microphone or without one, and must not answer with an empty name.
    ///
    /// What this deliberately does *not* assert is that the name appears in
    /// [`list_input_devices`]. It did until CI said otherwise: ALSA on a headless
    /// runner calls its default `"Default Audio Device"` while listing the same
    /// device as `"Discard all samples (playback) or generate zero samples
    /// (capture)"`. The two names are both real, and the function's documentation
    /// now says what that means — the name is for display, not for feeding back
    /// into `from_named_device`.
    #[test]
    fn default_input_device_name_is_absent_or_meaningful() {
        let Some(name) = default_input_device_name() else {
            return; // Headless host: no default input, which is a valid answer.
        };
        assert!(
            !name.trim().is_empty(),
            "a default input device reported an empty name"
        );
    }
}
