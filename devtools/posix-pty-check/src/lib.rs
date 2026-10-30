//! Standalone tests for the Linux host terminal's POSIX PTY implementation.
//!
//! Run with `cargo test --manifest-path devtools/posix-pty-check/Cargo.toml`.

// The app calls these `pub(crate)` items; here only the module's own tests
// do, so everything else would read as dead code.
#[allow(dead_code)]
#[path = "../../../src/platform/posix/terminal.rs"]
mod terminal;

// The included terminal reports foreground processes through this app type.
#[allow(dead_code)]
mod platform {
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) struct Foreground {
        pub(crate) name: String,
        pub(crate) command: String,
    }
}
