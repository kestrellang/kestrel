//! Pretty-printer for AstBody — renders the arena-based AST as an
//! indented tree for debugging and visualization.

use crate::arena::Arena;
use crate::ast_body::*;
use crate::ast_type::AstType;
use std::fmt::Write;

/// Pretty-print an AstBody as an indented tree.
pub fn pretty_print(body: &AstBody) -> String {
    let mut ctx = PrettyCtx {
        exprs: &body.exprs,
        pats: &body.pats,
        stmts: &body.stmts,
        buf: String::new(),
    };

    for (i, &sid) in body.statements.iter().enumerate() {
        if i > 0 {
            ctx.buf.push('\n');
        }
        ctx.print_stmt(sid, 0);
    }
    if let Some(tail) = body.tail_expr {
        if !body.statements.is_empty() {
            ctx.buf.push('\n');
        }
        ctx.indent(0);
        let _ = write!(ctx.buf, "tail: ");
        ctx.print_expr(tail, 0);
    }

    ctx.buf
}

struct PrettyCtx<'a> {
    exprs: &'a Arena<AstExpr>,
    pats: &'a Arena<AstPat>,
    stmts: &'a Arena<AstStmt>,
    buf: String,
}

impl PrettyCtx<'_> {
    fn indent(&mut self, depth: usize) {
        for _ in 0..depth {
            self.buf.push_str("  ");
        }
    }

    fn print_stmt(&mut self, id: StmtId, depth: usize) {
        let stmt = &self.stmts[id];
        match stmt {
            AstStmt::Let {
                is_mut,
                pattern,
                ty,
                value,
                ..
            } => {
                self.indent(depth);
                let keyword = if *is_mut { "var" } else { "let" };
                let _ = write!(self.buf, "{keyword} ");
                self.print_pat(*pattern);
                if let Some(ty) = ty {
                    let _ = write!(self.buf, ": {}", format_type(ty));
                }
                if let Some(val) = value {
                    self.buf.push_str(" = ");
                    self.print_expr(*val, depth);
                }
                self.buf.push('\n');
            },
            AstStmt::Expr { expr, .. } => {
                self.indent(depth);
                self.print_expr(*expr, depth);
                self.buf.push('\n');
            },
            AstStmt::GuardLet {
                conditions,
                else_body,
                ..
            } => {
                self.indent(depth);
                self.buf.push_str("guard ");
                for (i, cond) in conditions.iter().enumerate() {
                    if i > 0 {
                        self.buf.push_str(", ");
                    }
                    self.print_condition(cond, depth);
                }
                self.buf.push_str(" else {\n");
                self.print_block(else_body, depth + 1);
                self.indent(depth);
                self.buf.push_str("}\n");
            },
            AstStmt::Deinit { name, .. } => {
                self.indent(depth);
                let _ = writeln!(self.buf, "deinit {name}");
            },
        }
    }

    fn print_expr(&mut self, id: ExprId, depth: usize) {
        let expr = &self.exprs[id];
        match expr {
            AstExpr::Literal { kind, .. } => {
                self.buf.push_str(&format_literal(kind));
            },
            AstExpr::InterpolatedString { parts, .. } => {
                self.buf.push('"');
                for part in parts {
                    match part {
                        StringPart::Literal(s) => self.buf.push_str(s),
                        StringPart::Interpolation { expr, .. } => {
                            self.buf.push_str("\\(");
                            self.print_expr(*expr, depth);
                            self.buf.push(')');
                        },
                    }
                }
                self.buf.push('"');
            },
            AstExpr::Array { elements, .. } => {
                self.buf.push('[');
                for (i, &e) in elements.iter().enumerate() {
                    if i > 0 {
                        self.buf.push_str(", ");
                    }
                    self.print_expr(e, depth);
                }
                self.buf.push(']');
            },
            AstExpr::Dictionary { entries, .. } => {
                self.buf.push('[');
                for (i, entry) in entries.iter().enumerate() {
                    if i > 0 {
                        self.buf.push_str(", ");
                    }
                    self.print_expr(entry.key, depth);
                    self.buf.push_str(": ");
                    self.print_expr(entry.value, depth);
                }
                self.buf.push(']');
            },
            AstExpr::Tuple { elements, .. } => {
                self.buf.push('(');
                for (i, &e) in elements.iter().enumerate() {
                    if i > 0 {
                        self.buf.push_str(", ");
                    }
                    self.print_expr(e, depth);
                }
                self.buf.push(')');
            },
            AstExpr::Path { segments, .. } => {
                for (i, seg) in segments.iter().enumerate() {
                    if i > 0 {
                        self.buf.push('.');
                    }
                    self.buf.push_str(&seg.name);
                    if let Some(args) = &seg.type_args {
                        self.buf.push('[');
                        for (j, arg) in args.iter().enumerate() {
                            if j > 0 {
                                self.buf.push_str(", ");
                            }
                            self.buf.push_str(&format_type(arg));
                        }
                        self.buf.push(']');
                    }
                }
            },
            AstExpr::MemberAccess {
                base,
                member,
                type_args,
                ..
            } => {
                self.print_expr(*base, depth);
                self.buf.push('.');
                self.buf.push_str(member);
                if let Some(args) = type_args {
                    self.buf.push('[');
                    for (j, arg) in args.iter().enumerate() {
                        if j > 0 {
                            self.buf.push_str(", ");
                        }
                        self.buf.push_str(&format_type(arg));
                    }
                    self.buf.push(']');
                }
            },
            AstExpr::TupleIndex { base, index, .. } => {
                self.print_expr(*base, depth);
                let _ = write!(self.buf, ".{index}");
            },
            AstExpr::ImplicitMember {
                member, arguments, ..
            } => {
                let _ = write!(self.buf, ".{member}");
                if let Some(args) = arguments {
                    self.buf.push('(');
                    self.print_call_args(args, depth);
                    self.buf.push(')');
                }
            },
            AstExpr::Unary { op, operand, .. } => {
                self.buf.push_str(format_unary_op(op));
                self.print_expr(*operand, depth);
            },
            AstExpr::Postfix { operand, op, .. } => {
                self.print_expr(*operand, depth);
                self.buf.push_str(format_postfix_op(op));
            },
            AstExpr::Binary { lhs, op, rhs, .. } => {
                self.buf.push('(');
                self.print_expr(*lhs, depth);
                let _ = write!(self.buf, " {} ", format_binary_op(op));
                self.print_expr(*rhs, depth);
                self.buf.push(')');
            },
            AstExpr::Assignment { lhs, rhs, .. } => {
                self.print_expr(*lhs, depth);
                self.buf.push_str(" = ");
                self.print_expr(*rhs, depth);
            },
            AstExpr::CompoundAssignment { lhs, op, rhs, .. } => {
                self.print_expr(*lhs, depth);
                let _ = write!(self.buf, " {} ", format_compound_op(op));
                self.print_expr(*rhs, depth);
            },
            AstExpr::Call {
                callee, arguments, ..
            } => {
                self.print_expr(*callee, depth);
                self.buf.push('(');
                self.print_call_args(arguments, depth);
                self.buf.push(')');
            },
            AstExpr::If {
                conditions,
                then_body,
                else_body,
                ..
            } => {
                self.buf.push_str("if ");
                for (i, cond) in conditions.iter().enumerate() {
                    if i > 0 {
                        self.buf.push_str(", ");
                    }
                    self.print_condition(cond, depth);
                }
                self.buf.push_str(" {\n");
                self.print_block(then_body, depth + 1);
                self.indent(depth);
                self.buf.push('}');
                if let Some(else_b) = else_body {
                    match else_b {
                        ElseBody::Block(block) => {
                            self.buf.push_str(" else {\n");
                            self.print_block(block, depth + 1);
                            self.indent(depth);
                            self.buf.push('}');
                        },
                        ElseBody::ElseIf(expr) => {
                            self.buf.push_str(" else ");
                            self.print_expr(*expr, depth);
                        },
                    }
                }
            },
            AstExpr::While {
                label,
                condition,
                body,
                ..
            } => {
                if let Some(l) = label {
                    let _ = write!(self.buf, "{l}: ");
                }
                self.buf.push_str("while ");
                self.print_expr(*condition, depth);
                self.buf.push_str(" {\n");
                self.print_block(body, depth + 1);
                self.indent(depth);
                self.buf.push('}');
            },
            AstExpr::WhileLet {
                label,
                conditions,
                body,
                ..
            } => {
                if let Some(l) = label {
                    let _ = write!(self.buf, "{l}: ");
                }
                self.buf.push_str("while ");
                for (i, cond) in conditions.iter().enumerate() {
                    if i > 0 {
                        self.buf.push_str(", ");
                    }
                    self.print_condition(cond, depth);
                }
                self.buf.push_str(" {\n");
                self.print_block(body, depth + 1);
                self.indent(depth);
                self.buf.push('}');
            },
            AstExpr::Loop { label, body, .. } => {
                if let Some(l) = label {
                    let _ = write!(self.buf, "{l}: ");
                }
                self.buf.push_str("loop {\n");
                self.print_block(body, depth + 1);
                self.indent(depth);
                self.buf.push('}');
            },
            AstExpr::For {
                label,
                pattern,
                iterable,
                body,
                ..
            } => {
                if let Some(l) = label {
                    let _ = write!(self.buf, "{l}: ");
                }
                self.buf.push_str("for ");
                self.print_pat(*pattern);
                self.buf.push_str(" in ");
                self.print_expr(*iterable, depth);
                self.buf.push_str(" {\n");
                self.print_block(body, depth + 1);
                self.indent(depth);
                self.buf.push('}');
            },
            AstExpr::Break { label, .. } => {
                self.buf.push_str("break");
                if let Some(l) = label {
                    let _ = write!(self.buf, " {l}");
                }
            },
            AstExpr::Continue { label, .. } => {
                self.buf.push_str("continue");
                if let Some(l) = label {
                    let _ = write!(self.buf, " {l}");
                }
            },
            AstExpr::Return { value, .. } => {
                self.buf.push_str("return");
                if let Some(v) = value {
                    self.buf.push(' ');
                    self.print_expr(*v, depth);
                }
            },
            AstExpr::Throw { value, .. } => {
                self.buf.push_str("throw ");
                self.print_expr(*value, depth);
            },
            AstExpr::Try { operand, .. } => {
                self.buf.push_str("try ");
                self.print_expr(*operand, depth);
            },
            AstExpr::Closure { params, body, .. } => {
                self.buf.push_str("{ ");
                if !params.is_empty() {
                    self.buf.push('(');
                    for (i, p) in params.iter().enumerate() {
                        if i > 0 {
                            self.buf.push_str(", ");
                        }
                        self.print_pat(p.pattern);
                        if let Some(ty) = &p.ty {
                            let _ = write!(self.buf, ": {}", format_type(ty));
                        }
                    }
                    self.buf.push_str(") in\n");
                }
                self.print_block(body, depth + 1);
                self.indent(depth);
                self.buf.push('}');
            },
            AstExpr::Match {
                scrutinee, arms, ..
            } => {
                self.buf.push_str("match ");
                self.print_expr(*scrutinee, depth);
                self.buf.push_str(" {\n");
                for arm in arms {
                    self.indent(depth + 1);
                    self.print_pat(arm.pattern);
                    if let Some(g) = arm.guard {
                        self.buf.push_str(" if ");
                        self.print_expr(g, depth + 1);
                    }
                    self.buf.push_str(" => ");
                    self.print_expr(arm.body, depth + 1);
                    self.buf.push('\n');
                }
                self.indent(depth);
                self.buf.push('}');
            },
            AstExpr::Block { body, .. } => {
                self.buf.push_str("{\n");
                for &stmt in &body.stmts {
                    self.indent(depth + 1);
                    self.print_stmt(stmt, depth + 1);
                    self.buf.push('\n');
                }
                if let Some(tail) = body.tail_expr {
                    self.indent(depth + 1);
                    self.print_expr(tail, depth + 1);
                    self.buf.push('\n');
                }
                self.indent(depth);
                self.buf.push('}');
            },
            AstExpr::Paren { inner, .. } => {
                self.buf.push('(');
                self.print_expr(*inner, depth);
                self.buf.push(')');
            },
            AstExpr::Error { .. } => {
                self.buf.push_str("<error>");
            },
        }
    }

    fn print_pat(&mut self, id: PatId) {
        let pat = &self.pats[id];
        match pat {
            AstPat::Wildcard { .. } => self.buf.push('_'),
            AstPat::Binding { is_mut, name, .. } => {
                if *is_mut {
                    self.buf.push_str("var ");
                }
                self.buf.push_str(name);
            },
            AstPat::Tuple {
                prefix,
                has_rest,
                suffix,
                ..
            } => {
                self.buf.push('(');
                for (i, &e) in prefix.iter().enumerate() {
                    if i > 0 {
                        self.buf.push_str(", ");
                    }
                    self.print_pat(e);
                }
                if *has_rest {
                    if !prefix.is_empty() {
                        self.buf.push_str(", ");
                    }
                    self.buf.push_str("..");
                    for &e in suffix {
                        self.buf.push_str(", ");
                        self.print_pat(e);
                    }
                }
                self.buf.push(')');
            },
            AstPat::Literal { kind, .. } => {
                self.buf.push_str(&format_lit_pat(kind));
            },
            AstPat::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                if let Some(s) = start {
                    self.buf.push_str(&format_lit_pat(s));
                }
                self.buf.push_str(if *inclusive { "..=" } else { "..<" });
                if let Some(e) = end {
                    self.buf.push_str(&format_lit_pat(e));
                }
            },
            AstPat::Enum {
                case_name, args, ..
            } => {
                let _ = write!(self.buf, ".{case_name}");
                if !args.is_empty() {
                    self.buf.push('(');
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            self.buf.push_str(", ");
                        }
                        if let Some(label) = &arg.label {
                            let _ = write!(self.buf, "{label}: ");
                        }
                        self.print_pat(arg.pattern);
                    }
                    self.buf.push(')');
                }
            },
            AstPat::Struct {
                name,
                fields,
                has_rest,
                ..
            } => {
                let _ = write!(self.buf, "{name} {{ ");
                for (i, f) in fields.iter().enumerate() {
                    if i > 0 {
                        self.buf.push_str(", ");
                    }
                    self.buf.push_str(&f.field_name);
                    if let Some(p) = f.pattern {
                        self.buf.push_str(": ");
                        self.print_pat(p);
                    }
                }
                if *has_rest {
                    if !fields.is_empty() {
                        self.buf.push_str(", ");
                    }
                    self.buf.push_str("..");
                }
                self.buf.push_str(" }");
            },
            AstPat::Array {
                prefix,
                rest,
                suffix,
                ..
            } => {
                self.buf.push('[');
                let mut first = true;
                for &p in prefix {
                    if !first {
                        self.buf.push_str(", ");
                    }
                    first = false;
                    self.print_pat(p);
                }
                if let Some(r) = rest {
                    if !first {
                        self.buf.push_str(", ");
                    }
                    first = false;
                    self.buf.push_str("..");
                    if let Some(name) = r {
                        self.buf.push_str(name);
                    }
                }
                for &p in suffix {
                    if !first {
                        self.buf.push_str(", ");
                    }
                    first = false;
                    self.print_pat(p);
                }
                self.buf.push(']');
            },
            AstPat::At {
                is_mut,
                name,
                subpattern,
                ..
            } => {
                if *is_mut {
                    self.buf.push_str("var ");
                }
                let _ = write!(self.buf, "{name} @ ");
                self.print_pat(*subpattern);
            },
            AstPat::Or { alternatives, .. } => {
                for (i, &a) in alternatives.iter().enumerate() {
                    if i > 0 {
                        self.buf.push_str(" | ");
                    }
                    self.print_pat(a);
                }
            },
            AstPat::Rest { .. } => self.buf.push_str(".."),
            AstPat::Error { .. } => self.buf.push_str("<error>"),
        }
    }

    fn print_block(&mut self, block: &AstBlock, depth: usize) {
        for &sid in &block.stmts {
            self.print_stmt(sid, depth);
        }
        if let Some(tail) = block.tail_expr {
            self.indent(depth);
            self.print_expr(tail, depth);
            self.buf.push('\n');
        }
    }

    fn print_call_args(&mut self, args: &[CallArg], depth: usize) {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.buf.push_str(", ");
            }
            if let Some(label) = &arg.label {
                let _ = write!(self.buf, "{label}: ");
            }
            self.print_expr(arg.value, depth);
        }
    }

    fn print_condition(&mut self, cond: &IfCondition, depth: usize) {
        match cond {
            IfCondition::Expr(e) => self.print_expr(*e, depth),
            IfCondition::Let { pattern, value } => {
                self.buf.push_str("let ");
                self.print_pat(*pattern);
                self.buf.push_str(" = ");
                self.print_expr(*value, depth);
            },
        }
    }
}

