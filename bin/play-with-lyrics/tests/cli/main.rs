//! End-to-end tests that drive the compiled binary. The [`errors`] module
//! covers paths that return before any player launches; the [`playback`]
//! module runs the binary against fake player programs on `PATH`
//! (Unix-only), so no real media player is spawned, and asserts on the
//! arguments the fake player was launched with.

mod env;
mod errors;
#[cfg(unix)]
mod playback;
