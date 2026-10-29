# Landscape

[🇫🇷 PAYSAGE.md](https://github.com/warith-harchaoui/capture-helper-rs/blob/master/PAYSAGE.md) · 🇬🇧 English

Related and competing crates in the "live microphone capture in Rust" space, benchmarked against `capture-helper-rs`.
Ratings are ⭐ (1) to ⭐⭐⭐⭐⭐ (5), scored on `capture-helper-rs`'s intended job:
turning a live microphone into a stream of small, typed audio frames (enumerate devices, open one, read frames blocking or non-blocking, and find out why a stream ended). A crate optimised for a very different job is not penalised; the score
just reflects fit to *this* niche.

## At a glance

<!-- TABLE:START -->
| Rust Mic Capture | Device enumeration | Streaming frame API | f32 normalisation | Backpressure made visible | Typed errors | Resampling / channel mixing | Voice-activity detection | Light dependency tree |
| --- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **capture-helper-rs** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐ | ⭐ | ⭐⭐⭐⭐⭐ |
| cpal | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐ | ⭐⭐ | ⭐⭐⭐ | ⭐ | ⭐ | ⭐⭐⭐⭐⭐ |
| libmic-rs | ⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐ | ⭐ | ⭐⭐ | ⭐ | ⭐ | ⭐⭐⭐⭐ |
| decibri | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐ |
| rodio | ⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐ | ⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐ | ⭐⭐⭐ |
| libpulse-binding | ⭐⭐ | ⭐⭐⭐ | ⭐ | ⭐ | ⭐⭐ | ⭐⭐ | ⭐ | ⭐⭐ |
<!-- TABLE:END -->

## Positioning map

<!-- FIGURE:START -->
2D representation of the table above.

![Positioning map](https://raw.githubusercontent.com/warith-harchaoui/capture-helper-rs/master/assets/landscape.png)

The map is a 2-D summary of the eight criteria, so read it as a shape, not a scoreboard. `capture-helper-rs` is at the top-right corner. The axes read **Horizontal: Adaptable ↔ Connected** and **Vertical: Elegant ↔ Responsive**.

The horizontal axis separates crates that stay light and do one thing from crates that carry DSP: a light dependency tree loads positively to the right, while resampling and voice-activity detection load negatively. The vertical axis is about accountability — whether the crate normalises what the device gave and tells you what happened to your frames.

`capture-helper-rs` and `decibri` share the top edge, because both normalise samples and both account for frames. They sit at opposite ends horizontally, and that is the whole choice: `decibri` brings resampling and VAD, this crate refuses both and stays at two dependencies.

The most useful thing on this map is `cpal` sitting directly below `capture-helper-rs` at the same horizontal position. Same niche, same lightness, lower on accountability — which is exactly the gap this crate exists to fill, since `cpal` reports stream failures through a callback with no path back to the caller. `libpulse-binding` sits bottom-left as the raw, Linux-only option.
<!-- FIGURE:END -->

## Where the claims come from

Versions and publication dates read from the crates.io API on 29 October 2026:

| Crate | Version | Last published |
|---|---|---|
| `cpal` | 0.18.2 | 2026-08-16 |
| `libmic-rs` | 0.1.0 | 2025-05-29 |
| `decibri` | 6.4.0 | 2026-10-04 |
| `rodio` | 0.23.0 | 2026-10-11 |

`capture-helper-rs`'s own ratings are from this repository at version 0.1.3.

- **`cpal` is not a competitor — it is the floor this crate stands on** (`cpal = "0.18"`, which is the current major). It scores 2 on backpressure and 3 on typed errors not as a defect but because it reports stream failures through a callback with no path back to the caller; the whole point of `MicCapture::error()` is to give that answer somewhere a caller can read it.

- **The two 1s on this crate's own row are real and deliberate.** There is no resampling, no mono/stereo conversion and no voice-activity detection: a `MicFrame` carries exactly what the device gave, normalised to `f32`. `decibri` is the crate to reach for when conditioning and VAD are the requirement.

- **`decibri` scores 2 on the dependency tree** because conditioning and VAD bring their own weight; this crate ships `cpal` plus `thiserror`, with `clap` and `hound` only behind the opt-in `cli` feature.

Deliberately left as opinion rather than sourced: the *pipeline ergonomics*
style columns, and the ordering of the rows below the reference.

## Ideas worth stealing

Researched rather than guessed: what each neighbour does better, and what this crate would have to change to match it. Nothing here is a commitment — it is the reading list the next version starts from.

- **`decibri` — voice-activity detection, behind a feature** — The clearest missing capability, and one of this crate's two 1s. `decibri` does VAD on live and recorded audio. Every consumer of a microphone stream eventually wants to know where speech is; leaving it out means each of them bolts on their own.

- **`libmic-rs` — record straight to WAV as a library call** — The CLI already writes a WAV through `hound`. `libmic-rs`'s entire reason to exist is that one call, and exposing it from the library — not only the binary — would cost almost nothing.

- **`cpal` — device selection by index or substring** — This crate accepts an exact name or the default device, and nothing else. `cpal` exposes the host enumeration that would make a substring match possible, and its host abstraction is also the pointer toward SoundIO and PulseAudio backends.
