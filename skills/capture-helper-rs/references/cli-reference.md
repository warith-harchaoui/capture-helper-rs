# CLI reference — `capture-helper-rs`

Installed with `cargo install capture-helper-rs`. The binary is named after the
crate, not after the Python twin's `capture-helper`, so both can sit on `PATH`
without either shadowing the other.

The binary is built only with the `cli` feature, which is on by default. A
library consumer who wants neither `clap` nor `hound` in their dependency graph
depends on the crate with `default-features = false`.

## Output contract

- Success: the result on **stdout**, exit `0`.
- Failure: one line on **stderr** prefixed `capture-helper-rs: `, exit `1`.

## `devices`

```
capture-helper-rs devices
```

One device per line, the default marked with a leading `*`. Prints nothing and
still exits `0` when no microphone is plugged in — an empty list is an honest
answer, not a failure.

The name marked `*` is for display. Do not feed it back to `--device`: on ALSA
the default's reported name does not appear in the enumeration at all, so the
round trip fails on exactly the machines where it matters. Omit `--device` to
get the default.

## `record`

```
capture-helper-rs record <OUTPUT> --seconds N [--device NAME] [--pcm16]
```

Records for `N` seconds and prints the written path.

- `--device` takes a name from `devices`; omit it for the system default.
- `--pcm16` writes 16-bit integer samples instead of 32-bit float.

**On the sample format.** Float is the default because it is exactly what the
device handed over, with no quantisation step on the way out. Some readers only
accept integer PCM — Python's standard-library `wave` module among them, though
ffmpeg, soundfile and librosa all read float WAVs — so `--pcm16` trades a
little precision for being readable everywhere. With `--pcm16`, samples are
clamped to `[-1, 1]` before scaling: a device can hand back a value slightly
outside that range, and wrapping it into `i16` turns a loud peak into a loud
click.

**The WAV's format follows the device, not a flag.** `cpal` picks the sample
rate and channel count; the header cannot be written until the first frame
arrives and says what they are.

**If the device disappears mid-recording**, what was captured is finalised and
kept, and the command exits `1` with a message naming both the reason and the
file. Losing a partial recording to report an error would be the worse trade.
