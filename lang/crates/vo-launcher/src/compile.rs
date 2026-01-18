//! Compilation functions for Vo source code.

use std::path::{Path, PathBuf};
use vo_common::vfs::{FileSet, RealFs, ZipFs};
use vo_analysis::analyze_project;
use vo_codegen::compile_project;
use vo_module::{PackageResolverMixed, StdSource, LocalSource, ModSource};
use vo_runtime::ext_loader::ExtensionManifest;
use vo_vm::bytecode::Module;

use crate::stdlib::{EmbeddedStdlib, create_resolver};

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

pub fn compile_file(path: &str) -> Result<CompileOutput, CompileError> {
    compile_source_file(Path::new(path))
}

pub fn compile_string(code: &str) -> Result<CompileOutput, CompileError> {
    use std::io::Write;
    
    let temp_dir = std::env::temp_dir().join("vo_compile");
    std::fs::create_dir_all(&temp_dir)?;
    let temp_file = temp_dir.join("temp.vo");
    
    let mut file = std::fs::File::create(&temp_file)?;
    file.write_all(code.as_bytes())?;
    drop(file);
    
    let result = compile_file(temp_file.to_str().unwrap());
    let _ = std::fs::remove_file(&temp_file);
    result
}

pub fn compile_source(file: &str) -> Result<CompileOutput, CompileError> {
    let path = Path::new(file);
    
    if file.ends_with(".vo") {
        compile_source_file(path)
    } else if let Some((zip_path, internal_root)) = parse_zip_path(file) {
        compile_zip(Path::new(&zip_path), internal_root.as_deref())
    } else if file.ends_with(".voc") || file.ends_with(".vob") {
        let source_root = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let bytes = std::fs::read(file)?;
        let module = Module::deserialize(&bytes)
            .map_err(|e| CompileError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("{:?}", e)
            )))?;
        Ok(CompileOutput { module, source_root, extensions: Vec::new() })
    } else if path.is_dir() {
        compile_directory(path)
    } else {
        compile_source_file(path)
    }
}

fn parse_zip_path(file: &str) -> Option<(String, Option<String>)> {
    if file.ends_with(".zip") {
        Some((file.to_string(), None))
    } else if file.contains(".zip:") {
        let parts: Vec<&str> = file.splitn(2, ".zip:").collect();
        if parts.len() == 2 {
            Some((format!("{}.zip", parts[0]), Some(parts[1].to_string())))
        } else {
            None
        }
    } else {
        None
    }
}

fn compile_source_file(path: &Path) -> Result<CompileOutput, CompileError> {
    let (abs_root, rel_path) = if path.is_file() {
        let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let root = abs_path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let file_name = abs_path.file_name().unwrap_or_default();
        (root, PathBuf::from(file_name))
    } else {
        let root = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        (root, PathBuf::from("."))
    };
    
    let fs = RealFs::new(&abs_root);
    
    let file_set = if path.is_file() {
        FileSet::from_file(&fs, &rel_path, abs_root.clone())?
    } else {
        FileSet::collect(&fs, Path::new("."), abs_root.clone())?
    };
    
    if file_set.files.is_empty() {
        return Err(CompileError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no .vo files found"
        )));
    }
    
    let source_root = file_set.root.clone();
    let resolver = create_resolver(&abs_root);
    
    let project = analyze_project(file_set, &resolver)
        .map_err(|e| CompileError::Analysis(format!("{:?}", e)))?;
    
    let module = compile_project(&project)
        .map_err(|e| CompileError::Codegen(format!("{:?}", e)))?;

    Ok(CompileOutput { module, source_root, extensions: project.extensions })
}

fn compile_directory(path: &Path) -> Result<CompileOutput, CompileError> {
    let abs_root = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let fs = RealFs::new(&abs_root);
    
    let file_set = FileSet::collect(&fs, Path::new("."), abs_root.clone())?;
    
    if file_set.files.is_empty() {
        return Err(CompileError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no .vo files found"
        )));
    }
    
    let source_root = file_set.root.clone();
    let resolver = create_resolver(&abs_root);
    
    let project = analyze_project(file_set, &resolver)
        .map_err(|e| CompileError::Analysis(format!("{:?}", e)))?;
    
    let module = compile_project(&project)
        .map_err(|e| CompileError::Codegen(format!("{:?}", e)))?;

    Ok(CompileOutput { module, source_root, extensions: project.extensions })
}

fn compile_zip(zip_path: &Path, internal_root: Option<&str>) -> Result<CompileOutput, CompileError> {
    let zip_fs = match internal_root {
        Some(root) => ZipFs::from_path_with_root(zip_path, root),
        None => ZipFs::from_path(zip_path),
    }.map_err(|e| CompileError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        e.to_string()
    )))?;
    
    let abs_root = zip_path.canonicalize().unwrap_or_else(|_| zip_path.to_path_buf());
    let file_set = FileSet::collect(&zip_fs, Path::new("."), abs_root.clone())?;
    
    if file_set.files.is_empty() {
        return Err(CompileError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no .vo files found in zip"
        )));
    }
    
    let mod_root = dirs::home_dir()
        .map(|h| h.join(".vo/mod"))
        .unwrap_or_else(|| abs_root.join(".vo/mod"));
    
    let resolver = PackageResolverMixed {
        std: StdSource::with_fs(EmbeddedStdlib::new()),
        local: LocalSource::with_fs(zip_fs),
        r#mod: ModSource::with_fs(RealFs::new(mod_root)),
    };
    
    let project = analyze_project(file_set, &resolver)
        .map_err(|e| CompileError::Analysis(format!("{:?}", e)))?;
    
    let module = compile_project(&project)
        .map_err(|e| CompileError::Codegen(format!("{:?}", e)))?;

    Ok(CompileOutput { module, source_root: abs_root, extensions: project.extensions })
}