fn format_literal(lit: &AstLiteral) -> String {
    match lit {
        AstLiteral::Integer(s) => s.clone(),
        AstLiteral::Float(s) => s.clone(),
        // String / RawString / Char carry the full source token text
        // (including delimiters), so we emit them verbatim.
        AstLiteral::String(s) => s.clone(),
        AstLiteral::RawString(s) => s.clone(),
        AstLiteral::Char(s) => s.clone(),
        AstLiteral::Bool(b) => b.to_string(),
        AstLiteral::Null => "null".into(),
        AstLiteral::Unit => "()".into(),
    }
}

fn format_lit_pat(lit: &LitPatKind) -> String {
    match lit {
        LitPatKind::Integer(s) => s.clone(),
        LitPatKind::Float(s) => s.clone(),
        LitPatKind::String(s) => format!("\"{s}\""),
        LitPatKind::Bool(b) => b.to_string(),
        LitPatKind::Char(s) => format!("'{s}'"),
    }
}

fn format_type(ty: &AstType) -> String {
    match ty {
        AstType::Named { segments, .. } => segments
            .iter()
            .map(|s| {
                if s.type_args.is_empty() {
                    s.name.clone()
                } else {
                    let args: Vec<_> = s.type_args.iter().map(format_type).collect();
                    format!("{}[{}]", s.name, args.join(", "))
                }
            })
            .collect::<Vec<_>>()
            .join("."),
        AstType::Tuple(elems, _) => {
            let inner: Vec<_> = elems.iter().map(format_type).collect();
            format!("({})", inner.join(", "))
        },
        AstType::Function {
            params,
            return_type,
            ..
        } => {
            let p: Vec<_> = params.iter().map(format_type).collect();
            format!("({}) -> {}", p.join(", "), format_type(return_type))
        },
        AstType::Array(inner, _) => format!("[{}]", format_type(inner)),
        AstType::Dictionary(k, v, _) => format!("[{}: {}]", format_type(k), format_type(v)),
        AstType::Optional(inner, _) => format!("{}?", format_type(inner)),
        AstType::Result { ok, err, .. } => {
            format!("{} throws {}", format_type(ok), format_type(err))
        },
        AstType::Unit(_) => "()".into(),
        AstType::Never(_) => "Never".into(),
        AstType::Inferred(_) => "_".into(),
    }
}

