#![cfg_attr(dylint_lib = "perfectionist", feature(register_tool))]
#![cfg_attr(dylint_lib = "perfectionist", register_tool(perfectionist))]

//! Domain-agnostic building blocks for an interactive fuzzy selector.
//!
//! The crate provides two layers that a terminal front-end can build upon:
//! the text-matching primitives in [`fuzzy`] (a substring filter, a
//! subsequence match, and a "resolve to exactly one candidate" helper for
//! command-line flags) and the pure selector state in [`selection`] (the
//! query, the matching rows, and the cursor). Neither layer touches a
//! terminal, so both are unit tested without a TTY.

pub mod fuzzy;
pub mod selection;
