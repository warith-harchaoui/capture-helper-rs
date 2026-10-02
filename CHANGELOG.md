# Changelog

All notable changes to `capture-helper-rs` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and the project adheres to
[Semantic Versioning](https://semver.org/).

## [0.1.2] — 2026-10-02

A dead microphone now ends the loop instead of hanging it.

### Fixed
- **A stream that failed mid-capture left every consumer blocked forever.** `cpal` reports
  device failures through a callback that has no return path to the caller and cannot close
  the frame channel — the data callback owns the sender and outlives it. So the failure was
  printed to stderr and dropped, while `next_frame()` / `for frame in mic` sat on a channel
  nothing would ever write to again. The failure is now recorded in shared state that the
  blocking wait checks, so iteration ends when the device dies. Covered by three tests that
  exercise the plumbing without audio hardware (`a_failure_ends_the_wait_instead_of_hanging_forever`
  and friends).

### Added
- **`MicCapture::error()`** — the device error that ended the capture, if one did. This is the
  replacement for the stderr line: the message is handed back rather than printed, so a caller
  can tell a failure apart from an ordinary shutdown.
- **`Iterator for &MicCapture`** — iterate by reference and keep the handle, so
  `MicCapture::error()` can still be asked why the loop ended. The consuming impl gives that
  answer away with the handle; both drain the same channel, they differ only in what you still
  own afterwards.
- **`default_input_device_name()`** — which microphone `from_default_device` would open,
  without opening it. An `Option`, not a `Result`: "no default input" is the ordinary state of
  a headless machine, not a failure. For display, not for round-tripping: the name is not
  guaranteed to appear in `list_input_devices()`. ALSA on a headless runner calls its default
  `"Default Audio Device"` while listing the same device as `"Discard all samples (playback) or
  generate zero samples (capture)"` — found by CI, which is where a claim like that gets tested.

### Changed
- A library no longer writes to stderr on its own initiative; see `MicCapture::error()` above.
- **CI is one Linux job instead of a three-OS matrix.** The heavy, cross-platform testing is
  the local pre-push gate's job; CI confirms, it does not discover.
- `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]` replace `#![warn(missing_docs)]`, so
  both hold locally and not only under CI's `-D warnings`.
- `rust-version = "1.85"` is now declared. Verified, not guessed: the full suite passes on
  1.85.0, which is also the first release to carry edition 2024.

## [0.1.1] — 2026-09-05

### Changed
- Repo hygiene only, no library change. Added `scripts/check-fresh-resolve.sh` — it builds the
  crate with no `Cargo.lock`, the way a downstream consumer resolves it — and wired it into CI
  and a versioned `.githooks` pre-push gate.
- Every public item documented, so docs.rs carries the whole API.

### Fixed
- The pre-push gate never actually ran: a global `core.hooksPath` (installed by git-lfs)
  silently shadowed `.git/hooks`.

## [0.1.0] — 2026-09-05

First release. Live microphone capture on top of `cpal`: cross-platform input-device
enumeration (`list_input_devices`) and a streaming `MicFrame` API (`MicCapture`), with samples
normalized to `[-1.0, 1.0]` whatever the device's native format.

### Fixed before release
- `i16 → f32` normalization divided by `i16::MAX` (32767), sending `i16::MIN` to -1.0000305 —
  outside the `[-1.0, 1.0]` the crate documents. Now divides by 32768.
