//! Package resolution system.
//!
//! Provides three package sources:
//! - StdSource: Standard library packages
//! - LocalSource: Local packages (relative paths)
//! - ModSource: External module dependencies

use std::path::{Path, PathBuf};

use vo_common::vfs::{FileSystem, RealFs};

/// Package resolver configuration with three root paths.
#[derive(Debug, Clone)]
pub struct ResolverConfig {
    /// Standard library root path
    pub std_root: PathBuf,
    /// Local packages root path (project directory)
    pub local_root: PathBuf,
    /// Module cache root path
    pub mod_root: PathBuf,
}

impl ResolverConfig {
    /// Create a new resolver config with the given paths.
    pub fn new(std_root: PathBuf, local_root: PathBuf, mod_root: PathBuf) -> Self {
        Self { std_root, local_root, mod_root }
    }
    
    /// Create a config using environment variables and common defaults.
    pub fn from_env(project_dir: PathBuf) -> Self {
        let std_root = std::env::var("VO_STD")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                // Try common locations for stdlib
                // We check for "fmt/" subdir to verify it's a real stdlib, not just a coincidentally named dir
                let is_valid_stdlib = |p: &Path| p.is_dir() && p.join("fmt").is_dir();
                
                // 1. Project dir's "stdlib" folder
                let in_project = project_dir.join("stdlib");
                if is_valid_stdlib(&in_project) {
                    return in_project;
                }
                // 2. Parent dir's "stdlib" (for examples/)
                if let Some(parent) = project_dir.parent() {
                    let in_parent = parent.join("stdlib");
                    if is_valid_stdlib(&in_parent) {
                        return in_parent;
                    }
                    // 3. Grandparent (for examples/subdir/)
                    if let Some(grandparent) = parent.parent() {
                        let in_grandparent = grandparent.join("stdlib");
                        if is_valid_stdlib(&in_grandparent) {
                            return in_grandparent;
                        }
                        // 4. Great-grandparent (for test_data/stdlib/)
                        if let Some(great_grandparent) = grandparent.parent() {
                            let in_great = great_grandparent.join("stdlib");
                            if is_valid_stdlib(&in_great) {
                                return in_great;
                            }
                        }
                    }
                }
                // 5. Relative to executable
                if let Ok(exe) = std::env::current_exe() {
                    if let Some(exe_dir) = exe.parent() {
                        // Check ../stdlib (development) and ../lib/vo/stdlib (installed)
                        for rel in &["../../../stdlib", "../../stdlib", "../stdlib", "../lib/vo/stdlib"] {
                            let p = exe_dir.join(rel);
                            if is_valid_stdlib(&p) {
                                return p;
                            }
                        }
                    }
                }
                // Fallback
                project_dir.join("stdlib")
            });
        
        let mod_root = dirs::home_dir()
            .map(|h| h.join(".vo/mod"))
            .unwrap_or_else(|| project_dir.join(".vo/mod"));
        
        Self {
            std_root,
            local_root: project_dir,
            mod_root,
        }
    }
    
    /// Build a PackageResolver from this config.
    pub fn to_resolver(&self) -> PackageResolver {
        PackageResolver::with_roots(
            self.std_root.clone(),
            self.local_root.clone(),
            self.mod_root.clone(),
        )
    }
}

/// A resolved package from the package resolver.
#[derive(Debug, Clone)]
pub struct VfsPackage {
    /// Package name (e.g., "fmt", "mylib")
    pub name: String,
    /// Package path (e.g., "fmt", "./mylib", "github.com/user/pkg")
    pub path: String,
    /// Source files in the package
    pub files: Vec<VfsFile>,
}

/// A source file from a package.
#[derive(Debug, Clone)]
pub struct VfsFile {
    /// File path relative to package
    pub path: PathBuf,
    /// File content
    pub content: String,
}

/// Package source trait for package resolution.
pub trait PackageSource: Send + Sync {
    /// Resolve a package by import path.
    fn resolve(&self, import_path: &str) -> Option<VfsPackage>;
    
    /// Check if this source can handle the given import path.
    fn can_handle(&self, import_path: &str) -> bool;
}

/// Standard library package source.
pub struct StdSource<F: FileSystem = RealFs> {
    fs: F,
}

impl StdSource<RealFs> {
    pub fn new(root: PathBuf) -> Self {
        Self { fs: RealFs::new(&root) }
    }
}

impl<F: FileSystem> StdSource<F> {
    pub fn with_fs(fs: F) -> Self {
        Self { fs }
    }
    
    pub fn resolve(&self, import_path: &str) -> Option<VfsPackage> {
        resolve_package(&self.fs, import_path, import_path)
    }
    
    pub fn can_handle(&self, import_path: &str) -> bool {
        !import_path.contains('.')
    }
}

/// Local package source (relative paths).
pub struct LocalSource<F: FileSystem = RealFs> {
    fs: F,
}

impl LocalSource<RealFs> {
    pub fn new(root: PathBuf) -> Self {
        Self { fs: RealFs::new(&root) }
    }
}

impl<F: FileSystem> LocalSource<F> {
    pub fn with_fs(fs: F) -> Self {
        Self { fs }
    }
    
    pub fn resolve(&self, import_path: &str) -> Option<VfsPackage> {
        let rel_path = import_path.trim_start_matches("./");
        resolve_package(&self.fs, rel_path, import_path)
    }
    
