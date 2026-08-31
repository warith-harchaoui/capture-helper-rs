use std::time::Instant;

/// One chunk of raw microphone audio, delivered as soon as the underlying
/// `cpal` callback hands it over. Samples are always normalized to `f32` in
/// `[-1.0, 1.0]` regardless of the device's native sample format (`i16`,
/// `u16`, `f32`, ...), interleaved across `channels` the way `cpal` delivers
/// them.
///
/// This mirrors the shape (not the exact fields) of the Python
/// `capture-helper`'s `MicFrame` typed dict — the point of parity is "a small
/// chunk of PCM audio plus enough metadata to interpret it", not identical
/// field names across languages.
#[derive(Debug, Clone)]
pub struct MicFrame {
    /// Interleaved PCM samples, normalized to `[-1.0, 1.0]`.
    pub samples: Vec<f32>,
    /// Sample rate in Hz, as reported by the device's active configuration.
    pub sample_rate: u32,
    /// Number of interleaved channels in `samples`.
    pub channels: u16,
    /// Local capture time of this chunk, useful for latency/drift diagnostics.
    /// Not wall-clock time — see [`std::time::Instant`].
    pub timestamp: Instant,
}
