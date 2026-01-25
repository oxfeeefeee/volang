//! Compilation functions for Vo source code.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use vo_common::vfs::{FileSet, FileSystem, RealFs, ZipFs};
use vo_analysis::analyze_project;
use vo_codegen::compile_project;
use vo_module::{PackageResolverMixed, StdSource, LocalSource, ModSource};
use vo_runtime::ext_loader::ExtensionManifest;
use vo_vm::bytecode::Module;
use vo_stdlib::EmbeddedStdlib;

#[derive(Debug)]
pub enum CompileError {
    Io(std::io::Error),
    Parse(String),
    Analysis(String),
    Codegen(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::Io(e) => write!(f, "IO error: {}", e),
            CompileError::Parse(e) => write!(f, "Parse error: {}", e),
            CompileError::Analysis(e) => write!(f, "Analysis error: {}", e),
            CompileError::Codegen(e) => write!(f, "Codegen error: {}", e),
        }
    }
}

impl std::error::Error for CompileError {}

impl From<std::io::Error> for CompileError {
    fn from(e: std::io::Error) -> Self {
        CompileError::Io(e)
    }
}

#[derive(Debug, Clone)]
pub struct CompileOutput {
    pub module: Module,
    pub source_root: PathBuf,
    pub extensions: Vec<ExtensionManifest>,
}

/// Compile a Vo source file, directory, zip archive, or bytecode file.
pub fn compile(path: &str) -> Result<CompileOutput, CompileError> {
    let p = Path::new(path);
    
    if path.ends_with(".voc") || path.ends_with(".vob") {
        load_bytecode(p)
    } else if let Some((zip_path, internal_root)) = parse_zip_path(path) {
        compile_zip(Path::new(&zip_path), internal_root.as_deref())
    } else {
        let root = source_root(p);
        let single_file = if p.is_file() {
            Some(p.file_name().unwrap_or_default())
        } else {
            None
        };
        compile_with_fs(RealFs::new(&root), &root, single_file)
    }
}

fn compile_zip(zip_path: &Path, internal_root: Option<&str>) -> Result<CompileOutput, CompileError> {
    let zip_fs = match internal_root {
        Some(root) => ZipFs::from_path_with_root(zip_path, root),
        None => ZipFs::from_path(zip_path),
    }?;
    
    let abs_root = zip_path.canonicalize().unwrap_or_else(|_| zip_path.to_path_buf());
    let file_set = FileSet::collect(&zip_fs, Path::new("."), abs_root.clone())?;
    
    if file_set.files.is_empty() {
        return Err(CompileError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no .vo files found in zip"
        )));
    }
    
    let resolver = create_resolver(&abs_root, zip_fs);
    
    let project = analyze_project(file_set, &resolver)
        .map_err(|e| CompileError::Analysis(format!("{}", e)))?;
    
    let module = compile_project(&project)
        .map_err(|e| CompileError::Codegen(format!("{:?}", e)))?;

    Ok(CompileOutput {
        module,
        source_root: abs_root,
        extensions: project.extensions,
    })
}

/// Compile with cache support.
/// Caches compiled bytecode in `.vo-cache` directory under the source root.
pub fn compile_with_cache(path: &str) -> Result<CompileOutput, CompileError> {
    let entry_path = Path::new(path);
    let root = source_root(entry_path);
    
    let cache_dir = root.join(".vo-cache");
    let cache_file = cache_dir.join("module.voc");
    let meta_file = cache_dir.join("mtime");
    let ext_file = cache_dir.join("extensions");
    
    let current_mtime = max_mtime(&root);
    
    // Try cache
    if let Some(output) = try_load_cache(&cache_file, &meta_file, &ext_file, &root, current_mtime) {
        return Ok(output);
    }
    
    // Compile
    let output = compile(path)?;
    
    // Save cache
    if let Some(mtime) = current_mtime {
        let _ = fs::create_dir_all(&cache_dir);
        let _ = fs::write(&cache_file, output.module.serialize());
        let _ = fs::write(&meta_file, mtime.to_string());
        save_extensions(&ext_file, &output.extensions);
    }
    
    Ok(output)
}

/// Compile a string of Vo code.
pub fn compile_string(code: &str) -> Result<CompileOutput, CompileError> {
    use std::io::Write;
    
    let temp_dir = std::env::temp_dir().join("vo_compile");
    fs::create_dir_all(&temp_dir)?;
    let temp_file = temp_dir.join("temp.vo");
    
    let mut file = fs::File::create(&temp_file)?;
    file.write_all(code.as_bytes())?;
    drop(file);
    
    let result = compile(temp_file.to_str().unwrap());
    let _ = fs::remove_file(&temp_file);
    result
}

