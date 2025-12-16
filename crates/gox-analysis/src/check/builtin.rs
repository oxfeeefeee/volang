//! Built-in function type checking.
//!
//! This module handles type checking for Go's built-in functions:
//! - len, cap: Collection length/capacity
//! - append, copy: Slice operations
//! - make, delete, close: Collection management  
//! - panic, recover: Error handling
//! - print, println: Output
//! - assert: Testing (GoX extension)

use gox_common::Span;
use gox_syntax::ast::Expr;

use crate::errors::TypeError;
use crate::scope::BuiltinKind;
use crate::types::{BasicType, Type};

use super::TypeChecker;

impl<'a> TypeChecker<'a> {
    /// Checks a built-in function call.
    pub(crate) fn check_builtin_call(&mut self, kind: BuiltinKind, args: &[Expr], span: Span) -> Type {
        match kind {
            BuiltinKind::Len => self.check_len(args, span),
            BuiltinKind::Cap => self.check_cap(args, span),
            BuiltinKind::Append => self.check_append(args, span),
            BuiltinKind::Copy => self.check_copy(args, span),
            BuiltinKind::Delete => self.check_delete(args, span),
            BuiltinKind::Make => self.check_make(args, span),
            BuiltinKind::Close => self.check_close(args, span),
            BuiltinKind::Panic => self.check_panic(args, span),
            BuiltinKind::Recover => self.check_recover(args, span),
            BuiltinKind::Print | BuiltinKind::Println => self.check_print(args, span),
            BuiltinKind::Assert => self.check_assert(args, span),
        }
    }
    
    /// Checks assert(cond, args...) - no return value.
    fn check_assert(&mut self, args: &[Expr], span: Span) -> Type {
        if args.is_empty() {
            self.error(TypeError::PanicArgCount, span); // Reuse panic error for now
            return Type::Tuple(vec![]);
        }
        
        // First argument must be boolean (just check it's a valid expression)
        self.check_expr(&args[0]);
        
        // Remaining arguments can be any type (like println)
        for arg in args.iter().skip(1) {
            self.check_expr(arg);
        }
        
        Type::Tuple(vec![])
    }

    /// Checks len(x) - returns int.
    fn check_len(&mut self, args: &[Expr], span: Span) -> Type {
        if args.len() != 1 {
            self.error(TypeError::LenArgCount, span);
            return Type::Invalid;
        }

        let arg_ty = self.check_expr(&args[0]);
        match self.underlying_type(&arg_ty) {
            Type::Array(_) | Type::Slice(_) | Type::Map(_) | Type::Chan(_)
            | Type::Basic(BasicType::String) => Type::Basic(BasicType::Int),
            _ => {
                self.error(TypeError::LenArgType, span);
                Type::Invalid
            }
        }
    }

    /// Checks cap(x) - returns int.
    fn check_cap(&mut self, args: &[Expr], span: Span) -> Type {
        if args.len() != 1 {
            self.error(TypeError::CapArgCount, span);
            return Type::Invalid;
        }

        let arg_ty = self.check_expr(&args[0]);
        match self.underlying_type(&arg_ty) {
            Type::Array(_) | Type::Slice(_) | Type::Chan(_) => Type::Basic(BasicType::Int),
            _ => {
                self.error(TypeError::CapArgType, span);
                Type::Invalid
            }
        }
    }

    /// Checks append(slice, elems...) - returns slice type.
    fn check_append(&mut self, args: &[Expr], span: Span) -> Type {
        if args.is_empty() {
            self.error(TypeError::AppendArgCount, span);
            return Type::Invalid;
        }

        let slice_ty = self.check_expr(&args[0]);
        match self.underlying_type(&slice_ty) {
            Type::Slice(s) => {
                // Check remaining args are assignable to element type
                for arg in &args[1..] {
                    let arg_ty = self.check_expr(arg);
                    if !self.is_assignable(&arg_ty, &s.elem) {
                        self.error(TypeError::ElementTypeMismatch, arg.span);
                    }
                }
                slice_ty.clone()
            }
            _ => {
                self.error(TypeError::AppendArgType, span);
                Type::Invalid
            }
        }
    }

