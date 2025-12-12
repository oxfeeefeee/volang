//! Parser for GoX source code.

mod decl;
mod expr;
mod stmt;

use crate::ast::*;
use crate::lexer::Lexer;
use crate::token::{Span, Token, TokenKind};

/// Result type for parser operations.
pub type ParseResult<T> = Result<T, ParseError>;

/// Parse error with message and location.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl ParseError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parser for GoX source code.
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
    peek: Token,
    /// Whether composite literals are allowed in the current context.
    allow_composite_lit: bool,
    /// Set to true when expression parser encounters .(type) syntax.
    saw_type_switch: bool,
}

impl<'a> Parser<'a> {
    /// Create a new parser for the given source code.
    pub fn new(source: &'a str) -> Self {
        let mut lexer = Lexer::new(source);
        let current = lexer.next_token();
        let peek = lexer.next_token();
        Self {
            lexer,
            current,
            peek,
            allow_composite_lit: true,
            saw_type_switch: false,
        }
    }

    /// Parse a complete source file.
    pub fn parse_file(&mut self) -> ParseResult<SourceFile> {
        let start = self.current.span;

        // Parse package declaration
        let package = if self.cur_is(&TokenKind::Package) {
            self.next_token();
            let name = self.parse_ident()?;
            self.expect_semi()?;
            Some(name)
        } else {
            None
        };

        // Parse import declarations
        let mut imports = Vec::new();
        while self.cur_is(&TokenKind::Import) {
            imports.push(self.parse_import_decl()?);
        }

        // Parse top-level declarations
        let mut decls = Vec::new();
        while !self.at_eof() {
            // Skip stray semicolons at top level
            if self.eat(&TokenKind::Semi) {
                continue;
            }
            decls.push(self.parse_top_decl()?);
        }

        let end = if decls.is_empty() {
            if imports.is_empty() {
                start
            } else {
                imports.last().unwrap().span
            }
        } else {
            match decls.last().unwrap() {
                TopDecl::Var(d) => d.span,
                TopDecl::Const(d) => d.span,
                TopDecl::Type(d) => d.span,
                TopDecl::Interface(d) => d.span,
                TopDecl::Implements(d) => d.span,
                TopDecl::Func(d) => d.span,
            }
        };

        Ok(SourceFile {
            package,
            imports,
            decls,
            span: start.to(&end),
        })
    }

    /// Parse an import declaration.
    fn parse_import_decl(&mut self) -> ParseResult<ImportDecl> {
        let start = self.expect(&TokenKind::Import)?;
        let path = match &self.current.kind {
            TokenKind::String(s) => s.clone(),
            _ => return Err(self.error("expected string literal")),
        };
        let path_span = self.current.span;
        self.next_token();
        self.expect_semi()?;
        Ok(ImportDecl {
            path,
            span: start.to(&path_span),
        })
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Token Management
    // ═══════════════════════════════════════════════════════════════════════

    /// Advance to the next token.
    fn next_token(&mut self) {
        self.current = std::mem::replace(&mut self.peek, self.lexer.next_token());
    }

    /// Check if current token matches the given kind.
    fn cur_is(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.current.kind) == std::mem::discriminant(kind)
    }

    /// Check if peek token matches the given kind.
    fn peek_is(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek.kind) == std::mem::discriminant(kind)
    }

    /// Check if at end of file.
    fn at_eof(&self) -> bool {
        self.cur_is(&TokenKind::Eof)
    }

    /// Consume current token if it matches, return true if consumed.
    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.cur_is(kind) {
            self.next_token();
            true
        } else {
            false
        }
    }

    /// Expect current token to match, consume and return its span.
    fn expect(&mut self, kind: &TokenKind) -> ParseResult<Span> {
        if self.cur_is(kind) {
            let span = self.current.span;
            self.next_token();
            Ok(span)
        } else {
            Err(self.error(&format!(
                "expected {}, found {}",
                kind.name(),
                self.current.kind.name()
            )))
        }
    }

    /// Expect and consume a semicolon.
    fn expect_semi(&mut self) -> ParseResult<()> {
        self.expect(&TokenKind::Semi)?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Error Helpers
    // ═══════════════════════════════════════════════════════════════════════

    /// Create an error at the current position.
    fn error(&self, message: &str) -> ParseError {
        ParseError::new(message, self.current.span)
    }

    /// Create an error at a specific span.
    fn error_at(&self, message: &str, span: Span) -> ParseError {
        ParseError::new(message, span)
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Common Parsers
    // ═══════════════════════════════════════════════════════════════════════

    /// Parse an identifier.
    fn parse_ident(&mut self) -> ParseResult<Ident> {
        match &self.current.kind {
            TokenKind::Ident(name) => {
                let ident = Ident {
                    name: name.clone(),
                    span: self.current.span,
                };
                self.next_token();
                Ok(ident)
            }
            _ => Err(self.error("expected identifier")),
        }
    }

    /// Parse a comma-separated list of identifiers.
    fn parse_ident_list(&mut self) -> ParseResult<Vec<Ident>> {
        let mut idents = vec![self.parse_ident()?];
        while self.eat(&TokenKind::Comma) {
            idents.push(self.parse_ident()?);
        }
        Ok(idents)
    }

    /// Parse a comma-separated list of expressions.
    fn parse_expr_list(&mut self) -> ParseResult<Vec<Expr>> {
        let mut exprs = vec![self.parse_expr()?];
        while self.eat(&TokenKind::Comma) {
            exprs.push(self.parse_expr()?);
        }
        Ok(exprs)
    }

    /// Check if current token starts a type.
    fn is_type_start(&self) -> bool {
        matches!(
            &self.current.kind,
            TokenKind::Ident(_)
                | TokenKind::LBracket
                | TokenKind::Map
                | TokenKind::Chan
                | TokenKind::Func
                | TokenKind::Struct
        )
    }
}

/// Parse a GoX source file.
pub fn parse(source: &str) -> ParseResult<SourceFile> {
    Parser::new(source).parse_file()
}
