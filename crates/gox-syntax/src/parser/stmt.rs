//! Statement parsing for GoX.

use super::{ParseResult, Parser};
use crate::ast::{
    AssignOp, Assignment, Block, BreakStmt, CaseClause, CommClause, ContinueStmt, DefaultClause,
    DeferStmt, ElseClause, Expr, ExprStmt, ForRangeStmt, ForStmt, GoStmt, GotoStmt, Ident, IfStmt,
    LabeledStmt, RecvStmt, ReturnStmt, SelectCase, SelectStmt, SendStmt, ShortVarDecl, Stmt,
    SwitchStmt, TypeCaseClause, TypeSwitchStmt,
};
use crate::token::TokenKind;

impl<'a> Parser<'a> {
    // ═══════════════════════════════════════════════════════════════════════
    // Block
    // ═══════════════════════════════════════════════════════════════════════

    pub(super) fn parse_block(&mut self) -> ParseResult<Block> {
        let start = self.expect(&TokenKind::LBrace)?;

        let mut stmts = Vec::new();
        while !self.cur_is(&TokenKind::RBrace) && !self.at_eof() {
            stmts.push(self.parse_stmt()?);
        }

        let end = self.expect(&TokenKind::RBrace)?;

        Ok(Block {
            stmts,
            span: start.to(&end),
        })
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Statement Dispatch
    // ═══════════════════════════════════════════════════════════════════════

    pub(super) fn parse_stmt(&mut self) -> ParseResult<Stmt> {
        match &self.current.kind {
            TokenKind::LBrace => Ok(Stmt::Block(self.parse_block()?)),
            TokenKind::Var => Ok(Stmt::Var(self.parse_var_decl()?)),
            TokenKind::Const => Ok(Stmt::Const(self.parse_const_decl()?)),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::For => self.parse_for_stmt(),
            TokenKind::Switch => self.parse_switch_stmt(),
            TokenKind::Select => self.parse_select_stmt(),
            TokenKind::Go => self.parse_go_stmt(),
            TokenKind::Defer => self.parse_defer_stmt(),
            TokenKind::Goto => self.parse_goto_stmt(),
            TokenKind::Fallthrough => {
                let span = self.current.span;
                self.next_token();
                self.expect_semi()?;
                Ok(Stmt::Fallthrough(span))
            }
            TokenKind::Break => self.parse_break_stmt(),
            TokenKind::Continue => self.parse_continue_stmt(),
            TokenKind::Semi => {
                let span = self.current.span;
                self.next_token();
                Ok(Stmt::Empty(span))
            }
            _ => self.parse_simple_stmt(),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Simple Statements
    // ═══════════════════════════════════════════════════════════════════════

    fn parse_simple_stmt(&mut self) -> ParseResult<Stmt> {
        let exprs = self.parse_expr_list()?;

        // Check for labeled statement: label: stmt
        if exprs.len() == 1 && self.cur_is(&TokenKind::Colon) {
            if let Expr::Ident(label) = &exprs[0] {
                let label = label.clone();
                self.next_token(); // consume :
                let stmt = self.parse_stmt()?;
                let span = label.span.to(&stmt.span());
                return Ok(Stmt::Labeled(Box::new(LabeledStmt { label, stmt, span })));
            }
        }

        // Check for send statement: ch <- value
        if exprs.len() == 1 && self.cur_is(&TokenKind::Arrow) {
            self.next_token();
            let value = self.parse_expr()?;
            self.expect_semi()?;
            let span = exprs[0].span().to(&value.span());
            return Ok(Stmt::Send(SendStmt {
                chan: exprs.into_iter().next().unwrap(),
                value,
                span,
            }));
        }

        // Check for short var decl: x := expr
        if self.cur_is(&TokenKind::ColonAssign) {
            self.next_token();
            let values = self.parse_expr_list()?;

            let mut names = Vec::new();
            for expr in exprs {
                match expr {
                    Expr::Ident(id) => names.push(id),
                    _ => return Err(self.error_at("expected identifier", expr.span())),
                }
            }

            self.expect_semi()?;
            let span = names
                .first()
                .unwrap()
                .span
                .to(&values.last().unwrap().span());
            return Ok(Stmt::ShortVar(ShortVarDecl {
                names,
                values,
                span,
            }));
        }

        // Check for assignment
        if let Some(op) = self.try_assign_op() {
            self.next_token();
            let right = self.parse_expr_list()?;
            self.expect_semi()?;
            let span = exprs
                .first()
                .unwrap()
                .span()
                .to(&right.last().unwrap().span());
            return Ok(Stmt::Assign(Assignment {
                left: exprs,
                op,
                right,
                span,
            }));
        }

        // Expression statement
        if exprs.len() == 1 {
            let expr = exprs.into_iter().next().unwrap();
            let span = expr.span();
            self.expect_semi()?;
            return Ok(Stmt::Expr(ExprStmt { expr, span }));
        }

        Err(self.error("invalid statement"))
    }

    fn try_assign_op(&self) -> Option<AssignOp> {
        match &self.current.kind {
            TokenKind::Assign => Some(AssignOp::Assign),
            TokenKind::PlusAssign => Some(AssignOp::PlusAssign),
            TokenKind::MinusAssign => Some(AssignOp::MinusAssign),
            TokenKind::StarAssign => Some(AssignOp::StarAssign),
            TokenKind::SlashAssign => Some(AssignOp::SlashAssign),
            TokenKind::PercentAssign => Some(AssignOp::PercentAssign),
            _ => None,
        }
    }

    /// Parse a simple statement without consuming trailing semicolon.
    /// Used for if/switch init statements.
    fn parse_simple_stmt_no_semi(&mut self) -> ParseResult<Stmt> {
        let exprs = self.parse_expr_list()?;

        // Short var decl
        if self.cur_is(&TokenKind::ColonAssign) {
            self.next_token();
            let values = self.parse_expr_list()?;

            let mut names = Vec::new();
            for expr in exprs {
                match expr {
                    Expr::Ident(id) => names.push(id),
                    _ => return Err(self.error_at("expected identifier", expr.span())),
                }
            }

            let span = names
                .first()
                .unwrap()
                .span
                .to(&values.last().unwrap().span());
            return Ok(Stmt::ShortVar(ShortVarDecl {
                names,
                values,
                span,
            }));
        }

        // Assignment
        if let Some(op) = self.try_assign_op() {
            self.next_token();
            let right = self.parse_expr_list()?;
            let span = exprs
                .first()
                .unwrap()
                .span()
                .to(&right.last().unwrap().span());
            return Ok(Stmt::Assign(Assignment {
                left: exprs,
                op,
                right,
                span,
            }));
        }

        // Expression statement
        if exprs.len() == 1 {
            let expr = exprs.into_iter().next().unwrap();
            let span = expr.span();
            return Ok(Stmt::Expr(ExprStmt { expr, span }));
        }

        Err(self.error("invalid statement"))
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Return Statement
    // ═══════════════════════════════════════════════════════════════════════

    fn parse_return_stmt(&mut self) -> ParseResult<Stmt> {
        let start = self.expect(&TokenKind::Return)?;

        let values = if self.cur_is(&TokenKind::Semi) {
            Vec::new()
        } else {
            self.parse_expr_list()?
        };

        self.expect_semi()?;
        let end = values.last().map(|e| e.span()).unwrap_or(start);

        Ok(Stmt::Return(ReturnStmt {
            values,
            span: start.to(&end),
        }))
    }

    // ═══════════════════════════════════════════════════════════════════════
    // If Statement
    // ═══════════════════════════════════════════════════════════════════════

    fn parse_if_stmt(&mut self) -> ParseResult<Stmt> {
        let start = self.expect(&TokenKind::If)?;

        // Disable composite literals in condition
        let saved = self.allow_composite_lit;
        self.allow_composite_lit = false;

        // Parse first part (could be init or condition)
        let first = self.parse_simple_stmt_no_semi()?;

        let (init, cond) = if self.eat(&TokenKind::Semi) {
            // init; cond form
            let cond = self.parse_expr()?;
            (Some(Box::new(first)), cond)
        } else {
            // Just condition
            match first {
                Stmt::Expr(e) => (None, e.expr),
                _ => return Err(self.error("expected ';' after init statement")),
            }
        };

        self.allow_composite_lit = saved;

        let then_block = self.parse_block()?;

        let else_clause = if self.eat(&TokenKind::Else) {
            if self.cur_is(&TokenKind::If) {
                let if_stmt = self.parse_if_stmt()?;
                match if_stmt {
                    Stmt::If(inner) => Some(ElseClause::If(inner)),
                    _ => unreachable!(),
                }
            } else {
                Some(ElseClause::Block(self.parse_block()?))
            }
        } else {
            None
        };

        let end = else_clause
            .as_ref()
            .map(|e| match e {
                ElseClause::Block(b) => b.span,
                ElseClause::If(i) => i.span,
            })
            .unwrap_or(then_block.span);

        Ok(Stmt::If(Box::new(IfStmt {
            init,
            cond,
            then_block,
            else_clause,
            span: start.to(&end),
        })))
    }

    // ═══════════════════════════════════════════════════════════════════════
    // For Statement
    // ═══════════════════════════════════════════════════════════════════════

    fn parse_for_stmt(&mut self) -> ParseResult<Stmt> {
        let start = self.expect(&TokenKind::For)?;

        let saved = self.allow_composite_lit;
        self.allow_composite_lit = false;

        // Infinite loop: for { }
        if self.cur_is(&TokenKind::LBrace) {
            self.allow_composite_lit = saved;
            let body = self.parse_block()?;
            let body_span = body.span;
            return Ok(Stmt::For(Box::new(ForStmt {
                init: None,
                cond: None,
                post: None,
                body,
                span: start.to(&body_span),
            })));
        }

        // for range expr (no variables)
        if self.cur_is(&TokenKind::Range) {
            self.next_token();
            let expr = self.parse_expr()?;
            self.allow_composite_lit = saved;
            let body = self.parse_block()?;
            let body_span = body.span;
            return Ok(Stmt::ForRange(Box::new(ForRangeStmt {
                vars: None,
                is_define: false,
                expr,
                body,
                span: start.to(&body_span),
            })));
        }

        // Three-clause form starting with semicolon: for ; cond ; post { }
        if self.cur_is(&TokenKind::Semi) {
            self.next_token();
            let cond = if !self.cur_is(&TokenKind::Semi) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            self.expect(&TokenKind::Semi)?;

            let post = if !self.cur_is(&TokenKind::LBrace) {
                Some(Box::new(self.parse_for_post_stmt()?))
            } else {
                None
            };

            self.allow_composite_lit = saved;
            let body = self.parse_block()?;
            let body_span = body.span;

            return Ok(Stmt::For(Box::new(ForStmt {
                init: None,
                cond,
                post,
                body,
                span: start.to(&body_span),
            })));
        }

        // Parse first expression(s)
        let exprs = self.parse_expr_list()?;

        // Check for range: for i, v := range expr or for i, v = range expr
        if self.cur_is(&TokenKind::ColonAssign) || self.cur_is(&TokenKind::Assign) {
            let is_define = self.cur_is(&TokenKind::ColonAssign);
            self.next_token();

            if self.cur_is(&TokenKind::Range) {
                self.next_token();
                let range_expr = self.parse_expr()?;
                self.allow_composite_lit = saved;
                let body = self.parse_block()?;

                let mut vars = Vec::new();
                for expr in exprs {
                    match expr {
                        Expr::Ident(id) => vars.push(id),
                        _ => return Err(self.error_at("expected identifier", expr.span())),
                    }
                }

                let body_span = body.span;
                return Ok(Stmt::ForRange(Box::new(ForRangeStmt {
                    vars: Some(vars),
                    is_define,
                    expr: range_expr,
                    body,
                    span: start.to(&body_span),
                })));
            }

            // Not range, it's init statement
            let values = self.parse_expr_list()?;
            self.expect(&TokenKind::Semi)?;

            let init = if is_define {
                let mut names = Vec::new();
                for expr in exprs {
                    match expr {
                        Expr::Ident(id) => names.push(id),
                        _ => return Err(self.error_at("expected identifier", expr.span())),
                    }
                }
                let span = names
                    .first()
                    .unwrap()
                    .span
                    .to(&values.last().unwrap().span());
                Some(Box::new(Stmt::ShortVar(ShortVarDecl {
                    names,
                    values,
                    span,
                })))
            } else {
                let span = exprs
                    .first()
                    .unwrap()
                    .span()
                    .to(&values.last().unwrap().span());
                Some(Box::new(Stmt::Assign(Assignment {
                    left: exprs,
                    op: AssignOp::Assign,
                    right: values,
                    span,
                })))
            };

            let cond = if !self.cur_is(&TokenKind::Semi) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            self.expect(&TokenKind::Semi)?;

            let post = if !self.cur_is(&TokenKind::LBrace) {
                Some(Box::new(self.parse_for_post_stmt()?))
            } else {
                None
            };

            self.allow_composite_lit = saved;
            let body = self.parse_block()?;
            let body_span = body.span;

            return Ok(Stmt::For(Box::new(ForStmt {
                init,
                cond,
                post,
                body,
                span: start.to(&body_span),
            })));
        }

        // Check for semicolon (three-clause form)
        if self.eat(&TokenKind::Semi) {
            let init = if exprs.len() == 1 {
                let expr = exprs.into_iter().next().unwrap();
                let span = expr.span();
                Some(Box::new(Stmt::Expr(ExprStmt { expr, span })))
            } else {
                return Err(self.error("expected single expression in for init"));
            };

            let cond = if !self.cur_is(&TokenKind::Semi) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            self.expect(&TokenKind::Semi)?;

            let post = if !self.cur_is(&TokenKind::LBrace) {
                Some(Box::new(self.parse_for_post_stmt()?))
            } else {
                None
            };

            self.allow_composite_lit = saved;
            let body = self.parse_block()?;
            let body_span = body.span;

            return Ok(Stmt::For(Box::new(ForStmt {
                init,
                cond,
                post,
                body,
                span: start.to(&body_span),
            })));
        }

        // While-style: for cond { }
        self.allow_composite_lit = saved;

        if exprs.len() != 1 {
            return Err(self.error("expected single expression as for condition"));
        }
        let cond = Some(exprs.into_iter().next().unwrap());

        let body = self.parse_block()?;
        let body_span = body.span;
        Ok(Stmt::For(Box::new(ForStmt {
            init: None,
            cond,
            post: None,
            body,
            span: start.to(&body_span),
        })))
    }

    fn parse_for_post_stmt(&mut self) -> ParseResult<Stmt> {
        let exprs = self.parse_expr_list()?;

        if self.cur_is(&TokenKind::ColonAssign) {
            self.next_token();
            let values = self.parse_expr_list()?;

            let mut names = Vec::new();
            for expr in exprs {
                match expr {
                    Expr::Ident(id) => names.push(id),
                    _ => return Err(self.error_at("expected identifier", expr.span())),
                }
            }

            let span = names
                .first()
                .unwrap()
                .span
                .to(&values.last().unwrap().span());
            return Ok(Stmt::ShortVar(ShortVarDecl {
                names,
                values,
                span,
            }));
        }

        if let Some(op) = self.try_assign_op() {
            self.next_token();
            let right = self.parse_expr_list()?;
            let span = exprs
                .first()
                .unwrap()
                .span()
                .to(&right.last().unwrap().span());
            return Ok(Stmt::Assign(Assignment {
                left: exprs,
                op,
                right,
                span,
            }));
        }

        if exprs.len() == 1 {
            let expr = exprs.into_iter().next().unwrap();
            let span = expr.span();
            return Ok(Stmt::Expr(ExprStmt { expr, span }));
        }

        Err(self.error("invalid for post statement"))
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Switch Statement
    // ═══════════════════════════════════════════════════════════════════════

    fn parse_switch_stmt(&mut self) -> ParseResult<Stmt> {
        let start = self.expect(&TokenKind::Switch)?;

        let saved = self.allow_composite_lit;
        self.allow_composite_lit = false;

        // Check for switch { } (no expr)
        if self.cur_is(&TokenKind::LBrace) {
            self.allow_composite_lit = saved;
            return self.parse_switch_body(start, None, None);
        }

        // Parse first part - this will consume .(type) if present
        let first = self.parse_simple_stmt_no_semi()?;

        // Check if expression parser saw .(type) syntax
        if self.saw_type_switch && self.cur_is(&TokenKind::LBrace) {
            self.saw_type_switch = false;
            self.allow_composite_lit = saved;
            match first {
                Stmt::Expr(e) => {
                    return self.parse_type_switch_body(start, None, None, e.expr);
                }
                Stmt::ShortVar(svd) if svd.names.len() == 1 && svd.values.len() == 1 => {
                    let binding = svd.names.into_iter().next().unwrap();
                    let expr = svd.values.into_iter().next().unwrap();
                    return self.parse_type_switch_body(start, None, Some(binding), expr);
                }
                _ => return Err(self.error("invalid type switch")),
            }
        }

        let (init, expr) = if self.eat(&TokenKind::Semi) {
            // init; expr form or init; { form
            if self.cur_is(&TokenKind::LBrace) {
                (Some(Box::new(first)), None)
            } else {
                let expr = self.parse_expr()?;

                // Check for type switch after init (.(type) consumed)
                if self.saw_type_switch && self.cur_is(&TokenKind::LBrace) {
                    self.saw_type_switch = false;
                    self.allow_composite_lit = saved;
                    return self.parse_type_switch_body(start, Some(Box::new(first)), None, expr);
                }

                (Some(Box::new(first)), Some(expr))
            }
        } else {
            // Just expr
            match first {
                Stmt::Expr(e) => (None, Some(e.expr)),
                _ => return Err(self.error("expected ';' after init statement")),
            }
        };

        self.allow_composite_lit = saved;
        self.parse_switch_body(start, init, expr)
    }

    fn parse_switch_body(
        &mut self,
        start: crate::token::Span,
        init: Option<Box<Stmt>>,
        expr: Option<Expr>,
    ) -> ParseResult<Stmt> {
        self.expect(&TokenKind::LBrace)?;

        let mut cases = Vec::new();
        let mut default = None;

        while !self.cur_is(&TokenKind::RBrace) && !self.at_eof() {
            if self.cur_is(&TokenKind::Case) {
                cases.push(self.parse_case_clause()?);
            } else if self.cur_is(&TokenKind::Default) {
                let def_start = self.current.span;
                self.next_token();
                self.expect(&TokenKind::Colon)?;

                let mut stmts = Vec::new();
                while !self.cur_is(&TokenKind::Case)
                    && !self.cur_is(&TokenKind::Default)
                    && !self.cur_is(&TokenKind::RBrace)
                    && !self.at_eof()
                {
                    stmts.push(self.parse_stmt()?);
                }

                let end = stmts.last().map(|s| s.span()).unwrap_or(def_start);
                default = Some(DefaultClause {
                    stmts,
                    span: def_start.to(&end),
                });
            } else {
                return Err(self.error("expected case or default"));
            }
        }

        let end = self.expect(&TokenKind::RBrace)?;

        Ok(Stmt::Switch(Box::new(SwitchStmt {
            init,
            expr,
            cases,
            default,
            span: start.to(&end),
        })))
    }

    fn parse_case_clause(&mut self) -> ParseResult<CaseClause> {
        let start = self.expect(&TokenKind::Case)?;
        let exprs = self.parse_expr_list()?;
        self.expect(&TokenKind::Colon)?;

        let mut stmts = Vec::new();
        while !self.cur_is(&TokenKind::Case)
            && !self.cur_is(&TokenKind::Default)
            && !self.cur_is(&TokenKind::RBrace)
            && !self.at_eof()
        {
            stmts.push(self.parse_stmt()?);
        }

        let end = stmts.last().map(|s| s.span()).unwrap_or(start);
        Ok(CaseClause {
            exprs,
            stmts,
            span: start.to(&end),
        })
    }

    fn parse_type_switch_body(
        &mut self,
        start: crate::token::Span,
        _init: Option<Box<Stmt>>,
        binding: Option<Ident>,
        expr: Expr,
    ) -> ParseResult<Stmt> {
        // .(type) was already consumed by expression parser
        self.expect(&TokenKind::LBrace)?;

        let mut cases = Vec::new();
        while !self.cur_is(&TokenKind::RBrace) && !self.at_eof() {
            cases.push(self.parse_type_case_clause()?);
        }

        let end = self.expect(&TokenKind::RBrace)?;

        Ok(Stmt::TypeSwitch(Box::new(TypeSwitchStmt {
            binding,
            expr,
            cases,
            span: start.to(&end),
        })))
    }

    fn parse_type_case_clause(&mut self) -> ParseResult<TypeCaseClause> {
        let start = self.current.span;

        let types = if self.eat(&TokenKind::Case) {
            let mut types = vec![self.parse_type_or_nil()?];
            while self.eat(&TokenKind::Comma) {
                types.push(self.parse_type_or_nil()?);
            }
            Some(types)
        } else if self.eat(&TokenKind::Default) {
            None
        } else {
            return Err(self.error("expected case or default"));
        };

        self.expect(&TokenKind::Colon)?;

        let mut stmts = Vec::new();
        while !self.cur_is(&TokenKind::Case)
            && !self.cur_is(&TokenKind::Default)
            && !self.cur_is(&TokenKind::RBrace)
            && !self.at_eof()
        {
            stmts.push(self.parse_stmt()?);
        }

        let end = stmts.last().map(|s| s.span()).unwrap_or(start);
        Ok(TypeCaseClause {
            types,
            stmts,
            span: start.to(&end),
        })
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Select Statement
    // ═══════════════════════════════════════════════════════════════════════

    fn parse_select_stmt(&mut self) -> ParseResult<Stmt> {
        let start = self.expect(&TokenKind::Select)?;
        self.expect(&TokenKind::LBrace)?;

        let mut cases = Vec::new();
        while !self.cur_is(&TokenKind::RBrace) && !self.at_eof() {
            cases.push(self.parse_select_case()?);
        }

        let end = self.expect(&TokenKind::RBrace)?;

        Ok(Stmt::Select(Box::new(SelectStmt {
            cases,
            span: start.to(&end),
        })))
    }

    fn parse_select_case(&mut self) -> ParseResult<SelectCase> {
        let start = self.current.span;

        let comm = if self.eat(&TokenKind::Case) {
            Some(self.parse_comm_clause()?)
        } else if self.eat(&TokenKind::Default) {
            None
        } else {
            return Err(self.error("expected case or default"));
        };

        self.expect(&TokenKind::Colon)?;

        let mut stmts = Vec::new();
        while !self.cur_is(&TokenKind::Case)
            && !self.cur_is(&TokenKind::Default)
            && !self.cur_is(&TokenKind::RBrace)
            && !self.at_eof()
        {
            stmts.push(self.parse_stmt()?);
        }

        let end = stmts.last().map(|s| s.span()).unwrap_or(start);
        Ok(SelectCase {
            comm,
            stmts,
            span: start.to(&end),
        })
    }

    fn parse_comm_clause(&mut self) -> ParseResult<CommClause> {
        // Could be send or receive
        let saved = self.allow_composite_lit;
        self.allow_composite_lit = false;

        let exprs = self.parse_expr_list()?;

        // Send: ch <- value
        if exprs.len() == 1 && self.cur_is(&TokenKind::Arrow) {
            self.next_token();
            let value = self.parse_expr()?;
            self.allow_composite_lit = saved;
            let span = exprs[0].span().to(&value.span());
            return Ok(CommClause::Send(SendStmt {
                chan: exprs.into_iter().next().unwrap(),
                value,
                span,
            }));
        }

        // Receive with assignment: v := <-ch or v = <-ch
        if self.cur_is(&TokenKind::ColonAssign) || self.cur_is(&TokenKind::Assign) {
            let is_define = self.cur_is(&TokenKind::ColonAssign);
            self.next_token();

            // Expect receive expression
            self.expect(&TokenKind::Arrow)?;
            let recv_expr = self.parse_expr()?;
            self.allow_composite_lit = saved;

            let mut vars = Vec::new();
            for expr in exprs {
                match expr {
                    Expr::Ident(id) => vars.push(id),
                    _ => return Err(self.error_at("expected identifier", expr.span())),
                }
            }

            let span = vars.first().unwrap().span.to(&recv_expr.span());
            return Ok(CommClause::Recv(RecvStmt {
                vars: Some(vars),
                is_define,
                expr: recv_expr,
                span,
            }));
        }

        // Bare receive: <-ch (already parsed as expression)
        self.allow_composite_lit = saved;
        if exprs.len() == 1 {
            if let Expr::Receive(recv) = exprs.into_iter().next().unwrap() {
                return Ok(CommClause::Recv(RecvStmt {
                    vars: None,
                    is_define: false,
                    expr: recv.chan,
                    span: recv.span,
                }));
            }
        }

        Err(self.error("expected send or receive operation"))
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Other Statements
    // ═══════════════════════════════════════════════════════════════════════

    fn parse_go_stmt(&mut self) -> ParseResult<Stmt> {
        let start = self.expect(&TokenKind::Go)?;
        let expr = self.parse_expr()?;
        self.expect_semi()?;

        Ok(Stmt::Go(GoStmt {
            span: start.to(&expr.span()),
            expr,
        }))
    }

    fn parse_defer_stmt(&mut self) -> ParseResult<Stmt> {
        let start = self.expect(&TokenKind::Defer)?;
        let expr = self.parse_expr()?;
        self.expect_semi()?;

        Ok(Stmt::Defer(DeferStmt {
            span: start.to(&expr.span()),
            expr,
        }))
    }

    fn parse_goto_stmt(&mut self) -> ParseResult<Stmt> {
        let start = self.expect(&TokenKind::Goto)?;
        let label = self.parse_ident()?;
        self.expect_semi()?;

        Ok(Stmt::Goto(GotoStmt {
            span: start.to(&label.span),
            label,
        }))
    }

    fn parse_break_stmt(&mut self) -> ParseResult<Stmt> {
        let start = self.expect(&TokenKind::Break)?;

        let label = if let TokenKind::Ident(_) = &self.current.kind {
            Some(self.parse_ident()?)
        } else {
            None
        };

        self.expect_semi()?;
        let end = label.as_ref().map(|l| l.span).unwrap_or(start);

        Ok(Stmt::Break(BreakStmt {
            label,
            span: start.to(&end),
        }))
    }

    fn parse_continue_stmt(&mut self) -> ParseResult<Stmt> {
        let start = self.expect(&TokenKind::Continue)?;

        let label = if let TokenKind::Ident(_) = &self.current.kind {
            Some(self.parse_ident()?)
        } else {
            None
        };

        self.expect_semi()?;
        let end = label.as_ref().map(|l| l.span).unwrap_or(start);

        Ok(Stmt::Continue(ContinueStmt {
            label,
            span: start.to(&end),
        }))
    }
}
