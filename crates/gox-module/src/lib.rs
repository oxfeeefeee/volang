//! Module system for GoX.
//!
//! This crate implements the GoX module system as specified in `docs/gox-mod-spec.md`:
//!
//! - **gox.mod parsing**: Parse module declarations and dependencies
//! - **Dependency closure**: Compute transitive dependencies with version conflict detection
//! - **Import resolution**: Resolve import paths to filesystem locations
//!
//! # Example
//!
//! ```ignore
//! use gox_module::{ModFile, ModuleResolver};
//!
//! // Parse gox.mod
//! let mod_file = ModFile::parse_file("gox.mod")?;
//!
//! // Create resolver and compute closure
//! let resolver = ModuleResolver::new(project_root);
//! let closure = resolver.compute_closure(&mod_file)?;
//!
//! // Resolve an import path
//! let pkg_path = resolver.resolve_import("github.com/foo/bar/pkg", &closure)?;
//! ```

mod modfile;
mod resolver;
mod error;

pub use modfile::{ModFile, Require};
pub use resolver::{ModuleResolver, ModuleClosure, ResolvedPackage};
pub use error::{ModuleError, ModuleResult};
