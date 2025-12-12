//! Abstract Syntax Tree definitions for GoX.

use crate::token::Span;

// ═══════════════════════════════════════════════════════════════════════════
// Source File
// ═══════════════════════════════════════════════════════════════════════════

/// A complete GoX source file.
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub package: Option<Ident>,
    pub imports: Vec<ImportDecl>,
    pub decls: Vec<TopDecl>,
    pub span: Span,
}

/// Import declaration.
#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub path: String,
    pub span: Span,
}

// ═══════════════════════════════════════════════════════════════════════════
// Top-Level Declarations
// ═══════════════════════════════════════════════════════════════════════════

/// Top-level declarations.
#[derive(Debug, Clone)]
pub enum TopDecl {
    Var(VarDecl),
    Const(ConstDecl),
    Type(TypeDecl),
    Interface(InterfaceDecl),
    Implements(ImplementsDecl),
    Func(FuncDecl),
}

/// Variable declaration.
#[derive(Debug, Clone)]
pub struct VarDecl {
    pub specs: Vec<VarSpec>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct VarSpec {
    pub names: Vec<Ident>,
    pub ty: Option<Type>,
    pub values: Vec<Expr>,
    pub span: Span,
}

/// Constant declaration.
#[derive(Debug, Clone)]
pub struct ConstDecl {
    pub specs: Vec<ConstSpec>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ConstSpec {
    pub names: Vec<Ident>,
    pub ty: Option<Type>,
    pub values: Vec<Expr>,
    pub span: Span,
}

/// Type declaration.
#[derive(Debug, Clone)]
pub struct TypeDecl {
    pub name: Ident,
    pub ty: Type,
    pub span: Span,
}

/// Interface declaration.
#[derive(Debug, Clone)]
pub struct InterfaceDecl {
    pub name: Ident,
    pub elements: Vec<InterfaceElem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum InterfaceElem {
    Method(MethodSpec),
    Embedded(Ident),
}

#[derive(Debug, Clone)]
pub struct MethodSpec {
    pub name: Ident,
    pub params: Vec<Param>,
    pub result: Option<ResultType>,
    pub span: Span,
}

/// Implements declaration.
#[derive(Debug, Clone)]
pub struct ImplementsDecl {
    pub type_name: Ident,
    pub interfaces: Vec<Ident>,
    pub span: Span,
}

/// Function declaration.
#[derive(Debug, Clone)]
pub struct FuncDecl {
    pub receiver: Option<Receiver>,
    pub name: Ident,
    pub params: Vec<Param>,
    pub result: Option<ResultType>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Receiver {
    pub name: Ident,
    pub ty: Ident,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub names: Vec<Ident>,
    pub ty: Type,
    pub variadic: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ResultType {
    Single(Type),
    Tuple(Vec<Type>, Span),
}

impl ResultType {
    pub fn span(&self) -> Span {
        match self {
            ResultType::Single(t) => t.span(),
            ResultType::Tuple(_, s) => *s,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Types
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub enum Type {
    Named(Ident),
    Array(Box<ArrayType>),
    Slice(Box<SliceType>),
    Map(Box<MapType>),
    Chan(Box<ChanType>),
    Func(Box<FuncType>),
    Struct(Box<StructType>),
}

impl Type {
    pub fn span(&self) -> Span {
        match self {
            Type::Named(id) => id.span,
            Type::Array(a) => a.span,
            Type::Slice(s) => s.span,
            Type::Map(m) => m.span,
            Type::Chan(c) => c.span,
            Type::Func(f) => f.span,
            Type::Struct(s) => s.span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ArrayType {
    pub len: i64,
    pub elem: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct SliceType {
    pub elem: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MapType {
    pub key: Type,
    pub value: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ChanType {
    pub elem: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FuncType {
    pub params: Vec<Type>,
    pub result: Option<Box<ResultType>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructType {
    pub fields: Vec<FieldDecl>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub names: Vec<Ident>,
    pub ty: Type,
    pub tag: Option<String>,
    pub span: Span,
}

// ═══════════════════════════════════════════════════════════════════════════
// Statements
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub enum Stmt {
    Block(Block),
    Var(VarDecl),
    Const(ConstDecl),
    ShortVar(ShortVarDecl),
    Assign(Assignment),
    Expr(ExprStmt),
    Return(ReturnStmt),
    If(Box<IfStmt>),
    For(Box<ForStmt>),
    ForRange(Box<ForRangeStmt>),
    Switch(Box<SwitchStmt>),
    TypeSwitch(Box<TypeSwitchStmt>),
    Select(Box<SelectStmt>),
    Go(GoStmt),
    Defer(DeferStmt),
    Send(SendStmt),
    Goto(GotoStmt),
    Labeled(Box<LabeledStmt>),
    Fallthrough(Span),
    Break(BreakStmt),
    Continue(ContinueStmt),
    Empty(Span),
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Block(b) => b.span,
            Stmt::Var(d) => d.span,
            Stmt::Const(d) => d.span,
            Stmt::ShortVar(d) => d.span,
            Stmt::Assign(a) => a.span,
            Stmt::Expr(e) => e.span,
            Stmt::Return(r) => r.span,
            Stmt::If(i) => i.span,
            Stmt::For(f) => f.span,
            Stmt::ForRange(f) => f.span,
            Stmt::Switch(s) => s.span,
            Stmt::TypeSwitch(t) => t.span,
            Stmt::Select(s) => s.span,
            Stmt::Go(g) => g.span,
            Stmt::Defer(d) => d.span,
            Stmt::Send(s) => s.span,
            Stmt::Goto(g) => g.span,
            Stmt::Labeled(l) => l.span,
            Stmt::Fallthrough(s) => *s,
            Stmt::Break(b) => b.span,
            Stmt::Continue(c) => c.span,
            Stmt::Empty(s) => *s,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ShortVarDecl {
    pub names: Vec<Ident>,
    pub values: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Assignment {
    pub left: Vec<Expr>,
    pub op: AssignOp,
    pub right: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    PercentAssign,
}

#[derive(Debug, Clone)]
pub struct ExprStmt {
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub values: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub init: Option<Box<Stmt>>,
    pub cond: Expr,
    pub then_block: Block,
    pub else_clause: Option<ElseClause>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ElseClause {
    Block(Block),
    If(Box<IfStmt>),
}

#[derive(Debug, Clone)]
pub struct ForStmt {
    pub init: Option<Box<Stmt>>,
    pub cond: Option<Expr>,
    pub post: Option<Box<Stmt>>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ForRangeStmt {
    pub vars: Option<Vec<Ident>>,
    pub is_define: bool,
    pub expr: Expr,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct SwitchStmt {
    pub init: Option<Box<Stmt>>,
    pub expr: Option<Expr>,
    pub cases: Vec<CaseClause>,
    pub default: Option<DefaultClause>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CaseClause {
    pub exprs: Vec<Expr>,
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct DefaultClause {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeSwitchStmt {
    pub binding: Option<Ident>,
    pub expr: Expr,
    pub cases: Vec<TypeCaseClause>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeCaseClause {
    pub types: Option<Vec<TypeOrNil>>,
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypeOrNil {
    Type(Type),
    Nil(Span),
}

impl TypeOrNil {
    pub fn span(&self) -> Span {
        match self {
            TypeOrNil::Type(t) => t.span(),
            TypeOrNil::Nil(s) => *s,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SelectStmt {
    pub cases: Vec<SelectCase>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct SelectCase {
    pub comm: Option<CommClause>,
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum CommClause {
    Send(SendStmt),
    Recv(RecvStmt),
}

#[derive(Debug, Clone)]
pub struct RecvStmt {
    pub vars: Option<Vec<Ident>>,
    pub is_define: bool,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct GoStmt {
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct DeferStmt {
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct SendStmt {
    pub chan: Expr,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct GotoStmt {
    pub label: Ident,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct LabeledStmt {
    pub label: Ident,
    pub stmt: Stmt,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct BreakStmt {
    pub label: Option<Ident>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ContinueStmt {
    pub label: Option<Ident>,
    pub span: Span,
}

// ═══════════════════════════════════════════════════════════════════════════
// Expressions
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub enum Expr {
    Ident(Ident),
    Literal(Literal),
    Binary(Box<BinaryExpr>),
    Unary(Box<UnaryExpr>),
    Call(Box<CallExpr>),
    Index(Box<IndexExpr>),
    Slice(Box<SliceExpr>),
    Selector(Box<SelectorExpr>),
    CompositeLit(Box<CompositeLit>),
    Grouped(Box<Expr>, Span),
    Receive(Box<ReceiveExpr>),
    TypeAssert(Box<TypeAssertExpr>),
    FuncLit(Box<FuncLit>),
    Make(Box<MakeExpr>),
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Ident(id) => id.span,
            Expr::Literal(lit) => lit.span(),
            Expr::Binary(b) => b.span,
            Expr::Unary(u) => u.span,
            Expr::Call(c) => c.span,
            Expr::Index(i) => i.span,
            Expr::Slice(s) => s.span,
            Expr::Selector(s) => s.span,
            Expr::CompositeLit(c) => c.span,
            Expr::Grouped(_, s) => *s,
            Expr::Receive(r) => r.span,
            Expr::TypeAssert(t) => t.span,
            Expr::FuncLit(f) => f.span,
            Expr::Make(m) => m.span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64, Span),
    Float(f64, Span),
    String(String, Span),
    Bool(bool, Span),
    Nil(Span),
}

impl Literal {
    pub fn span(&self) -> Span {
        match self {
            Literal::Int(_, s) => *s,
            Literal::Float(_, s) => *s,
            Literal::String(_, s) => *s,
            Literal::Bool(_, s) => *s,
            Literal::Nil(s) => *s,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub left: Expr,
    pub op: BinaryOp,
    pub right: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    Pos,
}

#[derive(Debug, Clone)]
pub struct CallExpr {
    pub func: Expr,
    pub args: Vec<Expr>,
    pub spread: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IndexExpr {
    pub expr: Expr,
    pub index: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct SliceExpr {
    pub expr: Expr,
    pub low: Option<Box<Expr>>,
    pub high: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct SelectorExpr {
    pub expr: Expr,
    pub field: Ident,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CompositeLit {
    pub ty: Option<Type>,
    pub elements: Vec<Element>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Element {
    pub key: Option<ElementKey>,
    pub value: ElementValue,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ElementKey {
    Ident(Ident),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub enum ElementValue {
    Expr(Expr),
    Lit(Vec<Element>, Span),
}

#[derive(Debug, Clone)]
pub struct ReceiveExpr {
    pub chan: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeAssertExpr {
    pub expr: Expr,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FuncLit {
    pub params: Vec<Param>,
    pub result: Option<ResultType>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MakeExpr {
    pub ty: Type,
    pub size: Option<Expr>,
    pub cap: Option<Expr>,
    pub span: Span,
}
