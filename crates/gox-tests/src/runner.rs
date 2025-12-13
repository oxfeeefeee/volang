//! Test runner for file-based tests.

use std::fs;
use std::path::Path;

use gox_common::source::FileId;
use gox_syntax::parser;

use crate::printer::AstPrinter;

/// Result of running a single test.
#[derive(Debug)]
pub struct TestResult {
    pub file_name: String,
    pub passed: bool,
    pub message: String,
    pub expected: String,
    pub actual: String,
}

/// Test runner for file-based tests.
pub struct TestRunner {
    test_dir: std::path::PathBuf,
}

impl TestRunner {
    pub fn new(test_dir: impl AsRef<Path>) -> Self {
        Self {
            test_dir: test_dir.as_ref().to_path_buf(),
        }
    }

    pub fn run_all(&self) -> Vec<TestResult> {
        let mut results = Vec::new();
        
        if let Ok(entries) = fs::read_dir(&self.test_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "gox") {
                    results.push(run_test_file(&path));
                }
            }
        }
        
        results
    }
}

/// Runs all tests in a directory.
pub fn run_all_tests(test_dir: &Path) -> Vec<TestResult> {
    TestRunner::new(test_dir).run_all()
}

/// Runs a single test file.
pub fn run_test_file(path: &Path) -> TestResult {
    let file_name = path.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            return TestResult {
                file_name,
                passed: false,
                message: format!("Failed to read file: {}", e),
                expected: String::new(),
                actual: String::new(),
            };
        }
    };
    
    let sections = parse_test_file(&content);
    
    // Run parser and compare output
    if let Some(expected_parser) = sections.parser {
        let (file, diagnostics, interner) = parser::parse(FileId::new(0), &sections.source);
        
        // Check for expected errors
        if let Some(expected_errors) = &sections.errors {
            let actual_errors = format_diagnostics(&diagnostics);
            if !errors_match(&actual_errors, expected_errors) {
                return TestResult {
                    file_name,
                    passed: false,
                    message: "Error messages don't match".to_string(),
                    expected: expected_errors.clone(),
                    actual: actual_errors,
                };
            }
        }
        
        // Compare AST output
        let mut printer = AstPrinter::new(&interner);
        let actual_ast = printer.print_file(&file);
        
        if !ast_matches(&actual_ast, &expected_parser) {
            return TestResult {
                file_name,
                passed: false,
                message: "Parser output doesn't match".to_string(),
                expected: expected_parser,
                actual: actual_ast,
            };
        }
    }
    
    // Check for unexpected errors
    if sections.errors.is_none() {
        let (_, diagnostics, _) = parser::parse(FileId::new(0), &sections.source);
        if diagnostics.has_errors() {
            let actual_errors = format_diagnostics(&diagnostics);
            return TestResult {
                file_name,
                passed: false,
                message: "Unexpected parse errors".to_string(),
                expected: String::new(),
                actual: actual_errors,
            };
        }
    }
    
    TestResult {
        file_name,
        passed: true,
        message: "OK".to_string(),
        expected: String::new(),
        actual: String::new(),
    }
}

/// Parsed sections from a test file.
struct TestSections {
    source: String,
    parser: Option<String>,
    errors: Option<String>,
}

/// Parses a test file into sections.
fn parse_test_file(content: &str) -> TestSections {
    let mut source = String::new();
    let mut parser = None;
    let mut errors = None;
    
    let mut current_section = "source";
    let mut current_content = String::new();
    
    for line in content.lines() {
        let trimmed = line.trim();
        
        if trimmed.starts_with("=== ") && trimmed.ends_with(" ===") {
            // Save previous section
            match current_section {
                "source" => source = current_content.trim().to_string(),
                "parser" => parser = Some(current_content.trim().to_string()),
                "errors" => errors = Some(current_content.trim().to_string()),
                _ => {}
            }
            current_content.clear();
            
            // Parse new section name
            let section_name = trimmed.trim_start_matches("=== ").trim_end_matches(" ===");
            current_section = section_name;
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }
    
    // Save last section
    match current_section {
        "source" => source = current_content.trim().to_string(),
        "parser" => parser = Some(current_content.trim().to_string()),
        "errors" => errors = Some(current_content.trim().to_string()),
        _ => {}
    }
    
    TestSections { source, parser, errors }
}

/// Formats diagnostics for comparison.
fn format_diagnostics(diagnostics: &gox_common::diagnostics::DiagnosticSink) -> String {
    let mut output = String::new();
    for diag in diagnostics.iter() {
        output.push_str(&diag.message);
        output.push('\n');
    }
    output.trim().to_string()
}

/// Checks if actual errors match expected errors.
fn errors_match(actual: &str, expected: &str) -> bool {
    let actual_lines: Vec<&str> = actual.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    let expected_lines: Vec<&str> = expected.lines().map(|l| l.trim()).filter(|l| !l.is_empty() && !l.starts_with("//")).collect();
    
    if actual_lines.len() != expected_lines.len() {
        return false;
    }
    
    for (a, e) in actual_lines.iter().zip(expected_lines.iter()) {
        if !a.contains(e) && !e.contains(a) {
            return false;
        }
    }
    
    true
}

/// Checks if actual AST matches expected AST.
fn ast_matches(actual: &str, expected: &str) -> bool {
    let normalize = |s: &str| -> String {
        s.lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    
    normalize(actual) == normalize(expected)
}