    /// Checks copy(dst, src) - returns int.
    fn check_copy(&mut self, args: &[Expr], span: Span) -> Type {
        if args.len() != 2 {
            self.error(TypeError::CopyArgCount, span);
            return Type::Invalid;
        }

        let dst_ty = self.check_expr(&args[0]);
        let src_ty = self.check_expr(&args[1]);

        let dst_elem = match self.underlying_type(&dst_ty) {
            Type::Slice(s) => Some((*s.elem).clone()),
            _ => None,
        };

        let src_elem = match self.underlying_type(&src_ty) {
            Type::Slice(s) => Some((*s.elem).clone()),
            Type::Basic(BasicType::String) => Some(Type::Basic(BasicType::Uint8)),
            _ => None,
        };

        match (dst_elem, src_elem) {
            (Some(d), Some(s)) if d == s => Type::Basic(BasicType::Int),
            _ => {
                self.error(TypeError::CopyArgType, span);
                Type::Invalid
            }
        }
    }

    /// Checks delete(map, key).
    fn check_delete(&mut self, args: &[Expr], span: Span) -> Type {
        if args.len() != 2 {
            self.error(TypeError::DeleteArgCount, span);
            return Type::Tuple(vec![]);
        }

        let map_ty = self.check_expr(&args[0]);
        let key_ty = self.check_expr(&args[1]);

        match self.underlying_type(&map_ty) {
            Type::Map(m) => {
                if !self.is_assignable(&key_ty, &m.key) {
                    self.error(TypeError::KeyTypeMismatch, args[1].span);
                }
            }
            _ => {
                self.error(TypeError::DeleteArgType, span);
            }
        }

        Type::Tuple(vec![])
    }

    /// Checks make(type, args...).
    fn check_make(&mut self, args: &[Expr], span: Span) -> Type {
        if args.is_empty() {
            self.error(TypeError::MakeArgCount, span);
            return Type::Invalid;
        }

        // First argument should be a type - for now just check the expression
        // In a full implementation, we'd parse the type from the first arg
        let ty = self.check_expr(&args[0]);

        // Check optional length/capacity arguments are integers
        for arg in &args[1..] {
            let arg_ty = self.check_expr(arg);
            if !self.is_integer_type(&arg_ty) {
                self.error(TypeError::IndexNotInteger, arg.span);
            }
        }

        // Return the type being made
        ty
    }

    /// Checks close(chan).
    fn check_close(&mut self, args: &[Expr], span: Span) -> Type {
        if args.len() != 1 {
            self.error(TypeError::CloseArgCount, span);
            return Type::Tuple(vec![]);
        }

        let chan_ty = self.check_expr(&args[0]);
        match self.underlying_type(&chan_ty) {
            Type::Chan(c) => {
                if c.dir == crate::types::ChanDir::RecvOnly {
                    self.error(TypeError::ReceiveFromSendOnly, span);
                }
            }
            _ => {
                self.error(TypeError::CloseArgType, span);
            }
        }

        Type::Tuple(vec![])
    }

    /// Checks panic(v).
    fn check_panic(&mut self, args: &[Expr], span: Span) -> Type {
        if args.len() != 1 {
            self.error(TypeError::PanicArgCount, span);
        } else {
            self.check_expr(&args[0]);
        }
        Type::Tuple(vec![])
    }

    /// Checks recover() - returns interface{}.
    fn check_recover(&mut self, args: &[Expr], span: Span) -> Type {
        if !args.is_empty() {
            self.error(TypeError::RecoverArgCount, span);
        }
        // Returns interface{}
        Type::Interface(crate::types::InterfaceType {
            methods: vec![],
            embeds: vec![],
        })
    }

    /// Checks print/println(args...).
    fn check_print(&mut self, args: &[Expr], _span: Span) -> Type {
        // Just check all arguments are valid expressions
        for arg in args {
            self.check_expr(arg);
        }
        Type::Tuple(vec![])
    }
}

