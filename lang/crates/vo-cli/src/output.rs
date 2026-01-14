//! Output tags for script parsing.
//!
//! These tags are used by test scripts to determine execution results.
//! Each tag includes the error source for precise error categorization.

pub use vo_common_core::SourceLoc;
use vo_common::diagnostics::DiagnosticEmitter;
use vo_analysis::AnalysisError;

/// Success tag - execution completed without errors.
pub const TAG_OK: &str = "[VO:OK]";

/// Error source/kind for categorizing errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Parse error
    Parse,
    /// Type check error
    Check,
    /// Code generation error
    Codegen,
    /// Runtime panic
    Panic,
    /// IO error (file not found, etc.)
    Io,
}

impl ErrorKind {
    fn tag_prefix(self) -> &'static str {
        match self {
            ErrorKind::Parse => "[VO:PARSE:",
            ErrorKind::Check => "[VO:CHECK:",
            ErrorKind::Codegen => "[VO:CODEGEN:",
            ErrorKind::Panic => "[VO:PANIC:",
            ErrorKind::Io => "[VO:IO:",
        }
    }
}

/// Format a message with optional location: "file:line:col: message"
fn format_with_loc(loc: Option<&SourceLoc>, msg: &str) -> String {
    match loc {
        Some(l) => format!("{}: {}", l, msg),
        None => msg.to_string(),
    }
}

/// Format an error tag with source kind: "[VO:KIND:file:line:col: message]"
pub fn format_tag(kind: ErrorKind, loc: Option<&SourceLoc>, msg: &str) -> String {
    format!("{}{}]", kind.tag_prefix(), format_with_loc(loc, msg))
}

/// Report an analysis error (parse/check) with pretty print and tag.
pub fn report_analysis_error(e: &AnalysisError) {
    if let Some(diags) = e.diagnostics() {
        if let Some(source_map) = e.source_map() {
            let emitter = DiagnosticEmitter::new(source_map);
            eprintln!();
            emitter.emit_all(diags);
        }
        let (kind, error_type) = match e {
            AnalysisError::Parse(_, _) => (ErrorKind::Parse, "parse"),
            AnalysisError::Check(_, _) => (ErrorKind::Check, "type check"),
            _ => (ErrorKind::Io, "analysis"),
        };
        println!("{}", format_tag(kind, None, &format!("{} failed: {} error(s)", error_type, diags.error_count())));
    } else {
        println!("{}", format_tag(ErrorKind::Io, None, &e.to_string()));
    }
}
