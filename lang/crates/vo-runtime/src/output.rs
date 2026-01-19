//! Output capture for WASM/no_std environments.
//!
//! In std mode, print/println go directly to stdout.
//! In no_std mode (WASM), output is captured to a buffer that can be retrieved.

#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(not(feature = "std"))]
use core::cell::UnsafeCell;

/// Global output buffer for no_std mode (WASM is single-threaded).
/// SAFETY: WASM is single-threaded, so UnsafeCell access is safe.
#[cfg(not(feature = "std"))]
struct OutputBuffer(UnsafeCell<String>);

#[cfg(not(feature = "std"))]
unsafe impl Sync for OutputBuffer {}

#[cfg(not(feature = "std"))]
static OUTPUT_BUFFER: OutputBuffer = OutputBuffer(UnsafeCell::new(String::new()));

#[cfg(not(feature = "std"))]
impl OutputBuffer {
    fn with<R>(&self, f: impl FnOnce(&mut String) -> R) -> R {
        // SAFETY: WASM is single-threaded
        unsafe { f(&mut *self.0.get()) }
    }
}

/// Write to output (no newline).
#[cfg(feature = "std")]
#[inline]
pub fn write(s: &str) {
    print!("{}", s);
}

#[cfg(not(feature = "std"))]
#[inline]
pub fn write(s: &str) {
    OUTPUT_BUFFER.with(|buf| buf.push_str(s));
}

/// Write to output with newline.
#[cfg(feature = "std")]
#[inline]
pub fn writeln(s: &str) {
    println!("{}", s);
}

#[cfg(not(feature = "std"))]
#[inline]
pub fn writeln(s: &str) {
    OUTPUT_BUFFER.with(|buf| {
        buf.push_str(s);
        buf.push('\n');
    });
}

/// Take all captured output and clear the buffer.
/// In std mode, returns empty string (output went to stdout).
#[cfg(feature = "std")]
pub fn take_output() -> String {
    String::new()
}

#[cfg(not(feature = "std"))]
pub fn take_output() -> String {
    OUTPUT_BUFFER.with(|buf| core::mem::take(buf))
}

/// Clear the output buffer without returning contents.
#[cfg(feature = "std")]
pub fn clear_output() {}

#[cfg(not(feature = "std"))]
pub fn clear_output() {
    OUTPUT_BUFFER.with(|buf| buf.clear());
}
