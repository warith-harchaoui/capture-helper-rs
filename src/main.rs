//! `capture-helper-rs` — the command-line front end for [`capture_helper_rs`].
//!
//! Thin by design, like the rest of this suite's binaries: parse arguments,
//! drive the library, print, exit.
//!
//! Built only with the `cli` feature, which is on by default so
//! `cargo install capture-helper-rs` works, and off for anyone who depends on
//! the crate as a library with `default-features = false` — they get neither
//! `clap` nor `hound` in their dependency graph.
//!
//! The binary is named after the crate rather than after the Python twin's
//! `capture-helper` command on purpose: the two are meant to coexist, and
//! identical names would let `PATH` order decide which one runs.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use capture_helper_rs::{MicCapture, default_input_device_name, list_input_devices};
use clap::{Parser, Subcommand};

/// Turn a live microphone into PCM, from the command line.
#[derive(Parser, Debug)]
#[command(name = "capture-helper-rs", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// List the system's audio input devices, marking the default one.
    Devices,
    /// Record from a microphone into a WAV file.
    Record {
        /// Where to write the WAV.
        output: PathBuf,
        /// How long to record, in seconds.
        #[arg(long)]
        seconds: f64,
        /// Device name from `devices`; omit to use the system default.
        #[arg(long)]
        device: Option<String>,
        /// Write 16-bit integer samples instead of 32-bit float.
        ///
        /// Float is the default because it is exactly what the device handed
        /// us, with no quantisation step on the way out. Some readers only
        /// accept integer PCM, though — Python's standard-library `wave`
        /// module among them — so this flag trades a little precision for
        /// being readable everywhere.
        #[arg(long)]
        pcm16: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let outcome = match cli.command {
        Command::Devices => devices(),
        Command::Record {
            output,
            seconds,
            device,
            pcm16,
        } => record(&output, seconds, device.as_deref(), pcm16),
    };

    match outcome {
        Ok(text) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("capture-helper-rs: {message}");
            ExitCode::FAILURE
        }
    }
}

/// One device per line, the default marked with a leading `*`.
///
/// An empty list is an honest answer — no microphone plugged in — not an
/// error, so it prints nothing and still exits 0.
fn devices() -> Result<String, String> {
    let names = list_input_devices().map_err(|e| e.to_string())?;
    let default = default_input_device_name();
    Ok(names
        .iter()
        .map(|name| {
            if Some(name.as_str()) == default.as_deref() {
                format!("* {name}")
            } else {
                format!("  {name}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

/// Drain frames for `seconds` and write them as a WAV.
///
/// The default device is opened with `from_default_device()` rather than by
/// looking its name up: on ALSA the default's reported name does not appear in
/// the enumeration at all, so round-tripping it through `from_named_device`
/// would fail on exactly the machines where it matters.
fn record(
    output: &PathBuf,
    seconds: f64,
    device: Option<&str>,
    pcm16: bool,
) -> Result<String, String> {
    if seconds <= 0.0 {
        return Err(format!("--seconds must be positive, got {seconds}"));
    }
    let mic = match device {
        Some(name) => MicCapture::from_named_device(name),
        None => MicCapture::from_default_device(),
    }
    .map_err(|e| e.to_string())?;

    // The first frame carries the stream's real format; cpal picks it, we do
    // not get to choose it, so the WAV header cannot be written before one
    // arrives.
    let first = mic.next_frame().ok_or_else(|| match mic.error() {
        Some(e) => format!("the microphone stream failed before any audio arrived: {e}"),
        None => "the microphone stream ended before any audio arrived".to_string(),
    })?;

    let spec = hound::WavSpec {
        channels: first.channels,
        sample_rate: first.sample_rate,
        bits_per_sample: if pcm16 { 16 } else { 32 },
        sample_format: if pcm16 {
            hound::SampleFormat::Int
        } else {
            hound::SampleFormat::Float
        },
    };
    let mut writer = hound::WavWriter::create(output, spec).map_err(|e| e.to_string())?;
    let mut written = 0usize;
    let write_all = |samples: &[f32], writer: &mut hound::WavWriter<_>| -> Result<(), String> {
        for sample in samples {
            if pcm16 {
                // Clamp before scaling: a device can hand back a sample just
                // outside [-1, 1], and wrapping that into i16 turns a loud
                // peak into a loud click.
                let clamped = sample.clamp(-1.0, 1.0);
                writer
                    .write_sample((clamped * i16::MAX as f32) as i16)
                    .map_err(|e| e.to_string())?;
            } else {
                writer.write_sample(*sample).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    };
    write_all(&first.samples, &mut writer)?;
    written += first.samples.len();

    let deadline = Instant::now() + Duration::from_secs_f64(seconds);
    while Instant::now() < deadline {
        match mic.next_frame() {
            Some(frame) => {
                write_all(&frame.samples, &mut writer)?;
                written += frame.samples.len();
            }
            // `None` from the blocking call means no frame will ever arrive —
            // the device went away. Keep what was captured rather than
            // discarding it, and say why it stopped short.
            None => {
                writer.finalize().map_err(|e| e.to_string())?;
                let reason = mic
                    .error()
                    .unwrap_or_else(|| "the device stopped producing audio".to_string());
                return Err(format!(
                    "recording ended early after {} frames: {reason} ({} kept)",
                    written,
                    output.display()
                ));
            }
        }
    }
    writer.finalize().map_err(|e| e.to_string())?;
    Ok(output.display().to_string())
}
