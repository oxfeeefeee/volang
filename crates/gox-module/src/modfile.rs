//! Parser for gox.mod files.
//!
//! Format:
//! ```text
//! module <module-path>
//!
//! require <module-path> <version>
//! require <module-path> <version>
//! ```

use std::fs;
use std::path::Path;

use crate::error::{ModuleError, ModuleResult};

/// A parsed gox.mod file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModFile {
    /// The module path (e.g., "github.com/myuser/myproject").
    pub module: String,

    /// Direct dependencies.
    pub requires: Vec<Require>,
}

/// A single require directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Require {
    /// The module path.
    pub module: String,

    /// The exact version (e.g., "v1.2.3").
    pub version: String,
}

impl ModFile {
    /// Parses a gox.mod file from the given path.
    pub fn parse_file<P: AsRef<Path>>(path: P) -> ModuleResult<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(ModuleError::ModFileNotFound(path.to_path_buf()));
        }

        let content = fs::read_to_string(path)
            .map_err(|e| ModuleError::IoError(path.to_path_buf(), e.to_string()))?;

        Self::parse(&content, path)
    }

    /// Parses gox.mod content from a string.
    pub fn parse(content: &str, file_path: &Path) -> ModuleResult<Self> {
        let mut module: Option<String> = None;
        let mut requires: Vec<Require> = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let line_num = line_num + 1; // 1-indexed
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with("//") {
                continue;
            }

            // Parse module declaration
            if line.starts_with("module ") {
                if module.is_some() {
                    return Err(ModuleError::DuplicateModuleDecl(file_path.to_path_buf()));
                }

                let module_path = line["module ".len()..].trim();
                if !is_valid_module_path(module_path) {
                    return Err(ModuleError::ParseError {
                        file: file_path.to_path_buf(),
                        line: line_num,
                        message: format!("invalid module path: {}", module_path),
                    });
                }

                module = Some(module_path.to_string());
                continue;
            }

            // Parse require directive
            if line.starts_with("require ") {
                let rest = line["require ".len()..].trim();
                let parts: Vec<&str> = rest.split_whitespace().collect();

                if parts.len() != 2 {
                    return Err(ModuleError::ParseError {
                        file: file_path.to_path_buf(),
                        line: line_num,
                        message: format!(
                            "invalid require syntax, expected: require <module> <version>, got: {}",
                            line
                        ),
                    });
                }

                let req_module = parts[0];
                let req_version = parts[1];

                if !is_valid_module_path(req_module) {
                    return Err(ModuleError::ParseError {
                        file: file_path.to_path_buf(),
                        line: line_num,
                        message: format!("invalid module path: {}", req_module),
                    });
                }

                if !is_valid_version(req_version) {
                    return Err(ModuleError::ParseError {
                        file: file_path.to_path_buf(),
                        line: line_num,
                        message: format!("invalid version: {}", req_version),
                    });
                }

                requires.push(Require {
                    module: req_module.to_string(),
                    version: req_version.to_string(),
                });
                continue;
            }

            // Unknown directive
            return Err(ModuleError::ParseError {
                file: file_path.to_path_buf(),
                line: line_num,
                message: format!("unknown directive: {}", line),
            });
        }

        let module = module.ok_or_else(|| ModuleError::MissingModuleDecl(file_path.to_path_buf()))?;

        Ok(ModFile { module, requires })
    }

    /// Creates a new empty ModFile with the given module path.
    pub fn new(module: String) -> Self {
        ModFile {
            module,
            requires: Vec::new(),
        }
    }

    /// Adds a require directive.
    pub fn add_require(&mut self, module: String, version: String) {
        // Check if already exists and update version
        for req in &mut self.requires {
            if req.module == module {
                req.version = version;
                return;
            }
        }
        self.requires.push(Require { module, version });
    }

    /// Serializes the ModFile to a string.
    pub fn to_string(&self) -> String {
        let mut result = format!("module {}\n", self.module);

        if !self.requires.is_empty() {
            result.push('\n');
            for req in &self.requires {
                result.push_str(&format!("require {} {}\n", req.module, req.version));
            }
        }

        result
    }

    /// Writes the ModFile to a file.
    pub fn write_file<P: AsRef<Path>>(&self, path: P) -> ModuleResult<()> {
        let path = path.as_ref();
        fs::write(path, self.to_string())
            .map_err(|e| ModuleError::IoError(path.to_path_buf(), e.to_string()))
    }
}

