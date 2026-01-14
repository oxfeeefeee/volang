//! Extension loader for dynamic loading of native extensions.
//!
//! Extensions are discovered from `vo.ext.toml` manifests and loaded
//! at runtime via dlopen.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

use crate::ffi::{ExternEntry, ExternEntryWithContext, ExternFn, ExternFnWithContext};

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

/// Discover extensions from a project root directory.
///
/// Looks for `vo.ext.toml` files in the root and returns paths to native libraries.
pub fn discover_extensions(project_root: &Path) -> Result<Vec<ExtensionManifest>, ExtError> {
    let manifest_path = project_root.join("vo.ext.toml");
    if !manifest_path.exists() {
        return Ok(Vec::new());
    }

    let manifest = parse_manifest(&manifest_path)?;
    Ok(vec![manifest])
}

/// Parsed extension manifest.
#[derive(Debug)]
pub struct ExtensionManifest {
    /// Extension name.
    pub name: String,
    /// Path to native library.
    pub native_path: PathBuf,
}

/// Parse a vo.ext.toml manifest file.
fn parse_manifest(path: &Path) -> Result<ExtensionManifest, ExtError> {
    let content = std::fs::read_to_string(path)?;
    
    // Simple TOML parsing (avoid adding toml dependency)
    let mut name = String::new();
    let mut native_path = String::new();

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("name") {
            if let Some(val) = extract_toml_string(line) {
                name = val;
            }
        } else if line.starts_with("path") {
            if let Some(val) = extract_toml_string(line) {
                native_path = val;
            }
        }
    }

    if name.is_empty() {
        return Err(ExtError::ManifestError("missing 'name' in [extension]".to_string()));
    }
    if native_path.is_empty() {
        return Err(ExtError::ManifestError("missing 'path' in [native]".to_string()));
    }

    let parent = path.parent().unwrap_or(Path::new("."));
    let full_path = parent.join(&native_path);

    Ok(ExtensionManifest {
        name,
        native_path: full_path,
    })
}

/// Extract a string value from a TOML line like `key = "value"`.
fn extract_toml_string(line: &str) -> Option<String> {
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() != 2 {
        return None;
    }
    let val = parts[1].trim();
    if val.starts_with('"') && val.ends_with('"') && val.len() >= 2 {
        Some(val[1..val.len()-1].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_toml_string() {
        assert_eq!(extract_toml_string(r#"name = "test""#), Some("test".to_string()));
        assert_eq!(extract_toml_string(r#"path = "native/lib.so""#), Some("native/lib.so".to_string()));
        assert_eq!(extract_toml_string("invalid"), None);
    }
}
