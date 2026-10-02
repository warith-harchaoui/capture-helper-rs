//! `capture-helper-rs` — live microphone capture for Rust desktop apps.
//!
//! This is a Rust port of the *intent* of the Python
//! [`capture-helper`](https://github.com/warith-harchaoui/capture-helper)'s
//! `iter_mic_audio`: turn a live microphone into a stream of small audio
//! chunks with honest, typed errors. It is a deliberately narrow v0.1 — see
//! the crate README for exactly what is (and is not) in scope.
//!
//! ```no_run
//! use capture_helper_rs::{default_input_device_name, list_input_devices, MicCapture};
//!
//! // Enumerate available input devices, and see which one is the default.
//! for name in list_input_devices().unwrap() {
//!     println!("input device: {name}");
//! }
//! println!("default: {:?}", default_input_device_name());
//!
//! // Stream from the default microphone (needs real hardware to run).
//! let mic = MicCapture::from_default_device().unwrap();
//! for frame in (&mic).take(10) {
//!     println!("{} samples @ {} Hz", frame.samples.len(), frame.sample_rate);
//! }
//! // Iteration ends either because you stopped asking or because the device
//! // failed — this is how you tell which.
//! if let Some(err) = mic.error() {
//!     eprintln!("capture stopped: {err}");
//! }
//! ```

// Every public item carries a doc comment, and no `unsafe` appears anywhere in this
// crate — both are enforced here rather than left to review. `deny` (not `warn`) so
// the gate holds locally too, not only under CI's `-D warnings`.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod capture;
mod devices;
mod error;
mod frame;

pub use capture::MicCapture;
pub use devices::{default_input_device_name, list_input_devices};
pub use error::CaptureHelperError;
pub use frame::MicFrame;
