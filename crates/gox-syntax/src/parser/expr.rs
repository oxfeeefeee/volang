//! Expression parsing for GoX using Pratt parsing.

use super::{ParseResult, Parser};
use crate::ast::{
    ArrayType, BinaryExpr, BinaryOp, CallExpr, CompositeLit, Element, ElementKey, ElementValue,
    Expr, FuncLit, IndexExpr, Literal, MakeExpr, MapType, ReceiveExpr, SelectorExpr, SliceExpr,
    SliceType, StructType, Type, TypeAssertExpr, UnaryExpr, UnaryOp,
};
use crate::token::TokenKind;

// ═══════════════════════════════════════════════════════════════════════════
// Precedence Levels
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum Precedence {
    Lowest = 0,
    Or,          // ||
    And,         // &&
    Equals,      // == !=
    LessGreater, // < <= > >=
    Sum,         // + -
    Product,     // * / %
    Prefix,      // -x !x
    Call,        // f(x) a[i] a.b
}

impl Precedence {
    fn from_token(kind: &TokenKind) -> Self {
        match kind {
            TokenKind::Or => Precedence::Or,
            TokenKind::And => Precedence::And,
            TokenKind::Eq | TokenKind::NotEq => Precedence::Equals,
            TokenKind::Lt | TokenKind::LtEq | TokenKind::Gt | TokenKind::GtEq => {
                Precedence::LessGreater
            }
            TokenKind::Plus | TokenKind::Minus => Precedence::Sum,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Precedence::Product,
            TokenKind::LParen | TokenKind::LBracket | TokenKind::Dot => Precedence::Call,
            _ => Precedence::Lowest,
        }
    }
}

impl<'a> Parser<'a> {
    // ═══════════════════════════════════════════════════════════════════════
    // Expression Entry Point
    // ═══════════════════════════════════════════════════════════════════════

    pub(super) fn parse_expr(&mut self) -> ParseResult<Expr> {
        self.parse_expr_prec(Precedence::Lowest)
    }