fn source_root(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    } else {
        path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf()
    }
}

fn load_bytecode(path: &Path) -> Result<CompileOutput, CompileError> {
    let bytes = fs::read(path)?;
    let module = Module::deserialize(&bytes)
        .map_err(|e| CompileError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("{:?}", e)
        )))?;
    Ok(CompileOutput {
        module,
        source_root: path.parent().unwrap_or(Path::new(".")).to_path_buf(),
        extensions: Vec::new(),
    })
}

fn compile_with_fs<F: FileSystem>(fs: F, root: &Path, single_file: Option<&std::ffi::OsStr>) -> Result<CompileOutput, CompileError> {
    let file_set = if let Some(file_name) = single_file {
        FileSet::from_file(&fs, Path::new(file_name), root.to_path_buf())?
    } else {
        FileSet::collect(&fs, Path::new("."), root.to_path_buf())?
    };
    
    if file_set.files.is_empty() {
        return Err(CompileError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no .vo files found"
        )));
    }
    
    let resolver = create_resolver(&root, fs);
    
    let project = analyze_project(file_set, &resolver)
        .map_err(|e| CompileError::Analysis(format!("{}", e)))?;
    
    let module = compile_project(&project)
        .map_err(|e| CompileError::Codegen(format!("{:?}", e)))?;

    Ok(CompileOutput {
        module,
        source_root: root.to_path_buf(),
        extensions: project.extensions,
    })
}

fn create_resolver<F: FileSystem>(local_root: &Path, local_fs: F) -> PackageResolverMixed<EmbeddedStdlib, F, RealFs> {
    let mod_root = dirs::home_dir()
        .map(|h| h.join(".vo/mod"))
        .unwrap_or_else(|| local_root.join(".vo/mod"));
    
    PackageResolverMixed {
        std: StdSource::with_fs(EmbeddedStdlib::new()),
        local: LocalSource::with_fs(local_fs),
        r#mod: ModSource::with_fs(RealFs::new(mod_root)),
    }
}

fn parse_zip_path(path: &str) -> Option<(String, Option<String>)> {
    if path.ends_with(".zip") {
        Some((path.to_string(), None))
    } else if path.contains(".zip:") {
        let parts: Vec<&str> = path.splitn(2, ".zip:").collect();
        if parts.len() == 2 {
            Some((format!("{}.zip", parts[0]), Some(parts[1].to_string())))
        } else {
            None
        }
    } else {
        None
    }
}

fn max_mtime(dir: &Path) -> Option<u64> {
    let mut max = 0u64;
    for entry in walkdir(dir) {
        if entry.extension().map(|e| e == "vo").unwrap_or(false) {
            if let Ok(meta) = entry.metadata() {
                if let Ok(mtime) = meta.modified() {
                    if let Ok(dur) = mtime.duration_since(SystemTime::UNIX_EPOCH) {
                        max = max.max(dur.as_secs());
                    }
                }
            }
        }
    }
    if max == 0 { None } else { Some(max) }
}

fn walkdir(path: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                result.extend(walkdir(&p));
            } else {
                result.push(p);
            }
        }
    }
    result
}

fn try_load_cache(
    cache_file: &Path,
    meta_file: &Path,
    ext_file: &Path,
    source_root: &Path,
    current_mtime: Option<u64>,
) -> Option<CompileOutput> {
    let current = current_mtime?;
    let cached: u64 = fs::read_to_string(meta_file).ok()?.trim().parse().ok()?;
    if cached != current {
        return None;
    }
    
    let bytes = fs::read(cache_file).ok()?;
    let module = Module::deserialize(&bytes).ok()?;
    let extensions = load_extensions(ext_file);
    
    Some(CompileOutput {
        module,
        source_root: source_root.to_path_buf(),
        extensions,
    })
}

fn save_extensions(path: &Path, extensions: &[ExtensionManifest]) {
    use std::io::Write;
    if let Ok(mut f) = fs::File::create(path) {
        for ext in extensions {
            let _ = writeln!(f, "{}|{}", ext.name, ext.native_path.display());
        }
    }
}

fn load_extensions(path: &Path) -> Vec<ExtensionManifest> {
    let mut result = Vec::new();
    if let Ok(content) = fs::read_to_string(path) {
        for line in content.lines() {
            if let Some((name, lib_path)) = line.split_once('|') {
                result.push(ExtensionManifest {
                    name: name.to_string(),
                    native_path: PathBuf::from(lib_path),
                });
            }
        }
    }
    result
}
