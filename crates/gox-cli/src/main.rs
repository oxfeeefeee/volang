//! GoX Compiler CLI
//!
//! Command-line interface for the GoX compiler.

use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "gox")]
#[command(author = "GoX Team")]
#[command(version = "0.1.0")]
#[command(about = "GoX Language Compiler", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse a GoX source file and display the AST
    Parse {
        /// Input source file (.gox)
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Display token stream instead of AST
        #[arg(short, long)]
        tokens: bool,

        /// Pretty print the output
        #[arg(short, long)]
        pretty: bool,
    },

    /// Check a GoX source file for errors
    Check {
        /// Input source file (.gox)
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },

    /// Display version information
    Version,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse {
            file,
            tokens,
            pretty,
        } => {
            cmd_parse(&file, tokens, pretty);
        }
        Commands::Check { file } => {
            cmd_check(&file);
        }
        Commands::Version => {
            println!("gox {}", env!("CARGO_PKG_VERSION"));
        }
    }
}

fn cmd_parse(file: &PathBuf, tokens: bool, pretty: bool) {
    // Read source file
    let source = match fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: could not read '{}': {}", file.display(), e);
            process::exit(1);
        }
    };

    if tokens {
        // Display token stream
        println!("=== Tokens for {} ===\n", file.display());
        let mut lexer = gox_syntax::lexer::Lexer::new(&source);
        let tokens_vec = lexer.tokenize();
        for (i, token) in tokens_vec.iter().enumerate() {
            if pretty {
                println!("{:4}: {:?} @ {:?}", i, token.kind, token.span);
            } else {
                println!("{:?}", token.kind);
            }
        }
    } else {
        // Parse and display AST
        match gox_syntax::parser::parse(&source) {
            Ok(ast) => {
                println!("=== AST for {} ===\n", file.display());

                // Package
                if let Some(pkg) = &ast.package {
                    println!("package: {}", pkg.name);
                }

                // Imports
                if !ast.imports.is_empty() {
                    println!("\nimports:");
                    for imp in &ast.imports {
                        println!("  \"{}\"", imp.path);
                    }
                }

                // Declarations
                println!("\ndeclarations: {}", ast.decls.len());
                for decl in &ast.decls {
                    print_decl(decl, pretty);
                }

                println!("\n✓ Parsed successfully");
            }
            Err(e) => {
                eprintln!("error: {}", e.message);
                let span = e.span;
                // Find line and column
                let (line, col) = find_line_col(&source, span.start);
                eprintln!("  --> {}:{}:{}", file.display(), line, col);

                // Show the problematic line
                let lines: Vec<&str> = source.lines().collect();
                if line > 0 && line <= lines.len() {
                    let code_line = lines[line - 1];
                    eprintln!("   |");
                    eprintln!("{:3}| {}", line, code_line);
                    eprintln!("   | {}^", " ".repeat(col - 1));
                }
                process::exit(1);
            }
        }
    }
}

fn cmd_check(file: &PathBuf) {
    // Read source file
    let source = match fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: could not read '{}': {}", file.display(), e);
            process::exit(1);
        }
    };

    // Parse
    match gox_syntax::parser::parse(&source) {
        Ok(ast) => {
            println!(
                "✓ {} parsed successfully ({} declarations)",
                file.display(),
                ast.decls.len()
            );
            // TODO: Add semantic analysis here
        }
        Err(e) => {
            eprintln!("error: {}", e.message);
            let span = e.span;
            let (line, col) = find_line_col(&source, span.start);
            eprintln!("  --> {}:{}:{}", file.display(), line, col);
            process::exit(1);
        }
    }
}