    pub fn can_handle(&self, import_path: &str) -> bool {
        import_path.starts_with("./") || import_path.starts_with("../")
    }
}

/// External module package source (module cache).
pub struct ModSource<F: FileSystem = RealFs> {
    fs: F,
}

impl ModSource<RealFs> {
    pub fn new(root: PathBuf) -> Self {
        Self { fs: RealFs::new(&root) }
    }
}

impl<F: FileSystem> ModSource<F> {
    pub fn with_fs(fs: F) -> Self {
        Self { fs }
    }
    
    pub fn resolve(&self, import_path: &str) -> Option<VfsPackage> {
        resolve_package(&self.fs, import_path, import_path)
    }
    
    pub fn can_handle(&self, import_path: &str) -> bool {
        !import_path.starts_with("./") 
            && !import_path.starts_with("../")
            && import_path.contains('.')
    }
}

/// Combined package resolver that delegates to the appropriate source.
pub struct PackageResolver<F: FileSystem = RealFs> {
    pub std: StdSource<F>,
    pub local: LocalSource<F>,
    pub r#mod: ModSource<F>,
}

impl PackageResolver<RealFs> {
    /// Create a resolver with three filesystem root paths using real filesystem.
    ///
    /// # Arguments
    /// * `std_root` - Root path for standard library (e.g., "/usr/local/vo/std")
    /// * `local_root` - Root path for local packages (usually project directory)
    /// * `mod_root` - Root path for module cache (e.g., "~/.vo/mod")
    pub fn with_roots(std_root: PathBuf, local_root: PathBuf, mod_root: PathBuf) -> Self {
        Self {
            std: StdSource::new(std_root),
            local: LocalSource::new(local_root),
            r#mod: ModSource::new(mod_root),
        }
    }
}

impl<F: FileSystem + Clone> PackageResolver<F> {
    /// Create a resolver with a single shared filesystem (e.g., MemoryFs, ZipFs).
    /// All three sources share the same filesystem instance.
    pub fn with_fs(fs: F) -> Self {
        Self {
            std: StdSource::with_fs(fs.clone()),
            local: LocalSource::with_fs(fs.clone()),
            r#mod: ModSource::with_fs(fs),
        }
    }
}

impl<F: FileSystem> PackageResolver<F> {
    /// Resolve a package by import path.
    /// 
    /// Resolution order follows module.md spec:
    /// 1. Explicit local paths (./xxx or ../xxx) → LocalSource
    /// 2. External dependencies (has dots like github.com/...) → ModSource  
    /// 3. Non-dotted paths: try stdlib first, then local fallback
    pub fn resolve(&self, import_path: &str) -> Option<VfsPackage> {
        // Explicit local paths
        if import_path.starts_with("./") || import_path.starts_with("../") {
            return self.local.resolve(import_path);
        }
        
        // External dependencies (has dots = domain name)
        if import_path.contains('.') {
            return self.r#mod.resolve(import_path);
        }
        
        // Non-dotted paths: try stdlib first, then local fallback
        // This matches module.md spec section 6.3:
        //   "P" (known stdlib) → stdlib
        //   "P" (not stdlib) → <project-root>/<P>/
        if let Some(pkg) = self.std.resolve(import_path) {
            return Some(pkg);
        }
        
        // Fallback to local package (e.g., "iface" → ./iface/)
        self.local.resolve(import_path)
    }
}

/// Helper to resolve a package from a file system.
fn resolve_package<F: FileSystem>(fs: &F, fs_path: &str, import_path: &str) -> Option<VfsPackage> {
    let pkg_path = Path::new(fs_path);
    if !fs.is_dir(pkg_path) {
        return None;
    }
    
    let files = load_vo_files(fs, pkg_path)?;
    let name = fs_path.rsplit('/').next().unwrap_or(fs_path).to_string();
    
    Some(VfsPackage {
        name,
        path: import_path.to_string(),
        files,
    })
}

/// Helper to load all .vo files from a directory.
fn load_vo_files<F: FileSystem>(fs: &F, dir: &Path) -> Option<Vec<VfsFile>> {
    let mut files = Vec::new();
    
    let entries = fs.read_dir(dir).ok()?;
    for path in entries {
        if path.extension().is_some_and(|e| e == "vo") {
            if let Ok(content) = fs.read_file(&path) {
                files.push(VfsFile {
                    path: path.file_name().unwrap().into(),
                    content,
                });
            }
        }
    }
    
    if files.is_empty() {
        None
    } else {
        Some(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_can_handle() {
        let std_vfs = StdSource::new(PathBuf::new());
        let local_vfs = LocalSource::new(PathBuf::new());
        let mod_vfs = ModSource::new(PathBuf::new());
        
        // Stdlib
        assert!(std_vfs.can_handle("fmt"));
        assert!(std_vfs.can_handle("encoding/json"));
        assert!(!std_vfs.can_handle("github.com/user/pkg"));
        
        // Local
        assert!(local_vfs.can_handle("./mylib"));
        assert!(local_vfs.can_handle("../shared"));
        assert!(!local_vfs.can_handle("fmt"));
        
        // Mod
        assert!(mod_vfs.can_handle("github.com/user/pkg"));
        assert!(!mod_vfs.can_handle("fmt"));
        assert!(!mod_vfs.can_handle("./mylib"));
    }
}
