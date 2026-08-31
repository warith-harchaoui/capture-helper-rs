//! `capture-helper-rs` — live microphone capture for Rust desktop apps.
//!
//! This is a Rust port of the *intent* of the Python
//! [`capture-helper`](https://github.com/warith-harchaoui/capture-helper)'s
//! `iter_mic_audio`: turn a live microphone into a stream of small audio
//! chunks with honest, typed errors. It is a deliberately narrow v0.1 — see
//! the crate README for exactly what is (and is not) in scope.
//!
//! ```no_run
//! use capture_helper_rs::{list_input_devices, MicCapture};
//!
//! // Enumerate available input devices.
//! for name in list_input_devices().unwrap() {
//!     println!("input device: {name}");
//! }
//!
//! // Stream from the default microphone (needs real hardware to run).
//! let mic = MicCapture::from_default_device().unwrap();
//! for frame in mic.take(10) {
//!     println!("{} samples @ {} Hz", frame.samples.len(), frame.sample_rate);
//! }
//! ```

mod capture;
mod devices;
mod error;
mod frame;

pub use capture::MicCapture;
pub use devices::list_input_devices;
pub use error::CaptureHelperError;
pub use frame::MicFrame;