fn print_decl(decl: &gox_syntax::ast::TopDecl, pretty: bool) {
    use gox_syntax::ast::TopDecl;

    match decl {
        TopDecl::Var(v) => {
            for spec in &v.specs {
                let ty = spec
                    .ty
                    .as_ref()
                    .map(|t| format!(" {}", type_name(t)))
                    .unwrap_or_default();
                let names: Vec<_> = spec.names.iter().map(|n| n.name.as_str()).collect();
                println!("  var {}{}", names.join(", "), ty);
            }
        }
        TopDecl::Const(c) => {
            for spec in &c.specs {
                let names: Vec<_> = spec.names.iter().map(|n| n.name.as_str()).collect();
                println!("  const {}", names.join(", "));
            }
        }
        TopDecl::Type(t) => {
            println!("  type {} = {}", t.name.name, type_name(&t.ty));
        }
        TopDecl::Interface(i) => {
            println!("  interface {} ({} methods)", i.name.name, i.elements.len());
        }
        TopDecl::Implements(i) => {
            let ifaces: Vec<_> = i.interfaces.iter().map(|id| id.name.as_str()).collect();
            println!("  implements {} : {}", i.type_name.name, ifaces.join(", "));
        }
        TopDecl::Func(f) => {
            let receiver = f
                .receiver
                .as_ref()
                .map(|r| format!("({} {}) ", r.name.name, r.ty.name))
                .unwrap_or_default();
            let params: Vec<_> = f
                .params
                .iter()
                .map(|p| {
                    let names: Vec<_> = p.names.iter().map(|n| n.name.as_str()).collect();
                    format!("{} {}", names.join(", "), type_name(&p.ty))
                })
                .collect();
            let result = f
                .result
                .as_ref()
                .map(|r| format!(" {}", result_name(r)))
                .unwrap_or_default();
            println!(
                "  func {}{}({}){}",
                receiver,
                f.name.name,
                params.join(", "),
                result
            );
            if pretty {
                print_block(&f.body, 2);
            }
        }
    }
}

fn print_block(block: &gox_syntax::ast::Block, indent: usize) {
    for stmt in &block.stmts {
        print_stmt(stmt, indent);
    }
}