/// Validates a module path.
///
/// A valid module path:
/// - Is not empty
/// - Does not start or end with /
/// - Does not contain //
/// - Does not start with std/ (reserved for standard library)
fn is_valid_module_path(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    if path.starts_with('/') || path.ends_with('/') {
        return false;
    }
    if path.contains("//") {
        return false;
    }
    if path.starts_with("std/") || path == "std" {
        return false;
    }
    // Must contain at least one path component
    true
}

/// Validates a version string.
///
/// A valid version:
/// - Starts with 'v'
/// - Has format vMAJOR.MINOR.PATCH with optional pre-release/build metadata
fn is_valid_version(version: &str) -> bool {
    if !version.starts_with('v') {
        return false;
    }

    let rest = &version[1..];
    
    // Split off pre-release (-) and build metadata (+)
    let version_core = rest
        .split('-')
        .next()
        .unwrap_or("")
        .split('+')
        .next()
        .unwrap_or("");

    // Check MAJOR.MINOR.PATCH format
    let parts: Vec<&str> = version_core.split('.').collect();
    if parts.len() < 2 {
        return false;
    }

    // All parts must be numeric
    for part in &parts {
        if part.is_empty() || !part.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_simple() {
        let content = r#"
module github.com/myuser/myproject

require github.com/foo/bar v1.2.3
require github.com/baz/qux v0.1.0
"#;
        let mod_file = ModFile::parse(content, &PathBuf::from("gox.mod")).unwrap();
        
        assert_eq!(mod_file.module, "github.com/myuser/myproject");
        assert_eq!(mod_file.requires.len(), 2);
        assert_eq!(mod_file.requires[0].module, "github.com/foo/bar");
        assert_eq!(mod_file.requires[0].version, "v1.2.3");
        assert_eq!(mod_file.requires[1].module, "github.com/baz/qux");
        assert_eq!(mod_file.requires[1].version, "v0.1.0");
    }

    #[test]
    fn test_parse_with_comments() {
        let content = r#"
// This is a comment
module github.com/myuser/myproject

// Another comment
require github.com/foo/bar v1.2.3
"#;
        let mod_file = ModFile::parse(content, &PathBuf::from("gox.mod")).unwrap();
        
        assert_eq!(mod_file.module, "github.com/myuser/myproject");
        assert_eq!(mod_file.requires.len(), 1);
    }

    #[test]
    fn test_parse_no_requires() {
        let content = "module myproject\n";
        let mod_file = ModFile::parse(content, &PathBuf::from("gox.mod")).unwrap();
        
        assert_eq!(mod_file.module, "myproject");
        assert!(mod_file.requires.is_empty());
    }

    #[test]
    fn test_parse_missing_module() {
        let content = "require github.com/foo/bar v1.0.0\n";
        let result = ModFile::parse(content, &PathBuf::from("gox.mod"));
        
        assert!(matches!(result, Err(ModuleError::MissingModuleDecl(_))));
    }

    #[test]
    fn test_parse_duplicate_module() {
        let content = r#"
module github.com/a
module github.com/b
"#;
        let result = ModFile::parse(content, &PathBuf::from("gox.mod"));
        
        assert!(matches!(result, Err(ModuleError::DuplicateModuleDecl(_))));
    }

    #[test]
    fn test_parse_invalid_version() {
        let content = r#"
module myproject
require github.com/foo/bar 1.2.3
"#;
        let result = ModFile::parse(content, &PathBuf::from("gox.mod"));
        
        assert!(matches!(result, Err(ModuleError::ParseError { .. })));
    }

    #[test]
    fn test_parse_std_module_rejected() {
        let content = "module std/io\n";
        let result = ModFile::parse(content, &PathBuf::from("gox.mod"));
        
        assert!(matches!(result, Err(ModuleError::ParseError { .. })));
    }

    #[test]
    fn test_valid_versions() {
        assert!(is_valid_version("v1.0.0"));
        assert!(is_valid_version("v0.1.0"));
        assert!(is_valid_version("v2.0.0-beta.1"));
        assert!(is_valid_version("v1.2.3+meta"));
        assert!(is_valid_version("v1.2.3-rc.1+build.123"));
        assert!(is_valid_version("v1.0"));
        
        assert!(!is_valid_version("1.0.0"));
        assert!(!is_valid_version("v"));
        assert!(!is_valid_version(""));
        assert!(!is_valid_version("v.1.0"));
    }

    #[test]
    fn test_to_string() {
        let mod_file = ModFile {
            module: "github.com/myuser/myproject".to_string(),
            requires: vec![
                Require {
                    module: "github.com/foo/bar".to_string(),
                    version: "v1.2.3".to_string(),
                },
            ],
        };

        let expected = r#"module github.com/myuser/myproject

require github.com/foo/bar v1.2.3
"#;
        assert_eq!(mod_file.to_string(), expected);
    }
}