    fn parse_expr_prec(&mut self, min_prec: Precedence) -> ParseResult<Expr> {
        let mut left = self.parse_prefix()?;

        while !self.at_eof() {
            let prec = Precedence::from_token(&self.current.kind);
            if prec <= min_prec {
                break;
            }

            left = self.parse_infix(left)?;
        }

        Ok(left)
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Prefix Expressions
    // ═══════════════════════════════════════════════════════════════════════

    fn parse_prefix(&mut self) -> ParseResult<Expr> {
        match &self.current.kind {
            TokenKind::Ident(name) => {
                // Check for make builtin
                if name == "make" {
                    return self.parse_make_expr();
                }
                let id = self.parse_ident()?;
                self.maybe_parse_composite_lit(Expr::Ident(id))
            }
            TokenKind::Int(n) => {
                let lit = Literal::Int(*n, self.current.span);
                self.next_token();
                Ok(Expr::Literal(lit))
            }
            TokenKind::Float(n) => {
                let lit = Literal::Float(*n, self.current.span);
                self.next_token();
                Ok(Expr::Literal(lit))
            }
            TokenKind::String(s) => {
                let lit = Literal::String(s.clone(), self.current.span);
                self.next_token();
                Ok(Expr::Literal(lit))
            }
            TokenKind::True => {
                let lit = Literal::Bool(true, self.current.span);
                self.next_token();
                Ok(Expr::Literal(lit))
            }
            TokenKind::False => {
                let lit = Literal::Bool(false, self.current.span);
                self.next_token();
                Ok(Expr::Literal(lit))
            }
            TokenKind::Nil => {
                let lit = Literal::Nil(self.current.span);
                self.next_token();
                Ok(Expr::Literal(lit))
            }
            TokenKind::Minus => {
                let start = self.current.span;
                self.next_token();
                let expr = self.parse_expr_prec(Precedence::Prefix)?;
                Ok(Expr::Unary(Box::new(UnaryExpr {
                    op: UnaryOp::Neg,
                    span: start.to(&expr.span()),
                    expr,
                })))
            }
            TokenKind::Plus => {
                let start = self.current.span;
                self.next_token();
                let expr = self.parse_expr_prec(Precedence::Prefix)?;
                Ok(Expr::Unary(Box::new(UnaryExpr {
                    op: UnaryOp::Pos,
                    span: start.to(&expr.span()),
                    expr,
                })))
            }
            TokenKind::Not => {
                let start = self.current.span;
                self.next_token();
                let expr = self.parse_expr_prec(Precedence::Prefix)?;
                Ok(Expr::Unary(Box::new(UnaryExpr {
                    op: UnaryOp::Not,
                    span: start.to(&expr.span()),
                    expr,
                })))
            }
            TokenKind::Arrow => {
                // Receive expression: <-ch
                let start = self.current.span;
                self.next_token();
                let chan = self.parse_expr_prec(Precedence::Prefix)?;
                Ok(Expr::Receive(Box::new(ReceiveExpr {
                    span: start.to(&chan.span()),
                    chan,
                })))
            }
            TokenKind::LParen => {
                let start = self.current.span;
                self.next_token();
                let expr = self.parse_expr()?;
                let end = self.expect(&TokenKind::RParen)?;
                Ok(Expr::Grouped(Box::new(expr), start.to(&end)))
            }
            TokenKind::Func => self.parse_func_lit(),
            TokenKind::LBracket => self.parse_array_or_slice_lit(),
            TokenKind::Map => self.parse_map_lit(),
            TokenKind::Struct => self.parse_struct_lit(),
            _ => Err(self.error("unexpected token in expression")),
        }
    }

    fn maybe_parse_composite_lit(&mut self, expr: Expr) -> ParseResult<Expr> {
        if !self.allow_composite_lit || !self.cur_is(&TokenKind::LBrace) {
            return Ok(expr);
        }

        // expr must be a type (identifier)
        let ty = match expr {
            Expr::Ident(id) => Type::Named(id),
            _ => return Ok(expr),
        };

        self.parse_composite_lit_with_type(Some(ty))
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Infix Expressions
    // ═══════════════════════════════════════════════════════════════════════

    fn parse_infix(&mut self, left: Expr) -> ParseResult<Expr> {
        match &self.current.kind {
            // Binary operators
            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::Eq
            | TokenKind::NotEq
            | TokenKind::Lt
            | TokenKind::LtEq
            | TokenKind::Gt
            | TokenKind::GtEq
            | TokenKind::And
            | TokenKind::Or => {
                let op = self.token_to_binary_op(&self.current.kind);
                let prec = Precedence::from_token(&self.current.kind);
                self.next_token();
                let right = self.parse_expr_prec(prec)?;
                Ok(Expr::Binary(Box::new(BinaryExpr {
                    span: left.span().to(&right.span()),
                    left,
                    op,
                    right,
                })))
            }

            // Call
            TokenKind::LParen => {
                self.next_token();
                let (args, spread) = self.parse_call_args()?;
                let end = self.expect(&TokenKind::RParen)?;
                Ok(Expr::Call(Box::new(CallExpr {
                    span: left.span().to(&end),
                    func: left,
                    args,
                    spread,
                })))
            }

            // Index or Slice
            TokenKind::LBracket => {
                self.next_token();

                // Check for slice starting with : (e.g., a[:high])
                if self.cur_is(&TokenKind::Colon) {
                    self.next_token();
                    let high = if self.cur_is(&TokenKind::RBracket) {
                        None
                    } else {
                        Some(Box::new(self.parse_expr()?))
                    };
                    let end = self.expect(&TokenKind::RBracket)?;
                    return Ok(Expr::Slice(Box::new(SliceExpr {
                        span: left.span().to(&end),
                        expr: left,
                        low: None,
                        high,
                    })));
                }

                let first = self.parse_expr()?;

                // Check for slice expression
                if self.cur_is(&TokenKind::Colon) {
                    self.next_token();
                    let high = if self.cur_is(&TokenKind::RBracket) {
                        None
                    } else {
                        Some(Box::new(self.parse_expr()?))
                    };
                    let end = self.expect(&TokenKind::RBracket)?;
                    return Ok(Expr::Slice(Box::new(SliceExpr {
                        span: left.span().to(&end),
                        expr: left,
                        low: Some(Box::new(first)),
                        high,
                    })));
                }

                // Regular index
                let end = self.expect(&TokenKind::RBracket)?;
                Ok(Expr::Index(Box::new(IndexExpr {
                    span: left.span().to(&end),
                    expr: left,
                    index: first,
                })))
            }

            // Selector or Type Assertion
            TokenKind::Dot => {
                let start = left.span();
                self.next_token(); // consume dot

                if self.cur_is(&TokenKind::LParen) {
                    // Check for .(type) - type switch syntax
                    if self.peek_is(&TokenKind::Type) {
                        // Consume the rest of .(type) and return left
                        // Set flag so switch parser knows this is a type switch
                        self.next_token(); // consume LParen
                        self.next_token(); // consume Type
                        self.expect(&TokenKind::RParen)?;
                        self.saw_type_switch = true;
                        return Ok(left);
                    }
                    // Type assertion: x.(T)
                    self.next_token(); // consume LParen
                    let ty = self.parse_type()?;
                    let end = self.expect(&TokenKind::RParen)?;
                    return Ok(Expr::TypeAssert(Box::new(TypeAssertExpr {
                        expr: left,
                        ty,
                        span: start.to(&end),
                    })));
                }

                // Selector: x.field
                let field = self.parse_ident()?;
                Ok(Expr::Selector(Box::new(SelectorExpr {
                    span: start.to(&field.span),
                    expr: left,
                    field,
                })))
            }

            _ => Ok(left),
        }
    }

    fn token_to_binary_op(&self, kind: &TokenKind) -> BinaryOp {
        match kind {
            TokenKind::Plus => BinaryOp::Add,
            TokenKind::Minus => BinaryOp::Sub,
            TokenKind::Star => BinaryOp::Mul,
            TokenKind::Slash => BinaryOp::Div,
            TokenKind::Percent => BinaryOp::Mod,
            TokenKind::Eq => BinaryOp::Eq,
            TokenKind::NotEq => BinaryOp::NotEq,
            TokenKind::Lt => BinaryOp::Lt,
            TokenKind::LtEq => BinaryOp::LtEq,
            TokenKind::Gt => BinaryOp::Gt,
            TokenKind::GtEq => BinaryOp::GtEq,
            TokenKind::And => BinaryOp::And,
            TokenKind::Or => BinaryOp::Or,
            _ => unreachable!(),
        }
    }

    fn parse_call_args(&mut self) -> ParseResult<(Vec<Expr>, bool)> {
        let mut args = Vec::new();
        let mut spread = false;

        if self.cur_is(&TokenKind::RParen) {
            return Ok((args, spread));
        }

        args.push(self.parse_expr()?);
        while self.eat(&TokenKind::Comma) {
            if self.cur_is(&TokenKind::RParen) {
                break;
            }
            args.push(self.parse_expr()?);
        }

        // Check for spread: f(args...)
        if self.eat(&TokenKind::Ellipsis) {
            spread = true;
        }

        Ok((args, spread))
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Composite Literals
    // ═══════════════════════════════════════════════════════════════════════

    fn parse_array_or_slice_lit(&mut self) -> ParseResult<Expr> {
        let start = self.expect(&TokenKind::LBracket)?;

        if self.cur_is(&TokenKind::RBracket) {
            // Slice type: []T{...}
            self.next_token();
            let elem = self.parse_type()?;
            let ty = Type::Slice(Box::new(SliceType {
                span: start.to(&elem.span()),
                elem,
            }));
            return self.parse_composite_lit_with_type(Some(ty));
        }

        // Array type: [N]T{...}
        let len = match &self.current.kind {
            TokenKind::Int(n) => *n,
            _ => return Err(self.error("expected array length")),
        };
        self.next_token();
        self.expect(&TokenKind::RBracket)?;
        let elem = self.parse_type()?;
        let ty = Type::Array(Box::new(ArrayType {
            len,
            span: start.to(&elem.span()),
            elem,
        }));
        self.parse_composite_lit_with_type(Some(ty))
    }

    fn parse_map_lit(&mut self) -> ParseResult<Expr> {
        let start = self.expect(&TokenKind::Map)?;
        self.expect(&TokenKind::LBracket)?;
        let key = self.parse_type()?;
        self.expect(&TokenKind::RBracket)?;
        let value = self.parse_type()?;
        let end = value.span();
        let ty = Type::Map(Box::new(MapType {
            key,
            value,
            span: start.to(&end),
        }));
        self.parse_composite_lit_with_type(Some(ty))
    }

    fn parse_struct_lit(&mut self) -> ParseResult<Expr> {
        let start = self.expect(&TokenKind::Struct)?;
        self.expect(&TokenKind::LBrace)?;

        let mut fields = Vec::new();
        while !self.cur_is(&TokenKind::RBrace) && !self.at_eof() {
            fields.push(self.parse_field_decl()?);
        }

        let end = self.expect(&TokenKind::RBrace)?;
        let ty = Type::Struct(Box::new(StructType {
            fields,
            span: start.to(&end),
        }));

        // Anonymous struct literal
        self.parse_composite_lit_with_type(Some(ty))
    }

    fn parse_composite_lit_with_type(&mut self, ty: Option<Type>) -> ParseResult<Expr> {
        let start = ty.as_ref().map(|t| t.span()).unwrap_or(self.current.span);
        self.expect(&TokenKind::LBrace)?;

        let elements = self.parse_element_list()?;

        let end = self.expect(&TokenKind::RBrace)?;

        Ok(Expr::CompositeLit(Box::new(CompositeLit {
            ty,
            elements,
            span: start.to(&end),
        })))
    }

    fn parse_element_list(&mut self) -> ParseResult<Vec<Element>> {
        let mut elements = Vec::new();

        if self.cur_is(&TokenKind::RBrace) {
            return Ok(elements);
        }

        elements.push(self.parse_element()?);
        while self.eat(&TokenKind::Comma) {
            if self.cur_is(&TokenKind::RBrace) {
                break;
            }
            elements.push(self.parse_element()?);
        }

        Ok(elements)
    }

    fn parse_element(&mut self) -> ParseResult<Element> {
        let start = self.current.span;

        // Check for nested literal without type: { ... }
        if self.cur_is(&TokenKind::LBrace) {
            let inner_start = self.current.span;
            self.next_token();
            let inner_elements = self.parse_element_list()?;
            let inner_end = self.expect(&TokenKind::RBrace)?;
            return Ok(Element {
                key: None,
                value: ElementValue::Lit(inner_elements, inner_start.to(&inner_end)),
                span: start.to(&inner_end),
            });
        }

        // Parse first expression
        let first = self.parse_expr()?;

        // Check for key: value
        if self.eat(&TokenKind::Colon) {
            let key = match first {
                Expr::Ident(id) => ElementKey::Ident(id),
                other => ElementKey::Expr(other),
            };

            // Value could be nested literal or expression
            if self.cur_is(&TokenKind::LBrace) {
                let inner_start = self.current.span;
                self.next_token();
                let inner_elements = self.parse_element_list()?;
                let inner_end = self.expect(&TokenKind::RBrace)?;
                return Ok(Element {
                    key: Some(key),
                    value: ElementValue::Lit(inner_elements, inner_start.to(&inner_end)),
                    span: start.to(&inner_end),
                });
            }

            let value = self.parse_expr()?;
            let end = value.span();
            return Ok(Element {
                key: Some(key),
                value: ElementValue::Expr(value),
                span: start.to(&end),
            });
        }

        // Just value
        let end = first.span();
        Ok(Element {
            key: None,
            value: ElementValue::Expr(first),
            span: start.to(&end),
        })
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Function Literals
    // ═══════════════════════════════════════════════════════════════════════

    fn parse_func_lit(&mut self) -> ParseResult<Expr> {
        let start = self.expect(&TokenKind::Func)?;
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(&TokenKind::RParen)?;

        let result = self.parse_optional_result()?;
        let body = self.parse_block()?;

        Ok(Expr::FuncLit(Box::new(FuncLit {
            params,
            result,
            span: start.to(&body.span),
            body,
        })))
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Make Expression
    // ═══════════════════════════════════════════════════════════════════════

    fn parse_make_expr(&mut self) -> ParseResult<Expr> {
        let start = self.current.span;
        self.next_token(); // consume "make"
        self.expect(&TokenKind::LParen)?;

        let ty = self.parse_type()?;

        let size = if self.eat(&TokenKind::Comma) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let cap = if self.eat(&TokenKind::Comma) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let end = self.expect(&TokenKind::RParen)?;

        Ok(Expr::Make(Box::new(MakeExpr {
            ty,
            size,
            cap,
            span: start.to(&end),
        })))
    }
}
