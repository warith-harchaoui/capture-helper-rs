# TRIGGERS: capture-helper-rs

The exhaustive catalogue of what `capture-helper-rs` does and what should reach
for it. The Python twin, [`capture-helper`](https://github.com/warith-harchaoui/capture-helper),
has its own catalogue; the two overlap, and the section on choosing between
them is the part that matters.

`capture-helper-rs` turns a **live microphone** into a stream of PCM frames, via
[`cpal`](https://crates.io/crates/cpal). It is local-first: no service, no
upload, no account. It covers the microphone half of the original only — there
is **no camera support here**.

## The three operations → how to invoke

| Intent | CLI | Library |
|--------|-----|---------|
| What microphones exist? | `capture-helper-rs devices` | `list_input_devices` |
| Which one is the default? | marked `*` in `devices` | `default_input_device_name` |
| Capture live audio | `capture-helper-rs record` | `MicCapture`, `MicFrame` |

`MicCapture` is the real surface: an iterator of `MicFrame` that a pipeline
drains frame by frame, blocking (`next_frame`) or not (`try_next_frame`). The
`record` subcommand is one use of it — drain for N seconds, write a WAV.

## Natural-language phrasings that should fire

- **Devices**: "list my microphones", "what audio inputs are available",
  "which mic would be used by default", "enumerate input devices".
- **Record**: "record from the microphone", "capture 10 seconds of audio",
  "save what the mic hears to a wav".
- **Stream**: "stream the microphone into my pipeline", "feed live audio to a
  model", "iterate over mic frames", "live PCM in Rust", "real-time capture".
- **The crate itself**: `cargo add capture-helper-rs`, `cargo install
  capture-helper-rs`, "the Rust capture helper", "capture_helper_rs", "cpal
  wrapper".

## Two answers that look like failures and are not

- **An empty device list** is honest: no microphone is plugged in. Only a real
  host enumeration failure is an error.
- **`default_input_device_name()` returning `None`** is the ordinary state of a
  headless machine. It is also display-only: on ALSA the default's name does
  not appear in the enumeration, so open the default with
  `from_default_device()` rather than by passing that name to
  `from_named_device()`.

## Choosing between the Rust crate and the Python twin

Reach for **`capture-helper-rs`** when the calling code is Rust, when you want
live frames with no Python runtime in the loop, or when a single binary is
easier to deploy than an interpreter.

Reach for the **Python `capture-helper`** when you need the **camera**, the
multi-source scene GUI, video mixing, the HTTP API or the MCP tool set. None of
those exist here.

Both install side by side. The binary is deliberately named
`capture-helper-rs` rather than `capture-helper`, so neither shadows the other
on `PATH`.

## When NOT to use capture-helper-rs (SKIP)

- Anything with a **camera**, webcam, screen or video → the Python
  `capture-helper`.
- Reading an audio **file** rather than a live device → `audio-helper-rs`.
- Downloading audio from a URL → `youtube-helper-rs` or `podcast-helper-rs`.
- Transcription or speech-to-text → `vocal-helper`.
- Editing, converting or trimming what was captured → `audio-helper-rs`.
- Playback. This crate records; it does not play.

## See also

- [`README.md`](README.md) / [`LISEZMOI.md`](LISEZMOI.md): features and install.
- [`CHANGELOG.md`](CHANGELOG.md): what changed, version by version.
