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
}
