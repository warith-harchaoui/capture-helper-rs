---
name: capture-helper-rs
description: >-
  Live microphone capture in Rust, on top of cpal: list the system's audio input devices, find which one is the default, and stream it as MicFrame packets of f32 PCM that a pipeline drains frame by frame, blocking or not. The CLI can also record straight to a WAV. Local-first: no service, no upload, no account. Use when the user is writing Rust and says "list my microphones", "what audio inputs are available", "record from the mic", "capture 10 seconds of audio", "stream live audio into my pipeline", "feed the microphone to a model", or names cargo add capture-helper-rs, capture_helper_rs, or a cpal wrapper. Do NOT use for anything with a camera, webcam, screen or video -- the Python twin capture-helper covers those. Do NOT use to read an audio file (audio-helper-rs), to download (youtube-helper-rs), to transcribe (vocal-helper), or to play audio back.
license: BSD-3-Clause
metadata:
  author: Warith Harchaoui
  contact: warithmetics@deraison.ai
  repository: https://github.com/warith-harchaoui/capture-helper-rs
  crate: https://crates.io/crates/capture-helper-rs
  language: rust
---

# capture-helper-rs

A live microphone, as a stream of typed PCM frames. Microphone only — the
camera half of the Python twin is not here.

## The surface

| Intent | CLI | Library |
|--------|-----|---------|
| What microphones exist | `devices` | `list_input_devices` |
| Which is the default | marked `*` in `devices` | `default_input_device_name` |
| Capture live audio | `record` | `MicCapture`, `MicFrame` |

`MicCapture` is the real surface; `record` is one use of it.

## Writing Rust against it

```rust
use capture_helper_rs::MicCapture;

let mic = MicCapture::from_default_device()?;
for frame in &mic {
    // frame.samples: Vec<f32>, frame.sample_rate, frame.channels, frame.timestamp
    pipeline.push(&frame.samples);
}
// Iterating by reference keeps the handle, so this still has an answer:
if let Some(why) = mic.error() { eprintln!("capture ended: {why}"); }
```

Iterate **by reference**. Iterating by value consumes the handle, and with it
the only way to ask why the stream ended.

## Three things that look like bugs and are not

**An empty device list is an answer**, not an error: nothing is plugged in.
Only a genuine host enumeration failure returns `Err`.

**`default_input_device_name()` returning `None`** is the ordinary state of a
headless machine. It is also display-only — on ALSA the default's name is not
in the enumeration at all, so open the default with `from_default_device()`,
never by feeding that name back to `from_named_device()`.

**`try_next_frame()` returning `None` is ambiguous by design**: nothing yet, or
nothing ever. `next_frame()` plus `error()` are what distinguish the two.
`cpal` reports stream failures through a callback with no path back to the
caller, which is why the message is parked on the handle rather than printed.

## The CLI's WAV output

`record` writes 32-bit float by default, because that is exactly what the
device handed over, with no quantisation on the way out. Some readers only
accept integer PCM — Python's standard-library `wave` module among them — so
`--pcm16` trades a little precision for being readable everywhere.

## Choosing this crate over the Python twin

Pick this one for Rust calling code and live frames with no interpreter in the
loop. Pick the Python `capture-helper` for the camera, the multi-source scene
GUI, video mixing, the HTTP API or MCP.

Both install side by side: this binary is `capture-helper-rs`, not
`capture-helper`.

## References

- `references/triggers.md` — the full catalogue, mirroring the root `TRIGGERS.md`.
- `references/cli-reference.md` — every subcommand, flag and exit code.
- `references/surfaces.md` — which surfaces exist here, and which do not.
