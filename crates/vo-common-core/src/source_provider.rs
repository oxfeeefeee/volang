//! Source provider trait for on-demand source code reading.
//!
//! This trait abstracts source code access for error display.
//! It is `no_std` compatible with a default empty implementation.

#[cfg(not(feature = "std"))]
use alloc::string::String;

/// Source provider - on-demand source reading for error display.
///
/// Implementations provide access to source code files when rendering
/// diagnostic messages. The default implementation returns `None`,
/// which causes error rendering to fall back to simple format.
pub trait SourceProvider {
    /// Read file content by path.
    ///
    /// Returns `None` if the file is unavailable, which causes
    /// error rendering to use simple `file:line:col` format.
    fn read_source(&self, path: &str) -> Option<String> {
        let _ = path;
        None
    }
}

/// Empty source provider - always returns None.
///
/// Use this when source code is not available (e.g., running from .vob).
pub struct NoSource;

impl SourceProvider for NoSource {}