fn format_unary_op(op: &UnaryOp) -> &'static str {
    match op {
        UnaryOp::Neg => "-",
        UnaryOp::BitNot => "!",
        UnaryOp::LogicalNot => "not ",
        UnaryOp::Pos => "+",
    }
}

fn format_postfix_op(op: &PostfixOp) -> &'static str {
    match op {
        PostfixOp::Unwrap => "!",
    }
}

fn format_binary_op(op: &BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Rem => "%",
        BinaryOp::BitAnd => "&",
        BinaryOp::BitOr => "|",
        BinaryOp::BitXor => "^",
        BinaryOp::Shl => "<<",
        BinaryOp::Shr => ">>",
        BinaryOp::Eq => "==",
        BinaryOp::Ne => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Gt => ">",
        BinaryOp::Le => "<=",
        BinaryOp::Ge => ">=",
        BinaryOp::And => "and",
        BinaryOp::Or => "or",
        BinaryOp::Coalesce => "??",
        BinaryOp::RangeInclusive => "..=",
        BinaryOp::RangeExclusive => "..<",
    }
}

fn format_compound_op(op: &CompoundAssignOp) -> &'static str {
    match op {
        CompoundAssignOp::AddAssign => "+=",
        CompoundAssignOp::SubAssign => "-=",
        CompoundAssignOp::MulAssign => "*=",
        CompoundAssignOp::DivAssign => "/=",
        CompoundAssignOp::RemAssign => "%=",
        CompoundAssignOp::BitAndAssign => "&=",
        CompoundAssignOp::BitOrAssign => "|=",
        CompoundAssignOp::BitXorAssign => "^=",
        CompoundAssignOp::ShlAssign => "<<=",
        CompoundAssignOp::ShrAssign => ">>=",
    }
}
