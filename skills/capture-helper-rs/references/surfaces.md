# Surfaces — capture-helper-rs

The Python suite's rule is that a helper exposes five surfaces: a library, two
CLIs, a FastAPI HTTP server, an MCP tool set mounted on that same app, and an
installable agent skill. This crate carries three.

## What exists

**Library** — `cargo add capture-helper-rs`. The primary surface, and the only
one that reaches the streaming API: `MicCapture` as an iterator of `MicFrame`.
A command line can hand back a file; it cannot hand back a live stream.

**CLI** — `cargo install capture-helper-rs` gives the `capture-helper-rs`
binary, behind the default-on `cli` feature.

**Agent skill** — this directory.

## What does not exist, and why

**No camera.** This is the biggest gap against the Python twin and it is
deliberate: the original's camera half, multi-source scene GUI and video mixing
are out of scope here. The crate covers the microphone, completely.

**No HTTP API.** Streaming audio over HTTP is a different product with
different questions — backpressure, framing, authentication. The Python twin
serves the request/response surface; this crate feeds an in-process pipeline.

**No MCP.** The suite's rule is that MCP is *never* a second implementation:
`fastapi-mcp` mounts `/mcp` onto the FastAPI app the helper already serves, so
every MCP tool **is** one of its routes and the two cannot drift apart. Rust
has no equivalent of that mounting, so an MCP surface here would necessarily be
a separate implementation — exactly the drift the rule exists to prevent.

**Only one CLI, not two.** The Python side ships an argparse CLI and a click
twin because both parsers are idiomatic there. Rust has one idiomatic answer,
`clap`.

## Choosing between the two implementations

| Need | Reach for |
|---|---|
| Rust calling code, live frames, no interpreter | `capture-helper-rs` |
| A single binary to deploy | `capture-helper-rs` |
| Camera, webcam, screen, video | Python `capture-helper` |
| Multi-source scene GUI, video mixing | Python `capture-helper` |
| HTTP API, MCP tools | Python `capture-helper` |