fn print_stmt(stmt: &gox_syntax::ast::Stmt, indent: usize) {
    use gox_syntax::ast::Stmt;
    let pad = "  ".repeat(indent);

    match stmt {
        Stmt::Block(b) => {
            println!("{}{{", pad);
            print_block(b, indent + 1);
            println!("{}}}", pad);
        }
        Stmt::Var(v) => {
            for spec in &v.specs {
                let ty = spec
                    .ty
                    .as_ref()
                    .map(|t| format!(" {}", type_name(t)))
                    .unwrap_or_default();
                let names: Vec<_> = spec.names.iter().map(|n| n.name.as_str()).collect();
                println!("{}var {}{}", pad, names.join(", "), ty);
            }
        }
        Stmt::Const(c) => {
            for spec in &c.specs {
                let names: Vec<_> = spec.names.iter().map(|n| n.name.as_str()).collect();
                println!("{}const {}", pad, names.join(", "));
            }
        }
        Stmt::ShortVar(s) => {
            let names: Vec<_> = s.names.iter().map(|n| n.name.as_str()).collect();
            println!("{}{} := ...", pad, names.join(", "));
        }
        Stmt::Assign(a) => {
            let op = match a.op {
                gox_syntax::ast::AssignOp::Assign => "=",
                gox_syntax::ast::AssignOp::PlusAssign => "+=",
                gox_syntax::ast::AssignOp::MinusAssign => "-=",
                gox_syntax::ast::AssignOp::StarAssign => "*=",
                gox_syntax::ast::AssignOp::SlashAssign => "/=",
                gox_syntax::ast::AssignOp::PercentAssign => "%=",
            };
            println!("{}... {} ...", pad, op);
        }
        Stmt::Expr(_) => {
            println!("{}expr;", pad);
        }
        Stmt::Return(r) => {
            if r.values.is_empty() {
                println!("{}return", pad);
            } else {
                println!("{}return ({} values)", pad, r.values.len());
            }
        }
        Stmt::If(i) => {
            println!("{}if ... {{", pad);
            print_block(&i.then_block, indent + 1);
            if let Some(else_clause) = &i.else_clause {
                match else_clause {
                    gox_syntax::ast::ElseClause::Block(b) => {
                        println!("{}}} else {{", pad);
                        print_block(b, indent + 1);
                    }
                    gox_syntax::ast::ElseClause::If(elif) => {
                        println!("{}}} else if ... {{", pad);
                        print_block(&elif.then_block, indent + 1);
                    }
                }
            }
            println!("{}}}", pad);
        }
        Stmt::For(f) => {
            let clause = match (&f.init, &f.cond, &f.post) {
                (None, None, None) => "".to_string(),
                (None, Some(_), None) => "cond".to_string(),
                _ => "init; cond; post".to_string(),
            };
            println!("{}for {} {{", pad, clause);
            print_block(&f.body, indent + 1);
            println!("{}}}", pad);
        }
        Stmt::ForRange(f) => {
            let vars = f
                .vars
                .as_ref()
                .map(|v| {
                    v.iter()
                        .map(|id| id.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let op = if f.is_define { ":=" } else { "=" };
            if vars.is_empty() {
                println!("{}for range ... {{", pad);
            } else {
                println!("{}for {} {} range ... {{", pad, vars, op);
            }
            print_block(&f.body, indent + 1);
            println!("{}}}", pad);
        }
        Stmt::Switch(s) => {
            println!("{}switch ... {{ ({} cases)", pad, s.cases.len());
            for case in &s.cases {
                println!("{}  case ({} exprs):", pad, case.exprs.len());
                for stmt in &case.stmts {
                    print_stmt(stmt, indent + 2);
                }
            }
            if let Some(def) = &s.default {
                println!("{}  default:", pad);
                for stmt in &def.stmts {
                    print_stmt(stmt, indent + 2);
                }
            }
            println!("{}}}", pad);
        }
        Stmt::TypeSwitch(ts) => {
            let binding = ts
                .binding
                .as_ref()
                .map(|b| format!("{} := ", b.name))
                .unwrap_or_default();
            println!(
                "{}switch {}....(type) {{ ({} cases)",
                pad,
                binding,
                ts.cases.len()
            );
            for case in &ts.cases {
                if let Some(types) = &case.types {
                    let type_names: Vec<_> = types.iter().map(type_or_nil_name).collect();
                    println!("{}  case {}:", pad, type_names.join(", "));
                } else {
                    println!("{}  default:", pad);
                }
                for stmt in &case.stmts {
                    print_stmt(stmt, indent + 2);
                }
            }
            println!("{}}}", pad);
        }
        Stmt::Select(s) => {
            println!("{}select {{ ({} cases)", pad, s.cases.len());
            for case in &s.cases {
                if case.comm.is_some() {
                    println!("{}  case <comm>:", pad);
                } else {
                    println!("{}  default:", pad);
                }
                for stmt in &case.stmts {
                    print_stmt(stmt, indent + 2);
                }
            }
            println!("{}}}", pad);
        }
        Stmt::Go(_) => {
            println!("{}go ...()", pad);
        }
        Stmt::Defer(_) => {
            println!("{}defer ...()", pad);
        }
        Stmt::Send(_) => {
            println!("{}ch <- value", pad);
        }
        Stmt::Goto(g) => {
            println!("{}goto {}", pad, g.label.name);
        }
        Stmt::Labeled(l) => {
            println!("{}{}:", pad, l.label.name);
            print_stmt(&l.stmt, indent);
        }
        Stmt::Fallthrough(_) => {
            println!("{}fallthrough", pad);
        }
        Stmt::Break(b) => {
            let label = b
                .label
                .as_ref()
                .map(|l| format!(" {}", l.name))
                .unwrap_or_default();
            println!("{}break{}", pad, label);
        }
        Stmt::Continue(c) => {
            let label = c
                .label
                .as_ref()
                .map(|l| format!(" {}", l.name))
                .unwrap_or_default();
            println!("{}continue{}", pad, label);
        }
        Stmt::Empty(_) => {
            // Skip empty statements
        }
    }
}

fn type_or_nil_name(ton: &gox_syntax::ast::TypeOrNil) -> String {
    use gox_syntax::ast::TypeOrNil;
    match ton {
        TypeOrNil::Type(ty) => type_name(ty),
        TypeOrNil::Nil(_) => "nil".to_string(),
    }
}

fn type_name(ty: &gox_syntax::ast::Type) -> String {
    use gox_syntax::ast::Type;

    match ty {
        Type::Named(id) => id.name.clone(),
        Type::Array(a) => format!("[{}]{}", a.len, type_name(&a.elem)),
        Type::Slice(s) => format!("[]{}", type_name(&s.elem)),
        Type::Map(m) => format!("map[{}]{}", type_name(&m.key), type_name(&m.value)),
        Type::Chan(c) => format!("chan {}", type_name(&c.elem)),
        Type::Func(f) => {
            let params: Vec<_> = f.params.iter().map(type_name).collect();
            let result = f
                .result
                .as_ref()
                .map(|r| format!(" {}", result_name(r)))
                .unwrap_or_default();
            format!("func({}){}", params.join(", "), result)
        }
        Type::Struct(s) => format!("struct{{{} fields}}", s.fields.len()),
    }
}

fn result_name(r: &gox_syntax::ast::ResultType) -> String {
    use gox_syntax::ast::ResultType;
    match r {
        ResultType::Single(ty) => type_name(ty),
        ResultType::Tuple(types, _) => {
            let names: Vec<_> = types.iter().map(type_name).collect();
            format!("({})", names.join(", "))
        }
    }
}

fn find_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;

    for (i, ch) in source.chars().enumerate() {
        if i == offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line, col)
}
