#![cfg_attr(dylint_lib = "perfectionist", feature(register_tool))]
#![cfg_attr(dylint_lib = "perfectionist", register_tool(perfectionist))]

//! Shared process exit-code constants for the workspace's binaries.

/// The base a shell adds to a signal number to form the exit code of a
/// process killed by that signal.
const SIGNAL_TERMINATION_BASE: u8 = 128;

/// The POSIX number of the interactive interrupt signal (SIGINT).
const SIGINT: u8 = 2;

/// Exit code for a user-cancelled action: the shell convention of the
/// signal-termination base plus the interrupt signal number.
pub const CANCELLED: u8 = SIGNAL_TERMINATION_BASE + SIGINT;
