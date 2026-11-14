//! Output that works with a closed pipe. `println!` panics if the reader of
//! stdout is gone, for example in `spec-dna check --json | head -1`. These
//! functions ignore a `BrokenPipe` error, so the command ends with
//! its own exit code. Any other write error is a bug of the environment and
//! panics, as `println!` does. The macros of `main.rs` replace the macros of
//! the same name from `std` in the whole crate.

use std::fmt::Arguments;
use std::io::{self, Write};

fn done(stream: &str, result: io::Result<()>) {
    if let Err(e) = result
        && e.kind() != io::ErrorKind::BrokenPipe
    {
        panic!("failed printing to {stream}: {e}");
    }
}

pub fn stdout(args: Arguments) {
    done("stdout", io::stdout().lock().write_fmt(args));
}

pub fn stderr(args: Arguments) {
    done("stderr", io::stderr().lock().write_fmt(args));
}
