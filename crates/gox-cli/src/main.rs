//! GoX compiler CLI.
//!
//! Commands:
//! - `gox init <module-path>` - Initialize a new module
//! - `gox get <module>@<version>` - Download a dependency
//! - `gox build` - Build the current module
//! - `gox check` - Type-check without building

use std::env;
use std::process;

use clap::{Parser, Subcommand};
use gox_module::{ModFile, ModuleResolver};

#[derive(Parser)]
#[command(name = "gox")]
#[command(about = "GoX compiler", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new GoX module
    Init {
        /// Module path (e.g., github.com/user/project)
        module_path: String,
    },

    /// Download a dependency and add it to gox.mod
    Get {
        /// Module and version (e.g., github.com/foo/bar@v1.2.3)
        module_version: String,
    },

    /// Build the current module
    Build,

    /// Type-check the current module without building
    Check,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { module_path } => cmd_init(&module_path),
        Commands::Get { module_version } => cmd_get(&module_version),
        Commands::Build => cmd_build(),
        Commands::Check => cmd_check(),
    };

    if let Err(e) = result {
        eprintln!("error: {}", e);
        process::exit(1);
    }
}

/// Initialize a new GoX module.
fn cmd_init(module_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let mod_file_path = cwd.join("gox.mod");

    if mod_file_path.exists() {
        return Err(format!("gox.mod already exists in {}", cwd.display()).into());
    }

    // Validate module path
    if module_path.is_empty() {
        return Err("module path cannot be empty".into());
    }
    if module_path.starts_with("std/") || module_path == "std" {
        return Err("module path cannot start with 'std/' (reserved for standard library)".into());
    }

    let mod_file = ModFile::new(module_path.to_string());
    mod_file.write_file(&mod_file_path)?;

    println!("Initialized module {} in {}", module_path, cwd.display());
    Ok(())
}

/// Download a dependency and add it to gox.mod.
fn cmd_get(module_version: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Parse module@version
    let parts: Vec<&str> = module_version.splitn(2, '@').collect();
    if parts.len() != 2 {
        return Err(format!(
            "invalid format: expected <module>@<version>, got: {}",
            module_version
        ).into());
    }

    let module = parts[0];
    let version = parts[1];

    if !version.starts_with('v') {
        return Err(format!("version must start with 'v', got: {}", version).into());
    }

    let cwd = env::current_dir()?;
    let mod_file_path = cwd.join("gox.mod");

    // Load existing gox.mod
    let mut mod_file = ModFile::parse_file(&mod_file_path)?;

    // Create .goxdeps directory if needed
    let deps_dir = cwd.join(".goxdeps");
    std::fs::create_dir_all(&deps_dir)?;

    // TODO: Actually download the module from a registry
    // For now, just create a placeholder directory structure
    let module_dir = deps_dir.join(format!("{}@{}", module, version));
    if !module_dir.exists() {
        std::fs::create_dir_all(&module_dir)?;
        
        // Create a minimal gox.mod for the dependency
        let dep_mod = ModFile::new(module.to_string());
        dep_mod.write_file(module_dir.join("gox.mod"))?;
        
        println!("Created placeholder for {}@{}", module, version);
        println!("  (actual download not implemented yet)");
    }

    // Add to gox.mod
    mod_file.add_require(module.to_string(), version.to_string());
    mod_file.write_file(&mod_file_path)?;

    println!("Added {}@{} to gox.mod", module, version);
    Ok(())
}

/// Build the current module.
fn cmd_build() -> Result<(), Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let mod_file_path = cwd.join("gox.mod");

    // Load gox.mod
    let mod_file = ModFile::parse_file(&mod_file_path)?;
    println!("Building module: {}", mod_file.module);

    // Create resolver and compute closure
    let resolver = ModuleResolver::new(&cwd);
    let closure = resolver.compute_closure(&mod_file)?;

    println!("Dependency closure:");
    if closure.modules.is_empty() {
        println!("  (no dependencies)");
    } else {
        for (path, resolved) in &closure.modules {
            println!("  {}@{}", path, resolved.version);
        }
    }

    // TODO: Actually compile the module
    println!("\nBuild not implemented yet");
    Ok(())
}

/// Type-check the current module without building.
fn cmd_check() -> Result<(), Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let mod_file_path = cwd.join("gox.mod");

    // Load gox.mod
    let mod_file = ModFile::parse_file(&mod_file_path)?;
    println!("Checking module: {}", mod_file.module);

    // Create resolver and compute closure
    let resolver = ModuleResolver::new(&cwd);
    let _closure = resolver.compute_closure(&mod_file)?;

    // TODO: Find all .gox files and type-check them
    println!("\nType checking not implemented yet");
    Ok(())
}
