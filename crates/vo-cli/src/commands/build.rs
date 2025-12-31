//! `vo build` command - Build a Vo project.

use std::path::Path;
use vo_common::vfs::{FileSet, RealFs};
use vo_analysis::analyze_project;
use vo_module::{ModFile, VfsConfig};
use vo_codegen::compile_project;
use vo_vm::vm::Vm;
use super::run::StdMode;
use crate::output::TAG_OK;

/// Build and run a Vo project.
///
/// # Arguments
/// * `path` - Path to project directory (default: current directory)
/// * `std_mode` - Stdlib mode: "core" or "full"
///
/// # Output Tags (for script parsing)
/// * `[VO:OK]` - Execution completed successfully
/// * `[VO:PANIC:message]` - Program panicked
/// * `[VO:ERROR:message]` - Compilation/analysis error
///
/// # Examples
/// ```text
/// vo build
/// vo build ./myproject
/// vo build --std=core
/// ```
pub fn run(path: &str, std_mode: StdMode) -> Result<(), Box<dyn std::error::Error>> {
    let project_dir = Path::new(path).canonicalize()?;
    let mod_file_path = project_dir.join("vo.mod");

    // Check if vo.mod exists (optional for simple projects)
    if mod_file_path.exists() {
        let mod_file = ModFile::parse_file(&mod_file_path)?;
        println!("Building module: {}", mod_file.module);
    } else {
        println!("Building project in: {}", project_dir.display());
    }

    // Collect all .vo files
    let fs = RealFs;
    let file_set = FileSet::collect(&fs, &project_dir)?;
    
    if file_set.files.is_empty() {
        return Err("no .vo files found".into());
    }
    
    println!("Found {} source files", file_set.files.len());
    
    // Initialize VFS
    let vfs_config = VfsConfig::from_env(project_dir.clone());
    let vfs = vfs_config.to_vfs();
    
    // Analyze project
    let project = analyze_project(file_set, &vfs).map_err(|e| format!("analysis error: {}", e))?;
    println!("Analyzed {} packages", project.packages.len());
    
    // Compile to bytecode
    let module = compile_project(&project).map_err(|e| format!("codegen error: {}", e))?;
    
    // During development: run directly without writing file
    println!("Running module: {} (std={})", module.name, if std_mode == StdMode::Core { "core" } else { "full" });
    let mut vm = Vm::new();
    vm.load(module.clone());
    match vm.run() {
        Ok(()) => println!("{}", TAG_OK),
        Err(e) => return Err(format!("panic: {:?}", e).into()),
    }
    Ok(())
}
