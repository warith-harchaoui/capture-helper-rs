# Capture Helper (Rust)

[🇫🇷](https://github.com/warith-harchaoui/capture-helper-rs/blob/master/LISEZMOI.md) · [🇬🇧](https://github.com/warith-harchaoui/capture-helper-rs/blob/master/README.md)

[![crates.io](https://img.shields.io/crates/v/capture-helper-rs.svg)](https://crates.io/crates/capture-helper-rs) [![License: BSD-3-Clause](https://img.shields.io/badge/License-BSD%203--Clause-blue.svg)](./LICENSE)

Rust rewrite of the microphone half of [`capture-helper`](https://github.com/warith-harchaoui/capture-helper): turn a live microphone into a stream of small audio packets (PCM) that the rest of an audio pipeline can consume, with no third-party service in the loop.

This is not a line-by-line port. The camera half of the original (`iter_camera_frames`, the multi-source scene GUI, video mixing) is out of scope for v0.1 — only the microphone is covered.

## What this crate does (v0.1)

- `list_input_devices() -> Result<Vec<String>, CaptureHelperError>` — enumerates the system's audio input devices via [`cpal`](https://crates.io/crates/cpal). An empty list is an honest answer (no microphone plugged in); only a real host enumeration failure returns an error.
- `MicCapture` — opens a microphone stream (default or named device) and exposes it as an iterator of `MicFrame`: blocking via `for frame in mic { ... }` / `next_frame()`, or non-blocking via `try_next_frame()`.
- `MicFrame { samples: Vec<f32>, sample_rate: u32, channels: u16, timestamp: Instant }` — a PCM packet normalized to `f32` in `[-1.0, 1.0]`, regardless of the device's native sample format (`f32`, `i16`, `u16`).
- `CaptureHelperError` (via `thiserror`) — one variant per failure mode: no default device, named device not found, enumeration failure, stream-config failure, unsupported sample format, stream-build failure, stream-start failure. No catch-all `String`.

## What this crate does not do (yet, or at all)

- No camera, no images, no GUI — that half of the Python original stays there.
- No resampling, no mono/stereo conversion, no voice-activity detection: `MicFrame` delivers exactly what the device gives, normalized to `f32`, nothing more.
- No device selection by index or name substring (exact name or default device only).

## Example

```rust
use capture_helper_rs::{list_input_devices, MicCapture};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for name in list_input_devices()? {
        println!("input device: {name}");
    }

    let mic = MicCapture::from_default_device()?;
    for frame in mic.take(50) {
        println!("{} samples @ {} Hz, {} channel(s)", frame.samples.len(), frame.sample_rate, frame.channels);
    }

    Ok(())
}
```

## Test coverage — read before trusting CI

This environment (and probably yours) has no real microphone attached. Concretely:

- `list_input_devices()` **is** tested: it must always return a `Vec` (possibly empty) without panicking.
- Error paths reachable without hardware **are** tested: asking for a named device that clearly doesn't exist must produce `CaptureHelperError::DeviceNotFound` (or `DeviceEnumeration` if the host has no audio subsystem at all).
- Actually capturing a stream (`MicCapture::from_default_device()` receiving real samples, or `from_named_device()` on a name that truly exists) **is not tested in CI** — it needs a physical microphone and an OS permission grant. No test in this repository claims to verify that path; it's a manual check to run on a machine with a microphone before relying on it in production.

## Project status

Coverage measured with [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov) on 2026-08-31, on macOS (LLVM tools via Xcode):

| File | Line coverage |
|---|---|
| `src/error.rs` | 100.00% |
| `src/devices.rs` | 92.86% |
| `src/capture.rs` | 15.69% |
| **Total** | **32.03%** (128 lines, 87 uncovered) |

The overall figure is low because most of the uncovered code in `capture.rs` is exactly the path described above (building and reading a real `cpal` stream): it cannot be exercised without a physical microphone, and this crate does not fake a device just to inflate a percentage. `error.rs` and `devices.rs` — the code reachable without hardware — sit at 92–100%.

To reproduce:

```bash
cargo install cargo-llvm-cov
# macOS without rustup: point at Xcode's LLVM tools
export LLVM_COV=$(xcrun --find llvm-cov)
export LLVM_PROFDATA=$(xcrun --find llvm-profdata)
# with rustup (Linux/Windows/macOS): rustup component add llvm-tools-preview

cargo llvm-cov --summary-only
```

## Installation

```toml
[dependencies]
capture-helper-rs = "0.1"
```

Requires a recent stable Rust toolchain. `cpal` handles CoreAudio (macOS), WASAPI (Windows), and ALSA/PulseAudio/JACK/PipeWire (Linux) natively — no separate `ffmpeg`/`PortAudio` install needed.

## Checks before pushing

```bash
cargo build
cargo test
cargo clippy --all-targets
```

## Related

Part of the same author's local-first tooling as [`capture-helper`](https://github.com/warith-harchaoui/capture-helper) (Python) and the [AI Helpers](https://github.com/warith-harchaoui/ai-helpers) suite. Independent rewrite, not a binding.

## Author

- [Warith HARCHAOUI](https://linkedin.com/in/warith-harchaoui)

## License

BSD-3-Clause — see [LICENSE](./LICENSE).
