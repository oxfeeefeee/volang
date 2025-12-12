//! Declaration parsing for GoX.

use super::{ParseResult, Parser};
use crate::ast::{
    ArrayType, ChanType, ConstDecl, ConstSpec, FieldDecl, FuncDecl, FuncType, ImplementsDecl,
    InterfaceDecl, InterfaceElem, MapType, MethodSpec, Param, Receiver, ResultType, SliceType,
    StructType, TopDecl, Type, TypeDecl, TypeOrNil, VarDecl, VarSpec,
};
use crate::token::TokenKind;

impl<'a> Parser<'a> {
    // ═══════════════════════════════════════════════════════════════════════
    // Top-Level Declarations
    // ═══════════════════════════════════════════════════════════════════════

    pub(super) fn parse_top_decl(&mut self) -> ParseResult<TopDecl> {
        match &self.current.kind {
            TokenKind::Var => Ok(TopDecl::Var(self.parse_var_decl()?)),
            TokenKind::Const => Ok(TopDecl::Const(self.parse_const_decl()?)),
            TokenKind::Type => Ok(TopDecl::Type(self.parse_type_decl()?)),
            TokenKind::Interface => Ok(TopDecl::Interface(self.parse_interface_decl()?)),
            TokenKind::Implements => Ok(TopDecl::Implements(self.parse_implements_decl()?)),
            TokenKind::Func => Ok(TopDecl::Func(self.parse_func_decl()?)),
            _ => Err(self.error("expected declaration")),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Variable Declarations
    // ═══════════════════════════════════════════════════════════════════════

    pub(super) fn parse_var_decl(&mut self) -> ParseResult<VarDecl> {
        let start = self.expect(&TokenKind::Var)?;

        // Check for grouped declaration: var ( ... );
        if self.cur_is(&TokenKind::LParen) {
            self.next_token();
            let mut specs = Vec::new();
            while !self.cur_is(&TokenKind::RParen) && !self.at_eof() {
                specs.push(self.parse_var_spec()?);
                self.expect_semi()?;
            }
            let end = self.expect(&TokenKind::RParen)?;
            self.expect_semi()?;
            return Ok(VarDecl {
                specs,
                span: start.to(&end),
            });
        }

        // Single declaration
        let spec = self.parse_var_spec()?;
        self.expect_semi()?;
        let span = start.to(&spec.span);
        Ok(VarDecl {
            specs: vec![spec],
            span,
        })
    }

    fn parse_var_spec(&mut self) -> ParseResult<VarSpec> {
        let names = self.parse_ident_list()?;
        let start = names.first().unwrap().span;

        // Optional type
        let ty = if self.is_type_start() {
            Some(self.parse_type()?)
        } else {
            None
        };

        // Optional initializer
        let values = if self.eat(&TokenKind::Assign) {
            self.parse_expr_list()?
        } else {
            Vec::new()
        };

        let end = values
            .last()
            .map(|e| e.span())
            .or(ty.as_ref().map(|t| t.span()))
            .unwrap_or(names.last().unwrap().span);

        Ok(VarSpec {
            names,
            ty,
            values,
            span: start.to(&end),
        })
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Constant Declarations
    // ═══════════════════════════════════════════════════════════════════════

    pub(super) fn parse_const_decl(&mut self) -> ParseResult<ConstDecl> {
        let start = self.expect(&TokenKind::Const)?;

        // Check for grouped declaration: const ( ... );
        if self.cur_is(&TokenKind::LParen) {
            self.next_token();
            let mut specs = Vec::new();
            while !self.cur_is(&TokenKind::RParen) && !self.at_eof() {
                specs.push(self.parse_const_spec()?);
                self.expect_semi()?;
            }
            let end = self.expect(&TokenKind::RParen)?;
            self.expect_semi()?;
            return Ok(ConstDecl {
                specs,
                span: start.to(&end),
            });
        }

        // Single declaration
        let spec = self.parse_const_spec()?;
        self.expect_semi()?;
        let span = start.to(&spec.span);
        Ok(ConstDecl {
            specs: vec![spec],
            span,
        })
    }

    fn parse_const_spec(&mut self) -> ParseResult<ConstSpec> {
        let names = self.parse_ident_list()?;
        let start = names.first().unwrap().span;

        // Optional type
        let ty = if self.is_type_start() && !self.cur_is(&TokenKind::Assign) {
            Some(self.parse_type()?)
        } else {
            None
        };

        // Optional initializer (can be omitted for iota continuation)
        let values = if self.eat(&TokenKind::Assign) {
            self.parse_expr_list()?
        } else {
            Vec::new()
        };

        let end = values
            .last()
            .map(|e| e.span())
            .or(ty.as_ref().map(|t| t.span()))
            .unwrap_or(names.last().unwrap().span);

        Ok(ConstSpec {
            names,
            ty,
            values,
            span: start.to(&end),
        })
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Type Declarations
    // ═══════════════════════════════════════════════════════════════════════

    fn parse_type_decl(&mut self) -> ParseResult<TypeDecl> {
        let start = self.expect(&TokenKind::Type)?;
        let name = self.parse_ident()?;
        let ty = self.parse_type()?;
        self.expect_semi()?;

        Ok(TypeDecl {
            span: start.to(&ty.span()),
            name,
            ty,
        })
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Interface Declarations
    // ═══════════════════════════════════════════════════════════════════════

    fn parse_interface_decl(&mut self) -> ParseResult<InterfaceDecl> {
        let start = self.expect(&TokenKind::Interface)?;
        let name = self.parse_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut elements = Vec::new();
        while !self.cur_is(&TokenKind::RBrace) && !self.at_eof() {
            elements.push(self.parse_interface_elem()?);
        }

        let end = self.expect(&TokenKind::RBrace)?;
        self.expect_semi()?;

        Ok(InterfaceDecl {
            name,
            elements,
            span: start.to(&end),
        })
    }

    fn parse_interface_elem(&mut self) -> ParseResult<InterfaceElem> {
        let name = self.parse_ident()?;

        if self.cur_is(&TokenKind::LParen) {
            // Method: Name(params) Result?;
            let method_start = name.span;
            self.next_token();
            let params = self.parse_param_list()?;
            let rparen = self.expect(&TokenKind::RParen)?;
            let result = self.parse_optional_result()?;
            self.expect_semi()?;

            let end = result.as_ref().map(|r| r.span()).unwrap_or(rparen);
            Ok(InterfaceElem::Method(MethodSpec {
                name,
                params,
                result,
                span: method_start.to(&end),
            }))
        } else {
            // Embedded interface
            self.expect_semi()?;
            Ok(InterfaceElem::Embedded(name))
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Implements Declarations
    // ═══════════════════════════════════════════════════════════════════════

    fn parse_implements_decl(&mut self) -> ParseResult<ImplementsDecl> {
        let start = self.expect(&TokenKind::Implements)?;
        let type_name = self.parse_ident()?;
        self.expect(&TokenKind::Colon)?;
        let interfaces = self.parse_ident_list()?;
        self.expect_semi()?;

        let end = interfaces.last().unwrap().span;
        Ok(ImplementsDecl {
            type_name,
            interfaces,
            span: start.to(&end),
        })
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Function Declarations
    // ═══════════════════════════════════════════════════════════════════════

    fn parse_func_decl(&mut self) -> ParseResult<FuncDecl> {
        let start = self.expect(&TokenKind::Func)?;

        // Optional receiver
        let receiver = if self.cur_is(&TokenKind::LParen) {
            Some(self.parse_receiver()?)
        } else {
            None
        };

        let name = self.parse_ident()?;
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(&TokenKind::RParen)?;

        let result = self.parse_optional_result()?;
        let body = self.parse_block()?;

        Ok(FuncDecl {
            receiver,
            name,
            params,
            result,
            span: start.to(&body.span),
            body,
        })
    }

    fn parse_receiver(&mut self) -> ParseResult<Receiver> {
        let start = self.expect(&TokenKind::LParen)?;
        let name = self.parse_ident()?;
        let ty = self.parse_ident()?;
        let end = self.expect(&TokenKind::RParen)?;

        Ok(Receiver {
            name,
            ty,
            span: start.to(&end),
        })
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Parameters
    // ═══════════════════════════════════════════════════════════════════════

    pub(super) fn parse_param_list(&mut self) -> ParseResult<Vec<Param>> {
        let mut params = Vec::new();

        if self.cur_is(&TokenKind::RParen) {
            return Ok(params);
        }

        loop {
            // Collect names that share a type
            let mut names = vec![self.parse_ident()?];

            while self.cur_is(&TokenKind::Comma) && self.peek_is_ident() {
                self.next_token(); // consume comma
                let next_name = self.parse_ident()?;

                if self.cur_is(&TokenKind::Comma) {
                    // More names coming
                    names.push(next_name);
                } else {
                    // Type follows
                    names.push(next_name);
                    break;
                }
            }

            // Check for variadic
            let variadic = self.eat(&TokenKind::Ellipsis);

            // Parse type
            let ty = self.parse_type()?;

            let span = names.first().unwrap().span.to(&ty.span());
            params.push(Param {
                names,
                ty,
                variadic,
                span,
            });

            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }

        Ok(params)
    }

    fn peek_is_ident(&self) -> bool {
        matches!(&self.peek.kind, TokenKind::Ident(_))
    }

    pub(super) fn parse_optional_result(&mut self) -> ParseResult<Option<ResultType>> {
        if self.cur_is(&TokenKind::LBrace) || self.cur_is(&TokenKind::Semi) {
            return Ok(None);
        }

        // Tuple result: (Type, Type, ...)
        if self.cur_is(&TokenKind::LParen) {
            let start = self.current.span;
            self.next_token();
            let mut types = vec![self.parse_type()?];
            while self.eat(&TokenKind::Comma) {
                types.push(self.parse_type()?);
            }
            let end = self.expect(&TokenKind::RParen)?;
            return Ok(Some(ResultType::Tuple(types, start.to(&end))));
        }

        // Single type result
        let ty = self.parse_type()?;
        Ok(Some(ResultType::Single(ty)))
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Types
    // ═══════════════════════════════════════════════════════════════════════

    pub(super) fn parse_type(&mut self) -> ParseResult<Type> {
        match &self.current.kind {
            TokenKind::Ident(_) => {
                let id = self.parse_ident()?;
                Ok(Type::Named(id))
            }
            TokenKind::LBracket => self.parse_array_or_slice_type(),
            TokenKind::Map => self.parse_map_type(),
            TokenKind::Chan => self.parse_chan_type(),
            TokenKind::Func => self.parse_func_type(),
            TokenKind::Struct => self.parse_struct_type(),
            _ => Err(self.error("expected type")),
        }
    }

    fn parse_array_or_slice_type(&mut self) -> ParseResult<Type> {
        let start = self.expect(&TokenKind::LBracket)?;

        if self.cur_is(&TokenKind::RBracket) {
            // Slice type: []T
            self.next_token();
            let elem = self.parse_type()?;
            return Ok(Type::Slice(Box::new(SliceType {
                span: start.to(&elem.span()),
                elem,
            })));
        }

        // Array type: [N]T
        let len = match &self.current.kind {
            TokenKind::Int(n) => *n,
            _ => return Err(self.error("expected array length")),
        };
        self.next_token();
        self.expect(&TokenKind::RBracket)?;
        let elem = self.parse_type()?;

        Ok(Type::Array(Box::new(ArrayType {
            len,
            span: start.to(&elem.span()),
            elem,
        })))
    }

    fn parse_map_type(&mut self) -> ParseResult<Type> {
        let start = self.expect(&TokenKind::Map)?;
        self.expect(&TokenKind::LBracket)?;
        let key = self.parse_type()?;
        self.expect(&TokenKind::RBracket)?;
        let value = self.parse_type()?;

        let end = value.span();
        Ok(Type::Map(Box::new(MapType {
            key,
            value,
            span: start.to(&end),
        })))
    }

    fn parse_chan_type(&mut self) -> ParseResult<Type> {
        let start = self.expect(&TokenKind::Chan)?;
        let elem = self.parse_type()?;

        Ok(Type::Chan(Box::new(ChanType {
            span: start.to(&elem.span()),
            elem,
        })))
    }

    fn parse_func_type(&mut self) -> ParseResult<Type> {
        let start = self.expect(&TokenKind::Func)?;
        self.expect(&TokenKind::LParen)?;

        let mut params = Vec::new();
        if !self.cur_is(&TokenKind::RParen) {
            params.push(self.parse_type()?);
            while self.eat(&TokenKind::Comma) {
                params.push(self.parse_type()?);
            }
        }
        let rparen = self.expect(&TokenKind::RParen)?;

        let result = self.parse_optional_result()?;
        let end = result.as_ref().map(|r| r.span()).unwrap_or(rparen);

        Ok(Type::Func(Box::new(FuncType {
            params,
            result: result.map(Box::new),
            span: start.to(&end),
        })))
    }

    fn parse_struct_type(&mut self) -> ParseResult<Type> {
        let start = self.expect(&TokenKind::Struct)?;
        self.expect(&TokenKind::LBrace)?;

        let mut fields = Vec::new();
        while !self.cur_is(&TokenKind::RBrace) && !self.at_eof() {
            fields.push(self.parse_field_decl()?);
        }

        let end = self.expect(&TokenKind::RBrace)?;

        Ok(Type::Struct(Box::new(StructType {
            fields,
            span: start.to(&end),
        })))
    }

    pub(super) fn parse_field_decl(&mut self) -> ParseResult<FieldDecl> {
        // Could be: names Type tag? ; or just Type ; (anonymous field)
        let first = self.parse_ident()?;

        if self.is_type_start() {
            // Named field(s)
            let mut names = vec![first];
            while self.eat(&TokenKind::Comma) {
                names.push(self.parse_ident()?);
            }
            let ty = self.parse_type()?;

            // Optional tag
            let tag = if let TokenKind::String(s) = &self.current.kind {
                let t = s.clone();
                self.next_token();
                Some(t)
            } else {
                None
            };

            self.expect_semi()?;

            let end = tag.as_ref().map(|_| self.current.span).unwrap_or(ty.span());

            let start_span = names.first().unwrap().span;
            Ok(FieldDecl {
                names,
                ty,
                tag,
                span: start_span.to(&end),
            })
        } else {
            // Anonymous field (embedded type)
            let tag = if let TokenKind::String(s) = &self.current.kind {
                let t = s.clone();
                self.next_token();
                Some(t)
            } else {
                None
            };

            self.expect_semi()?;

            Ok(FieldDecl {
                names: Vec::new(),
                ty: Type::Named(first.clone()),
                tag,
                span: first.span,
            })
        }
    }

    pub(super) fn parse_type_or_nil(&mut self) -> ParseResult<TypeOrNil> {
        if self.cur_is(&TokenKind::Nil) {
            let span = self.current.span;
            self.next_token();
            Ok(TypeOrNil::Nil(span))
        } else {
            Ok(TypeOrNil::Type(self.parse_type()?))
        }
    }
}
