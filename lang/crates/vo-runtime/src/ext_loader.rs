//! Extension loader for dynamic loading of native extensions.
//!
//! Extensions are discovered from `vo.ext.toml` manifests and loaded
//! at runtime via dlopen.

use std::collections::HashMap;
use std::path::Path;

use libloading::{Library, Symbol};

use crate::ffi::{ExternEntry, ExternEntryWithContext, ExternFn, ExternFnWithContext};

// Re-export from vo-module
pub use vo_module::{ExtensionManifest, discover_extensions};

/// ABI version - must match vo-ext's ABI_VERSION.
pub const ABI_VERSION: u32 = 1;

/// Extension table from loaded library.
#[repr(C)]
pub struct ExtensionTable {
    pub version: u32,
    pub entry_count: usize,
    pub entries: *const ExternEntry,
    pub entry_with_context_count: usize,
    pub entries_with_context: *const ExternEntryWithContext,
}

/// Error type for extension loading.
#[derive(Debug)]
pub enum ExtError {
    /// Failed to load library.
    LoadFailed(String),
    /// Missing entry point function.
    MissingEntryPoint,
    /// ABI version mismatch.
    VersionMismatch { expected: u32, found: u32 },
    /// Manifest parse error.
    ManifestError(String),
    /// IO error.
    Io(std::io::Error),
}

impl std::fmt::Display for ExtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtError::LoadFailed(msg) => write!(f, "failed to load extension: {}", msg),
            ExtError::MissingEntryPoint => write!(f, "extension missing vo_ext_get_entries"),
            ExtError::VersionMismatch { expected, found } => {
                write!(f, "ABI version mismatch: expected {}, found {}", expected, found)
            }
            ExtError::ManifestError(msg) => write!(f, "manifest error: {}", msg),
            ExtError::Io(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for ExtError {}

impl From<std::io::Error> for ExtError {
    fn from(e: std::io::Error) -> Self {
        ExtError::Io(e)
    }
}

/// A loaded extension.
struct LoadedExtension {
    /// Keep library alive.
    _lib: Library,
    /// Name of the extension.
    name: String,
    /// Basic extern functions.
    entries: &'static [ExternEntry],
    /// Context extern functions.
    entries_with_context: &'static [ExternEntryWithContext],
}

/// Extension loader and registry.
pub struct ExtensionLoader {
    /// Loaded extensions.
    loaded: Vec<LoadedExtension>,
    /// Cache: function name -> index in loaded + entry index.
    cache: HashMap<String, (usize, usize, bool)>, // (ext_idx, entry_idx, with_context)
}

impl ExtensionLoader {
    /// Create a new extension loader.
    pub fn new() -> Self {
        Self {
            loaded: Vec::new(),
            cache: HashMap::new(),
        }
    }

    /// Load an extension from a dynamic library path.
    pub fn load(&mut self, path: &Path, name: &str) -> Result<(), ExtError> {
        // Use RTLD_GLOBAL so symbols are visible to other extensions
        #[cfg(unix)]
        let lib = unsafe {
            let flags = libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_GLOBAL;
            libloading::os::unix::Library::open(Some(path), flags)
                .map(|l| Library::from(l))
                .map_err(|e| ExtError::LoadFailed(e.to_string()))?
        };
        #[cfg(not(unix))]
        let lib = unsafe {
            Library::new(path).map_err(|e| ExtError::LoadFailed(e.to_string()))?
        };

        let get_entries: Symbol<extern "C" fn() -> ExtensionTable> = unsafe {
            lib.get(b"vo_ext_get_entries")
                .map_err(|_| ExtError::MissingEntryPoint)?
        };

        let table = get_entries();

        if table.version != ABI_VERSION {
            return Err(ExtError::VersionMismatch {
                expected: ABI_VERSION,
                found: table.version,
            });
        }

        let entries: &'static [ExternEntry] = unsafe {
            std::slice::from_raw_parts(table.entries, table.entry_count)
        };

        let entries_with_context: &'static [ExternEntryWithContext] = unsafe {
            std::slice::from_raw_parts(table.entries_with_context, table.entry_with_context_count)
        };

        let ext_idx = self.loaded.len();

        // Build cache for basic entries
        for (i, entry) in entries.iter().enumerate() {
            self.cache.insert(entry.name.to_string(), (ext_idx, i, false));
        }

        // Build cache for context entries
        for (i, entry) in entries_with_context.iter().enumerate() {
            self.cache.insert(entry.name.to_string(), (ext_idx, i, true));
        }

        self.loaded.push(LoadedExtension {
            _lib: lib,
            name: name.to_string(),
            entries,
            entries_with_context,
        });

        Ok(())
    }

    /// Lookup a basic extern function by name.
    pub fn lookup(&self, name: &str) -> Option<ExternFn> {
        let (ext_idx, entry_idx, with_context) = self.cache.get(name)?;
        if *with_context {
            return None;
        }
        Some(self.loaded[*ext_idx].entries[*entry_idx].func)
    }

    /// Lookup a context extern function by name.
    pub fn lookup_with_context(&self, name: &str) -> Option<ExternFnWithContext> {
        let (ext_idx, entry_idx, with_context) = self.cache.get(name)?;
        if !*with_context {
            return None;
        }
        Some(self.loaded[*ext_idx].entries_with_context[*entry_idx].func)
    }

    /// Get list of loaded extension names.
    pub fn loaded_extensions(&self) -> Vec<&str> {
        self.loaded.iter().map(|e| e.name.as_str()).collect()
    }
}

impl Default for ExtensionLoader {
    fn default() -> Self {
        Self::new()
    }
}

