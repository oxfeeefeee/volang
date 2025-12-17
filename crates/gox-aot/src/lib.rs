//! GoX AOT Compiler
//!
//! This crate provides AOT (Ahead-Of-Time) compilation for GoX bytecode
//! to native executables using Cranelift.
//!
//! ## Architecture
//!
//! ```text
//! VM Bytecode → Cranelift IR → Object File (.o) → Executable
//! ```
//!
//! ## Supported Targets
//!
//! - macOS (Mach-O, x86_64 and ARM64)
//! - Linux (ELF, x86_64 and ARM64) - planned
//! - Windows (PE, x86_64) - planned

/// Target platform for native compilation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// macOS x86_64
    MacosX64,
    /// macOS ARM64 (Apple Silicon)
    MacosArm64,
    /// Linux x86_64
    LinuxX64,
    /// Linux ARM64
    LinuxArm64,
}

impl Target {
    /// Detect the current host target
    pub fn host() -> Self {
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        return Target::MacosX64;
        
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        return Target::MacosArm64;
        
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        return Target::LinuxX64;
        
        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        return Target::LinuxArm64;
        
        #[cfg(not(any(
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "aarch64"),
        )))]
        panic!("Unsupported host platform")
    }
}

/// Native compiler context
pub struct NativeCompiler {
    target: Target,
}

impl NativeCompiler {
    /// Create a new native compiler for the given target
    pub fn new(target: Target) -> Self {
        Self { target }
    }

    /// Create a native compiler for the host platform
    pub fn for_host() -> Self {
        Self::new(Target::host())
    }

    /// Get the target platform
    pub fn target(&self) -> Target {
        self.target
    }
}

// TODO: Implement Cranelift-based AOT compilation
// - compile_module(bytecode: &Module) -> Vec<u8>  // Object file bytes
// - link(objects: &[Vec<u8>], runtime: &Path) -> Executable
// - Bytecode → Cranelift IR translation
// - Object file emission (ELF/Mach-O)
