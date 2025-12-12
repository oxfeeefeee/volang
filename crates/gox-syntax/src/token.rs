//! Token definitions for GoX lexer.

use std::fmt;

pub use gox_common::Span;

/// A token with its kind and source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// Token kinds for GoX.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // ═══════════════════════════════════════════════════════════════════════
    // Literals
    // ═══════════════════════════════════════════════════════════════════════
    Ident(String),
    Int(i64),
    Float(f64),
    String(String),

    // ═══════════════════════════════════════════════════════════════════════
    // Keywords
    // ═══════════════════════════════════════════════════════════════════════

    // Declaration keywords
    Package,
    Import,
    Var,
    Const,
    Type,
    Func,
    Interface,
    Implements,
    Struct,
    Map,
    Chan,

    // Control flow keywords
    If,
    Else,
    For,
    Range,
    Switch,
    Case,
    Default,
    Return,
    Break,
    Continue,
    Goto,
    Fallthrough,
    Select,

    // Concurrency keywords
    Go,
    Defer,

    // Literal keywords
    True,
    False,
    Nil,

    // ═══════════════════════════════════════════════════════════════════════
    // Operators
    // ═══════════════════════════════════════════════════════════════════════

    // Arithmetic
    Plus,    // +
    Minus,   // -
    Star,    // *
    Slash,   // /
    Percent, // %

    // Comparison
    Eq,    // ==
    NotEq, // !=
    Lt,    // <
    LtEq,  // <=
    Gt,    // >
    GtEq,  // >=

    // Logical
    And, // &&
    Or,  // ||
    Not, // !

    // Channel
    Arrow, // <-

    // Misc
    Ellipsis, // ...

    // ═══════════════════════════════════════════════════════════════════════
    // Assignment
    // ═══════════════════════════════════════════════════════════════════════
    Assign,        // =
    ColonAssign,   // :=
    PlusAssign,    // +=
    MinusAssign,   // -=
    StarAssign,    // *=
    SlashAssign,   // /=
    PercentAssign, // %=

    // ═══════════════════════════════════════════════════════════════════════
    // Delimiters
    // ═══════════════════════════════════════════════════════════════════════
    LParen,   // (
    RParen,   // )
    LBracket, // [
    RBracket, // ]
    LBrace,   // {
    RBrace,   // }
    Comma,    // ,
    Colon,    // :
    Semi,     // ;
    Dot,      // .

    // ═══════════════════════════════════════════════════════════════════════
    // Special
    // ═══════════════════════════════════════════════════════════════════════
    Eof,
    Invalid(char),
    UnterminatedString,
}

impl TokenKind {
    /// Get a human-readable name for this token kind.
    pub fn name(&self) -> &'static str {
        match self {
            TokenKind::Ident(_) => "identifier",
            TokenKind::Int(_) => "integer",
            TokenKind::Float(_) => "float",
            TokenKind::String(_) => "string",
            TokenKind::Package => "package",
            TokenKind::Import => "import",
            TokenKind::Var => "var",
            TokenKind::Const => "const",
            TokenKind::Type => "type",
            TokenKind::Func => "func",
            TokenKind::Interface => "interface",
            TokenKind::Implements => "implements",
            TokenKind::Struct => "struct",
            TokenKind::Map => "map",
            TokenKind::Chan => "chan",
            TokenKind::If => "if",
            TokenKind::Else => "else",
            TokenKind::For => "for",
            TokenKind::Range => "range",
            TokenKind::Switch => "switch",
            TokenKind::Case => "case",
            TokenKind::Default => "default",
            TokenKind::Return => "return",
            TokenKind::Break => "break",
            TokenKind::Continue => "continue",
            TokenKind::Goto => "goto",
            TokenKind::Fallthrough => "fallthrough",
            TokenKind::Select => "select",
            TokenKind::Go => "go",
            TokenKind::Defer => "defer",
            TokenKind::True => "true",
            TokenKind::False => "false",
            TokenKind::Nil => "nil",
            TokenKind::Plus => "+",
            TokenKind::Minus => "-",
            TokenKind::Star => "*",
            TokenKind::Slash => "/",
            TokenKind::Percent => "%",
            TokenKind::Eq => "==",
            TokenKind::NotEq => "!=",
            TokenKind::Lt => "<",
            TokenKind::LtEq => "<=",
            TokenKind::Gt => ">",
            TokenKind::GtEq => ">=",
            TokenKind::And => "&&",
            TokenKind::Or => "||",
            TokenKind::Not => "!",
            TokenKind::Arrow => "<-",
            TokenKind::Ellipsis => "...",
            TokenKind::Assign => "=",
            TokenKind::ColonAssign => ":=",
            TokenKind::PlusAssign => "+=",
            TokenKind::MinusAssign => "-=",
            TokenKind::StarAssign => "*=",
            TokenKind::SlashAssign => "/=",
            TokenKind::PercentAssign => "%=",
            TokenKind::LParen => "(",
            TokenKind::RParen => ")",
            TokenKind::LBracket => "[",
            TokenKind::RBracket => "]",
            TokenKind::LBrace => "{",
            TokenKind::RBrace => "}",
            TokenKind::Comma => ",",
            TokenKind::Colon => ":",
            TokenKind::Semi => ";",
            TokenKind::Dot => ".",
            TokenKind::Eof => "end of file",
            TokenKind::Invalid(_) => "invalid character",
            TokenKind::UnterminatedString => "unterminated string",
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}
