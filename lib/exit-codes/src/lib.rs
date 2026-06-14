#![cfg_attr(dylint_lib = "perfectionist", feature(register_tool))]
#![cfg_attr(dylint_lib = "perfectionist", register_tool(perfectionist))]

//! Shared process exit-code constants for the workspace's binaries.

/// Exit code for a user-cancelled action, following the shell convention of
/// 128 plus the signal number (SIGINT is 2).
pub const CANCELLED: u8 = 130;
