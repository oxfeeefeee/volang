//! Integration tests with embedded Vo source code.
//!
//! These tests compile and run Vo code directly from Rust strings,
//! useful for testing compiler features without external files.

use std::path::PathBuf;
use vo_common::vfs::{FileSet, MemoryFs};
use vo_analysis::analyze_project;
use vo_codegen::compile_project;
use vo_module::PackageResolver;
use vo_vm::vm::Vm;

/// Compile and run embedded Vo source code.
/// Returns Ok(()) on success, Err(message) on failure.
fn run_vo(name: &str, source: &str) -> Result<(), String> {
    run_vo_multi(&[(name, source)])
}

/// Compile and run multiple Vo source files.
fn run_vo_multi(files: &[(&str, &str)]) -> Result<(), String> {
    // Create MemoryFs with the source files
    let mut fs = MemoryFs::new();
    for (name, content) in files {
        fs.add_file(*name, *content);
    }
    
    // Collect files into FileSet
    let file_set = FileSet::collect(&fs, std::path::Path::new("."), PathBuf::from("."))
        .map_err(|e| format!("Failed to collect files: {}", e))?;
    
    if file_set.files.is_empty() {
        return Err("No .vo files found".to_string());
    }
    
    // Create resolver with MemoryFs for imports
    let resolver = PackageResolver::with_fs(fs.clone());
    
    // Analyze
    let project = analyze_project(file_set, &resolver)
        .map_err(|e| format!("Analysis failed: {:?}", e))?;
    
    // Compile
    let module = compile_project(&project)
        .map_err(|e| format!("Codegen failed: {:?}", e))?;
    
    // Run
    let mut vm = Vm::new();
    vm.load(module);
    vm.run().map_err(|e| format!("Runtime error: {:?}", e))?;
    
    Ok(())
}

#[test]
fn test_hello_world() {
    let result = run_vo("main.vo", r#"
package main

func main() {
    println("Hello from embedded Vo!")
}
"#);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_arithmetic() {
    let result = run_vo("main.vo", r#"
package main

func main() {
    a := 10
    b := 20
    c := a + b
    assert(c == 30, "addition failed")
    
    d := b - a
    assert(d == 10, "subtraction failed")
    
    e := a * b
    assert(e == 200, "multiplication failed")
    
    f := b / a
    assert(f == 2, "division failed")
}
"#);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_if_else() {
    let result = run_vo("main.vo", r#"
package main

func main() {
    x := 10
    y := 0
    if x > 5 {
        y = 1
    } else {
        y = 2
    }
    assert(y == 1, "if branch")
}
"#);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_functions() {
    let result = run_vo("main.vo", r#"
package main

func add(a, b int) int {
    return a + b
}

func factorial(n int) int {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

func main() {
    assert(add(3, 4) == 7, "add")
    assert(factorial(5) == 120, "factorial")
}
"#);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_structs() {
    let result = run_vo("main.vo", r#"
package main

type Point struct {
    x, y int
}

func (p Point) Sum() int {
    return p.x + p.y
}

func main() {
    p := Point{x: 10, y: 20}
    assert(p.x == 10, "field x")
    assert(p.y == 20, "field y")
    assert(p.Sum() == 30, "method Sum")
}
"#);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_arrays_slices() {
    let result = run_vo("main.vo", r#"
package main

func main() {
    // Array
    arr := [3]int{1, 2, 3}
    assert(arr[0] == 1, "arr[0]")
    assert(arr[1] == 2, "arr[1]")
    assert(arr[2] == 3, "arr[2]")
    assert(len(arr) == 3, "len(arr)")
    
    // Slice
    s := []int{10, 20, 30}
    assert(s[0] == 10, "s[0]")
    assert(len(s) == 3, "len(s)")
    
    // Append
    s = append(s, 40)
    assert(len(s) == 4, "len after append")
    assert(s[3] == 40, "s[3]")
}
"#);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_maps() {
    let result = run_vo("main.vo", r#"
package main

func main() {
    m := map[string]int{
        "one": 1,
        "two": 2,
    }
    
    assert(m["one"] == 1, "m[one]")
    assert(m["two"] == 2, "m[two]")
    
    m["three"] = 3
    assert(m["three"] == 3, "m[three]")
    assert(len(m) == 3, "len(m)")
}
"#);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_closures() {
    let result = run_vo("main.vo", r#"
package main

func main() {
    // Simple closure
    add := func(a, b int) int {
        return a + b
    }
    assert(add(3, 4) == 7, "closure add")
    
    // Closure capturing variable
    x := 10
    addX := func(n int) int {
        return n + x
    }
    assert(addX(5) == 15, "closure capture")
}
"#);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_multi_file_import() {
    let result = run_vo_multi(&[
        ("main.vo", r#"
package main

import "./math"

func main() {
    assert(math.Add(10, 20) == 30, "import Add")
    assert(math.Mul(3, 4) == 12, "import Mul")
}
"#),
        ("math/math.vo", r#"
package math

func Add(a, b int) int {
    return a + b
}

func Mul(a, b int) int {
    return a * b
}
"#),
    ]);
    assert!(result.is_ok(), "Failed: {:?}", result);
}
