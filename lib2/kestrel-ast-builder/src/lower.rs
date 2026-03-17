//! CST-to-AST lowering: converts CST expression/statement/pattern nodes into
//! arena-based `AstBody` representation.
//!
//! The resulting AST is unresolved — paths are just names, no symbols, no types.
//! Grouping parens are dropped, for-loops are NOT desugared.

use kestrel_span2::Span;
use kestrel_syntax_tree2::utils::{find_child, get_node_span, is_trivia};
use kestrel_syntax_tree2::{SyntaxKind, SyntaxNode};
use kestrel_ast::arena::Arena;
use kestrel_ast::ast_body::*;
use kestrel_ast::AstType;
use crate::ast_type::ast_type_from_cst;
use crate::builders::helpers::is_type_kind;

/// Lower a CodeBlock CST node into an AstBody.
pub fn lower_body(code_block: &SyntaxNode, file_id: usize) -> AstBody {
    let mut ctx = LowerCtx::new(file_id);
    let block = ctx.lower_block(code_block);
    AstBody {
        exprs: ctx.exprs,
        pats: ctx.pats,
        stmts: ctx.stmts,
        statements: block.stmts,
        tail_expr: block.tail_expr,
    }
}

/// Lower a DefaultValue CST node (single expression) into an AstBody.
///
/// The DefaultValue node wraps `= expression` — we extract the expression
/// child and store it as the tail expression of the body.
pub fn lower_default_value(node: &SyntaxNode, file_id: usize) -> AstBody {
    let mut ctx = LowerCtx::new(file_id);

    // DefaultValue contains: Equals token + Expression child
    let tail = node
        .children()
        .find(|c| is_expr_kind(c.kind()))
        .map(|c| ctx.lower_expr(&c));

    AstBody {
        exprs: ctx.exprs,
        pats: ctx.pats,
        stmts: ctx.stmts,
        statements: Vec::new(),
        tail_expr: tail,
    }
}

/// Lower a bare expression node (not wrapped) into an AstBody.
///
/// Used for field initializers where the parser emits `= Expression` directly
/// under FieldDeclaration without a DefaultValue wrapper.
pub fn lower_default_value_expr(node: &SyntaxNode, file_id: usize) -> AstBody {
    let mut ctx = LowerCtx::new(file_id);
    let tail = ctx.lower_expr(node);

    AstBody {
        exprs: ctx.exprs,
        pats: ctx.pats,
        stmts: ctx.stmts,
        statements: Vec::new(),
        tail_expr: Some(tail),
    }
}

// ===== Lowering context =====

struct LowerCtx {
    exprs: Arena<AstExpr>,
    pats: Arena<AstPat>,
    stmts: Arena<AstStmt>,
    file_id: usize,
}

impl LowerCtx {
    fn new(file_id: usize) -> Self {
        Self {
            exprs: Arena::new(),
            pats: Arena::new(),
            stmts: Arena::new(),
            file_id,
        }
    }

    fn span(&self, node: &SyntaxNode) -> Span {
        get_node_span(node, self.file_id)
    }

    fn alloc_expr(&mut self, expr: AstExpr) -> ExprId {
        self.exprs.alloc(expr)
    }

    fn alloc_pat(&mut self, pat: AstPat) -> PatId {
        self.pats.alloc(pat)
    }

    fn alloc_stmt(&mut self, stmt: AstStmt) -> StmtId {
        self.stmts.alloc(stmt)
    }

    // ===== Block lowering =====

    /// Lower a CodeBlock: `{ stmt; stmt; [tail_expr] }`
    fn lower_block(&mut self, node: &SyntaxNode) -> AstBlock {
        let mut stmts = Vec::new();
        let mut tail_expr = None;

        // Collect children with their CST nodes for later analysis
        let children: Vec<_> = node.children().collect();
        let child_count = children.len();

        for (i, child) in children.iter().enumerate() {
            match child.kind() {
                // Statement wrapper nodes contain the actual statement
                SyntaxKind::Statement => {
                    // If this is the last child and it's a bare ExpressionStatement
                    // (statement-like expr without semicolon), promote to tail expr.
                    // This handles `if/else`, `match`, etc. at the end of a block.
                    let is_last = i == child_count - 1
                        || children[i + 1..].iter().all(|c| c.kind() == SyntaxKind::RBrace);
                    if is_last && tail_expr.is_none() {
                        if let Some(inner) = child.children().next() {
                            if inner.kind() == SyntaxKind::ExpressionStatement
                                && !has_semicolon(&inner)
                            {
                                // Promote to tail expression
                                let expr_id = self.lower_expr_stmt_as_expr(&inner);
                                tail_expr = Some(expr_id);
                                continue;
                            }
                        }
                    }
                    if let Some(inner) = child.children().next() {
                        let stmt_id = self.lower_stmt(&inner);
                        stmts.push(stmt_id);
                    }
                }
                // Bare expression at end of block = tail expression
                SyntaxKind::Expression => {
                    let expr_id = self.lower_expr(&child);
                    tail_expr = Some(expr_id);
                }
                _ if is_expr_kind(child.kind()) => {
                    let expr_id = self.lower_expr(&child);
                    tail_expr = Some(expr_id);
                }
                _ => {}
            }
        }

        AstBlock { stmts, tail_expr }
    }

    // ===== Statement lowering =====

    fn lower_stmt(&mut self, node: &SyntaxNode) -> StmtId {
        match node.kind() {
            SyntaxKind::VariableDeclaration => self.lower_variable_decl(node),
            SyntaxKind::ExpressionStatement => self.lower_expr_stmt(node),
            SyntaxKind::GuardLetStatement => self.lower_guard_let(node),
            SyntaxKind::DeinitStatement => self.lower_deinit_stmt(node),
            _ => {
                // Fallback: wrap as expression statement if it looks like an expr
                if is_expr_kind(node.kind()) {
                    let expr_id = self.lower_expr(node);
                    let span = self.span(node);
                    self.alloc_stmt(AstStmt::Expr { expr: expr_id, span })
                } else {
                    // Unknown statement kind — emit error expression
                    let span = self.span(node);
                    let err = self.alloc_expr(AstExpr::Error { span: span.clone() });
                    self.alloc_stmt(AstStmt::Expr { expr: err, span })
                }
            }
        }
    }

    /// `let/var pattern [: Type] [= expr];`
    fn lower_variable_decl(&mut self, node: &SyntaxNode) -> StmtId {
        let span = self.span(node);

        // Check for var keyword (mutable)
        let is_mut = node
            .children_with_tokens()
            .any(|e| e.as_token().is_some_and(|t| t.kind() == SyntaxKind::Var));

        // Pattern child
        let pattern = node
            .children()
            .find(|c| c.kind() == SyntaxKind::Pattern || is_pattern_kind(c.kind()))
            .map(|c| self.lower_pat(&c))
            .unwrap_or_else(|| self.alloc_pat(AstPat::Error { span: span.clone() }));

        // Optional type annotation — find a type node
        let ty = node
            .children()
            .find(|c| is_type_kind(c.kind()))
            .and_then(|c| ast_type_from_cst(&c, self.file_id));

        // Optional initializer — direct Expression child after Equals token.
        // (VariableDeclaration uses `= expr` directly, NOT wrapped in DefaultValue.)
        let value = {
            let mut found_equals = false;
            let mut result = None;
            for child in node.children_with_tokens() {
                if child.as_token().is_some_and(|t| t.kind() == SyntaxKind::Equals) {
                    found_equals = true;
                } else if found_equals {
                    if let Some(expr_node) = child.into_node() {
                        if expr_node.kind() == SyntaxKind::Expression || is_expr_kind(expr_node.kind()) {
                            result = Some(self.lower_expr(&expr_node));
                            break;
                        }
                    }
                }
            }
            result
        };

        self.alloc_stmt(AstStmt::Let {
            is_mut,
            pattern,
            ty,
            value,
            span,
        })
    }

    /// `expression;`
    /// Extract the expression from an ExpressionStatement, used when
    /// promoting the last expression statement to a tail expression.
    fn lower_expr_stmt_as_expr(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);
        node.children()
            .find(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()))
            .map(|c| self.lower_expr(&c))
            .unwrap_or_else(|| self.alloc_expr(AstExpr::Error { span }))
    }

    fn lower_expr_stmt(&mut self, node: &SyntaxNode) -> StmtId {
        let span = self.span(node);

        let expr = node
            .children()
            .find(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()))
            .map(|c| self.lower_expr(&c))
            .unwrap_or_else(|| self.alloc_expr(AstExpr::Error { span: span.clone() }));

        self.alloc_stmt(AstStmt::Expr { expr, span })
    }

    /// `guard let pattern = expr [, let pattern = expr] else { block }`
    fn lower_guard_let(&mut self, node: &SyntaxNode) -> StmtId {
        let span = self.span(node);

        let conditions = self.lower_let_conditions(node, SyntaxKind::GuardLetCondition);

        // Else block — find CodeBlock child
        let else_body = node
            .children()
            .find(|c| c.kind() == SyntaxKind::CodeBlock)
            .map(|c| self.lower_block(&c))
            .unwrap_or_else(|| AstBlock {
                stmts: Vec::new(),
                tail_expr: None,
            });

        self.alloc_stmt(AstStmt::GuardLet {
            conditions,
            else_body,
            span,
        })
    }

    /// `deinit identifier;`
    fn lower_deinit_stmt(&mut self, node: &SyntaxNode) -> StmtId {
        let span = self.span(node);

        let name = node
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::Identifier)
            .map(|t| t.text().to_string())
            .unwrap_or_default();

        self.alloc_stmt(AstStmt::Deinit { name, span })
    }

    // ===== Expression lowering =====

    fn lower_expr(&mut self, node: &SyntaxNode) -> ExprId {
        // Unwrap Expression wrapper nodes
        let node = unwrap_expr(node);

        match node.kind() {
            // Literals
            SyntaxKind::ExprInteger => self.lower_literal(&node, |text| AstLiteral::Integer(text)),
            SyntaxKind::ExprFloat => self.lower_literal(&node, |text| AstLiteral::Float(text)),
            SyntaxKind::ExprString => self.lower_literal(&node, |text| AstLiteral::String(text)),
            SyntaxKind::ExprRawString => {
                self.lower_literal(&node, |text| AstLiteral::RawString(text))
            }
            SyntaxKind::ExprChar => self.lower_literal(&node, |text| AstLiteral::Char(text)),
            SyntaxKind::ExprBool => {
                let span = self.span(&node);
                let text = first_token_text(&node);
                let val = text.as_deref() == Some("true");
                self.alloc_expr(AstExpr::Literal {
                    kind: AstLiteral::Bool(val),
                    span,
                })
            }
            SyntaxKind::ExprNull => {
                let span = self.span(&node);
                self.alloc_expr(AstExpr::Literal {
                    kind: AstLiteral::Null,
                    span,
                })
            }
            SyntaxKind::ExprUnit => {
                let span = self.span(&node);
                self.alloc_expr(AstExpr::Literal {
                    kind: AstLiteral::Unit,
                    span,
                })
            }
            SyntaxKind::ExprInterpolatedString => self.lower_interpolated_string(&node),

            // Collections
            SyntaxKind::ExprArray => self.lower_array(&node),
            SyntaxKind::ExprDictionary => self.lower_dictionary(&node),
            SyntaxKind::ExprTuple => self.lower_tuple(&node),

            // Grouping — transparent
            SyntaxKind::ExprGrouping => {
                let inner = node
                    .children()
                    .find(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()));
                match inner {
                    Some(c) => self.lower_expr(&c),
                    None => {
                        let span = self.span(&node);
                        self.alloc_expr(AstExpr::Error { span })
                    }
                }
            }

            // Path / member access
            SyntaxKind::ExprPath => self.lower_path(&node),
            SyntaxKind::ExprTupleIndex => self.lower_tuple_index(&node),
            SyntaxKind::ExprImplicitMemberAccess => self.lower_implicit_member(&node),

            // Operators
            SyntaxKind::ExprUnary => self.lower_unary(&node),
            SyntaxKind::ExprPostfix => self.lower_postfix(&node),
            SyntaxKind::ExprBinary => self.lower_binary(&node),
            SyntaxKind::ExprAssignment => self.lower_assignment(&node),
            SyntaxKind::ExprCompoundAssignment => self.lower_compound_assignment(&node),

            // Call
            SyntaxKind::ExprCall => self.lower_call(&node),

            // Control flow
            SyntaxKind::ExprIf => self.lower_if(&node),
            SyntaxKind::ExprWhile => self.lower_while(&node),
            SyntaxKind::ExprFor => self.lower_for(&node),
            SyntaxKind::ExprLoop => self.lower_loop(&node),
            SyntaxKind::ExprBreak => self.lower_break(&node),
            SyntaxKind::ExprContinue => self.lower_continue(&node),
            SyntaxKind::ExprReturn => self.lower_return(&node),
            SyntaxKind::ExprThrow => self.lower_throw(&node),
            SyntaxKind::ExprTry => self.lower_try(&node),

            // Closure / Match
            SyntaxKind::ExprClosure => self.lower_closure(&node),
            SyntaxKind::ExprMatch => self.lower_match(&node),

            _ => {
                let span = self.span(&node);
                self.alloc_expr(AstExpr::Error { span })
            }
        }
    }

    // ----- Literals -----

    fn lower_literal(&mut self, node: &SyntaxNode, mk: impl FnOnce(String) -> AstLiteral) -> ExprId {
        let span = self.span(node);
        let text = first_token_text(node).unwrap_or_default();
        self.alloc_expr(AstExpr::Literal {
            kind: mk(text),
            span,
        })
    }

    // ----- Interpolated String -----

    fn lower_interpolated_string(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);
        let mut parts = Vec::new();

        for child in node.children() {
            match child.kind() {
                SyntaxKind::StringLiteralPart => {
                    let text = child
                        .children_with_tokens()
                        .filter_map(|e| e.into_token())
                        .map(|t| t.text().to_string())
                        .collect::<Vec<_>>()
                        .join("");
                    parts.push(StringPart::Literal(text));
                }
                SyntaxKind::StringInterpolation => {
                    // Contains an expression child, optional FormatSpecifier
                    let expr = child
                        .children()
                        .find(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()))
                        .map(|c| self.lower_expr(&c))
                        .unwrap_or_else(|| {
                            self.alloc_expr(AstExpr::Error {
                                span: self.span(&child),
                            })
                        });

                    let format = find_child(&child, SyntaxKind::FormatSpecifier).map(|fs| {
                        fs.children_with_tokens()
                            .filter_map(|e| e.into_token())
                            .map(|t| t.text().to_string())
                            .collect::<Vec<_>>()
                            .join("")
                    });

                    parts.push(StringPart::Interpolation { expr, format });
                }
                _ => {}
            }
        }

        // If no structured children, the interpolated string is a raw token —
        // treat entire text as a literal part
        if parts.is_empty() {
            if let Some(text) = first_token_text(node) {
                parts.push(StringPart::Literal(text));
            }
        }

        self.alloc_expr(AstExpr::InterpolatedString { parts, span })
    }

    // ----- Collections -----

    fn lower_array(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);
        let elements = self.lower_expr_list(node);
        self.alloc_expr(AstExpr::Array { elements, span })
    }

    fn lower_dictionary(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);
        let mut entries = Vec::new();

        for child in node.children() {
            if child.kind() == SyntaxKind::DictionaryEntry {
                // DictionaryEntry has two Expression children: key and value
                let mut exprs = child
                    .children()
                    .filter(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()));

                if let (Some(key_node), Some(val_node)) = (exprs.next(), exprs.next()) {
                    let key = self.lower_expr(&key_node);
                    let value = self.lower_expr(&val_node);
                    entries.push(DictEntry { key, value });
                }
            }
        }

        self.alloc_expr(AstExpr::Dictionary { entries, span })
    }

    fn lower_tuple(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);
        let elements = self.lower_expr_list(node);
        self.alloc_expr(AstExpr::Tuple { elements, span })
    }

    /// Collect expression children (skipping brackets, commas, etc.)
    fn lower_expr_list(&mut self, node: &SyntaxNode) -> Vec<ExprId> {
        node.children()
            .filter(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()))
            .map(|c| self.lower_expr(&c))
            .collect()
    }

    // ----- Path / Member Access -----

    /// ExprPath: either a pure path (all identifier tokens) or member access
    /// (has a child Expression node as base).
    fn lower_path(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);

        // Check if first non-trivia child is an expression node (member access)
        // vs an identifier token (pure path)
        let first_non_trivia = node
            .children_with_tokens()
            .find(|e| !e.as_token().is_some_and(|t| is_trivia(t.kind())));

        let has_expr_base = first_non_trivia
            .as_ref()
            .is_some_and(|e| e.as_node().is_some_and(|n| is_expr_kind(n.kind()) || n.kind() == SyntaxKind::Expression));

        if has_expr_base {
            // Member access chain: lower base, then walk Dot+Identifier pairs
            self.lower_member_access_chain(node, span)
        } else {
            // Pure path: collect Identifier tokens and TypeArgumentLists
            self.lower_pure_path(node, span)
        }
    }

    /// Pure path: `a.b.c` or `Foo[Int].bar` — all segments are identifier tokens.
    fn lower_pure_path(&mut self, node: &SyntaxNode, span: Span) -> ExprId {
        let mut segments = Vec::new();
        let mut skip_next = false;

        let elements: Vec<_> = node.children_with_tokens().collect();
        for (i, elem) in elements.iter().enumerate() {
            if skip_next {
                skip_next = false;
                continue;
            }

            if let Some(token) = elem.as_token() {
                if token.kind() == SyntaxKind::Identifier {
                    let name = token.text().to_string();
                    let tok_span = Span::new(self.file_id, token.text_range().into());

                    // Check for type arguments following this identifier
                    let type_args = elements.get(i + 1)
                        .and_then(|next| next.as_node())
                        .filter(|n| n.kind() == SyntaxKind::TypeArgumentList)
                        .map(|n| {
                            skip_next = true;
                            extract_type_args(n, self.file_id)
                        });

                    segments.push(ExprPathSegment {
                        name,
                        type_args,
                        span: tok_span,
                    });
                }
            }
        }

        self.alloc_expr(AstExpr::Path { segments, span })
    }

    /// Member access chain: `expr.foo.bar` — first child is an expression node.
    fn lower_member_access_chain(&mut self, node: &SyntaxNode, span: Span) -> ExprId {
        // Lower the base expression (first child node)
        let base_node = node
            .children()
            .find(|c| is_expr_kind(c.kind()) || c.kind() == SyntaxKind::Expression)
            .unwrap();
        let mut current = self.lower_expr(&base_node);

        // Collect all children into a vec to avoid borrow issues
        let elements: Vec<_> = node.children_with_tokens().collect();

        // Find where the base expression ends
        let mut start_idx = 0;
        for (i, elem) in elements.iter().enumerate() {
            if elem.as_node().is_some_and(|n| {
                is_expr_kind(n.kind()) || n.kind() == SyntaxKind::Expression
            }) {
                start_idx = i + 1;
                break;
            }
        }

        // Walk remaining elements looking for Dot + Identifier [+ TypeArgumentList]
        let mut i = start_idx;
        while i < elements.len() {
            if let Some(token) = elements[i].as_token() {
                if token.kind() == SyntaxKind::Dot {
                    i += 1;
                    // Next should be an Identifier
                    if i < elements.len() {
                        if let Some(id_token) = elements[i].as_token() {
                            if id_token.kind() == SyntaxKind::Identifier {
                                let member = id_token.text().to_string();
                                i += 1;

                                // Check for type arguments
                                let type_args = elements.get(i)
                                    .and_then(|e| e.as_node())
                                    .filter(|n| n.kind() == SyntaxKind::TypeArgumentList)
                                    .map(|n| {
                                        i += 1;
                                        extract_type_args(n, self.file_id)
                                    });

                                current = self.alloc_expr(AstExpr::MemberAccess {
                                    base: current,
                                    member,
                                    type_args,
                                    span: span.clone(),
                                });
                                continue;
                            }
                        }
                    }
                }
            }
            i += 1;
        }

        current
    }

    fn lower_tuple_index(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);

        // First child: base expression
        let base = node
            .children()
            .find(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()))
            .map(|c| self.lower_expr(&c))
            .unwrap_or_else(|| self.alloc_expr(AstExpr::Error { span: span.clone() }));

        // Integer token for the index
        let index = node
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::Integer)
            .and_then(|t| t.text().parse::<u32>().ok())
            .unwrap_or(0);

        self.alloc_expr(AstExpr::TupleIndex { base, index, span })
    }

    fn lower_implicit_member(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);

        // Name child contains the identifier
        let member = find_child(node, SyntaxKind::Name)
            .and_then(|n| {
                n.children_with_tokens()
                    .filter_map(|e| e.into_token())
                    .find(|t| t.kind() == SyntaxKind::Identifier)
                    .map(|t| t.text().to_string())
            })
            .or_else(|| {
                // Fallback: look for direct identifier token
                node.children_with_tokens()
                    .filter_map(|e| e.into_token())
                    .find(|t| t.kind() == SyntaxKind::Identifier)
                    .map(|t| t.text().to_string())
            })
            .unwrap_or_default();

        // Optional arguments
        let arguments = find_child(node, SyntaxKind::ArgumentList).map(|al| self.lower_arguments(&al));

        self.alloc_expr(AstExpr::ImplicitMember {
            member,
            arguments,
            span,
        })
    }

    // ----- Operators -----

    fn lower_unary(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);

        // Operator token comes first
        let op = node
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| !is_trivia(t.kind()))
            .and_then(|t| token_to_unary_op(t.kind()))
            .unwrap_or(UnaryOp::Neg);

        // Operand expression
        let operand = node
            .children()
            .find(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()))
            .map(|c| self.lower_expr(&c))
            .unwrap_or_else(|| self.alloc_expr(AstExpr::Error { span: span.clone() }));

        self.alloc_expr(AstExpr::Unary { op, operand, span })
    }

    fn lower_postfix(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);

        let operand = node
            .children()
            .find(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()))
            .map(|c| self.lower_expr(&c))
            .unwrap_or_else(|| self.alloc_expr(AstExpr::Error { span: span.clone() }));

        // For now, only Unwrap (!) postfix operator
        self.alloc_expr(AstExpr::Postfix {
            operand,
            op: PostfixOp::Unwrap,
            span,
        })
    }

    fn lower_binary(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);

        // Two expression children: lhs and rhs
        let mut exprs = node
            .children()
            .filter(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()));

        let lhs = exprs
            .next()
            .map(|c| self.lower_expr(&c))
            .unwrap_or_else(|| self.alloc_expr(AstExpr::Error { span: span.clone() }));

        // Operator token between them
        let op = node
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find_map(|t| token_to_binary_op(t.kind()))
            .unwrap_or(BinaryOp::Add);

        let rhs = exprs
            .next()
            .map(|c| self.lower_expr(&c))
            .unwrap_or_else(|| self.alloc_expr(AstExpr::Error { span: span.clone() }));

        self.alloc_expr(AstExpr::Binary {
            lhs,
            op,
            rhs,
            span,
        })
    }

    fn lower_assignment(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);
        let mut exprs = node
            .children()
            .filter(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()));

        let lhs = exprs
            .next()
            .map(|c| self.lower_expr(&c))
            .unwrap_or_else(|| self.alloc_expr(AstExpr::Error { span: span.clone() }));
        let rhs = exprs
            .next()
            .map(|c| self.lower_expr(&c))
            .unwrap_or_else(|| self.alloc_expr(AstExpr::Error { span: span.clone() }));

        self.alloc_expr(AstExpr::Assignment { lhs, rhs, span })
    }

    fn lower_compound_assignment(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);
        let mut exprs = node
            .children()
            .filter(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()));

        let lhs = exprs
            .next()
            .map(|c| self.lower_expr(&c))
            .unwrap_or_else(|| self.alloc_expr(AstExpr::Error { span: span.clone() }));

        let op = node
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find_map(|t| token_to_compound_assign_op(t.kind()))
            .unwrap_or(CompoundAssignOp::AddAssign);

        let rhs = exprs
            .next()
            .map(|c| self.lower_expr(&c))
            .unwrap_or_else(|| self.alloc_expr(AstExpr::Error { span: span.clone() }));

        self.alloc_expr(AstExpr::CompoundAssignment {
            lhs,
            op,
            rhs,
            span,
        })
    }

    // ----- Call -----

    fn lower_call(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);

        // First child expression is the callee
        let callee = node
            .children()
            .find(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()))
            .map(|c| self.lower_expr(&c))
            .unwrap_or_else(|| self.alloc_expr(AstExpr::Error { span: span.clone() }));

        // ArgumentList child
        let arguments = find_child(node, SyntaxKind::ArgumentList)
            .map(|al| self.lower_arguments(&al))
            .unwrap_or_default();

        self.alloc_expr(AstExpr::Call {
            callee,
            arguments,
            span,
        })
    }

    /// Lower an ArgumentList node into a Vec<CallArg>.
    fn lower_arguments(&mut self, node: &SyntaxNode) -> Vec<CallArg> {
        let mut args = Vec::new();

        for child in node.children() {
            if child.kind() == SyntaxKind::Argument {
                // Argument: [Identifier Colon] Expression
                let label = self.extract_arg_label(&child);

                let value = child
                    .children()
                    .find(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()))
                    .map(|c| self.lower_expr(&c))
                    .unwrap_or_else(|| {
                        self.alloc_expr(AstExpr::Error {
                            span: self.span(&child),
                        })
                    });

                args.push(CallArg { label, value });
            }
        }

        args
    }

    /// Extract a label from an Argument node: if there's an Identifier followed
    /// by a Colon, the identifier is the label.
    fn extract_arg_label(&self, node: &SyntaxNode) -> Option<String> {
        let mut iter = node
            .children_with_tokens()
            .filter(|e| !e.as_token().is_some_and(|t| is_trivia(t.kind())));

        let first = iter.next()?;
        let second = iter.next();

        // Label exists if: first is Identifier, second is Colon
        if first.as_token().is_some_and(|t| t.kind() == SyntaxKind::Identifier)
            && second
                .as_ref()
                .is_some_and(|s| s.as_token().is_some_and(|t| t.kind() == SyntaxKind::Colon))
        {
            Some(first.as_token().unwrap().text().to_string())
        } else {
            None
        }
    }

    // ----- Control flow -----

    fn lower_if(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);

        // Conditions: mix of bare expressions and IfLetCondition nodes
        let conditions = self.lower_if_conditions(node);

        // Then body: first CodeBlock child
        let then_body = node
            .children()
            .find(|c| c.kind() == SyntaxKind::CodeBlock)
            .map(|c| self.lower_block(&c))
            .unwrap_or_else(|| AstBlock {
                stmts: Vec::new(),
                tail_expr: None,
            });

        // Else clause
        let else_body = find_child(node, SyntaxKind::ElseClause).map(|ec| {
            // ElseClause contains either a CodeBlock or a nested ExprIf
            if let Some(code_block) = find_child(&ec, SyntaxKind::CodeBlock) {
                ElseBody::Block(self.lower_block(&code_block))
            } else if let Some(if_expr) = ec
                .children()
                .find(|c| c.kind() == SyntaxKind::ExprIf || c.kind() == SyntaxKind::Expression)
            {
                ElseBody::ElseIf(self.lower_expr(&if_expr))
            } else {
                ElseBody::Block(AstBlock {
                    stmts: Vec::new(),
                    tail_expr: None,
                })
            }
        });

        self.alloc_expr(AstExpr::If {
            conditions,
            then_body,
            else_body,
            span,
        })
    }

    /// Lower if-conditions: mix of IfLetCondition and bare expressions before
    /// the first CodeBlock.
    fn lower_if_conditions(&mut self, node: &SyntaxNode) -> Vec<IfCondition> {
        let mut conditions = Vec::new();

        for child in node.children() {
            match child.kind() {
                SyntaxKind::IfLetCondition => {
                    // let pattern = expr
                    let (pat, val) = self.lower_let_condition_parts(&child);
                    conditions.push(IfCondition::Let {
                        pattern: pat,
                        value: val,
                    });
                }
                // Bare expression condition (before the CodeBlock)
                SyntaxKind::Expression if conditions.is_empty() || !has_sibling_code_block_before(node, &child) => {
                    // Only treat as condition if it appears before any CodeBlock
                    if appears_before_code_block(node, &child) {
                        let expr = self.lower_expr(&child);
                        conditions.push(IfCondition::Expr(expr));
                    }
                }
                SyntaxKind::CodeBlock => break,
                _ => {}
            }
        }

        // If no explicit conditions found, look for a direct expression condition
        if conditions.is_empty() {
            // The condition might be a direct expression child (not wrapped in IfLetCondition)
            if let Some(expr_node) = node
                .children()
                .find(|c| (c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind())) && c.kind() != SyntaxKind::CodeBlock)
            {
                if appears_before_code_block(node, &expr_node) {
                    let expr = self.lower_expr(&expr_node);
                    conditions.push(IfCondition::Expr(expr));
                }
            }
        }

        conditions
    }

    fn lower_while(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);
        let label = extract_loop_label(node);

        // Check for WhileLetCondition children — if present, it's a while-let
        let has_let_conditions = node
            .children()
            .any(|c| c.kind() == SyntaxKind::WhileLetCondition);

        if has_let_conditions {
            let conditions = self.lower_let_conditions(node, SyntaxKind::WhileLetCondition);
            let body = node
                .children()
                .find(|c| c.kind() == SyntaxKind::CodeBlock)
                .map(|c| self.lower_block(&c))
                .unwrap_or_else(|| AstBlock {
                    stmts: Vec::new(),
                    tail_expr: None,
                });

            self.alloc_expr(AstExpr::WhileLet {
                label,
                conditions,
                body,
                span,
            })
        } else {
            // Simple while: condition expression + body
            let condition = node
                .children()
                .find(|c| {
                    (c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()))
                        && c.kind() != SyntaxKind::CodeBlock
                })
                .map(|c| self.lower_expr(&c))
                .unwrap_or_else(|| self.alloc_expr(AstExpr::Error { span: span.clone() }));

            let body = node
                .children()
                .find(|c| c.kind() == SyntaxKind::CodeBlock)
                .map(|c| self.lower_block(&c))
                .unwrap_or_else(|| AstBlock {
                    stmts: Vec::new(),
                    tail_expr: None,
                });

            self.alloc_expr(AstExpr::While {
                label,
                condition,
                body,
                span,
            })
        }
    }

    fn lower_for(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);
        let label = extract_loop_label(node);

        // ForPattern child wraps the pattern
        let pattern = find_child(node, SyntaxKind::ForPattern)
            .and_then(|fp| {
                fp.children()
                    .find(|c| c.kind() == SyntaxKind::Pattern || is_pattern_kind(c.kind()))
            })
            .map(|p| self.lower_pat(&p))
            .unwrap_or_else(|| self.alloc_pat(AstPat::Error { span: span.clone() }));

        // ForIterable child wraps the iterable expression
        let iterable = find_child(node, SyntaxKind::ForIterable)
            .and_then(|fi| {
                fi.children()
                    .find(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()))
            })
            .map(|e| self.lower_expr(&e))
            .unwrap_or_else(|| self.alloc_expr(AstExpr::Error { span: span.clone() }));

        let body = node
            .children()
            .find(|c| c.kind() == SyntaxKind::CodeBlock)
            .map(|c| self.lower_block(&c))
            .unwrap_or_else(|| AstBlock {
                stmts: Vec::new(),
                tail_expr: None,
            });

        self.alloc_expr(AstExpr::For {
            label,
            pattern,
            iterable,
            body,
            span,
        })
    }

    fn lower_loop(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);
        let label = extract_loop_label(node);

        let body = node
            .children()
            .find(|c| c.kind() == SyntaxKind::CodeBlock)
            .map(|c| self.lower_block(&c))
            .unwrap_or_else(|| AstBlock {
                stmts: Vec::new(),
                tail_expr: None,
            });

        self.alloc_expr(AstExpr::Loop { label, body, span })
    }

    fn lower_break(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);
        let label = extract_jump_label(node);
        self.alloc_expr(AstExpr::Break { label, span })
    }

    fn lower_continue(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);
        let label = extract_jump_label(node);
        self.alloc_expr(AstExpr::Continue { label, span })
    }

    fn lower_return(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);
        let value = node
            .children()
            .find(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()))
            .map(|c| self.lower_expr(&c));
        self.alloc_expr(AstExpr::Return { value, span })
    }

    fn lower_throw(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);
        let value = node
            .children()
            .find(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()))
            .map(|c| self.lower_expr(&c))
            .unwrap_or_else(|| self.alloc_expr(AstExpr::Error { span: span.clone() }));
        self.alloc_expr(AstExpr::Throw { value, span })
    }

    fn lower_try(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);
        let operand = node
            .children()
            .find(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()))
            .map(|c| self.lower_expr(&c))
            .unwrap_or_else(|| self.alloc_expr(AstExpr::Error { span: span.clone() }));
        self.alloc_expr(AstExpr::Try { operand, span })
    }

    // ----- Closure -----

    /// Lower a parameterless closure as a block expression.
    /// Used in match arm bodies where `{ stmts; expr }` is parsed as a closure.
    fn lower_closure_as_block(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);
        let body = node
            .children()
            .find(|c| c.kind() == SyntaxKind::CodeBlock)
            .map(|c| self.lower_block(&c))
            .unwrap_or_else(|| self.lower_closure_body(node));
        self.alloc_expr(AstExpr::Block { body, span })
    }

    fn lower_closure(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);

        // ClosureParams child (optional)
        let has_explicit_params = find_child(node, SyntaxKind::ClosureParams).is_some();
        let mut params: Vec<ClosureParam> = find_child(node, SyntaxKind::ClosureParams)
            .map(|cp| {
                cp.children()
                    .filter(|c| c.kind() == SyntaxKind::ClosureParam)
                    .map(|param_node| {
                        let pattern = param_node
                            .children()
                            .find(|c| c.kind() == SyntaxKind::Pattern || is_pattern_kind(c.kind()))
                            .map(|p| self.lower_pat(&p))
                            .unwrap_or_else(|| {
                                self.alloc_pat(AstPat::Error {
                                    span: self.span(&param_node),
                                })
                            });

                        let ty = param_node
                            .children()
                            .find(|c| is_type_kind(c.kind()))
                            .and_then(|c| ast_type_from_cst(&c, self.file_id));

                        ClosureParam { pattern, ty }
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Implicit `it` parameter: when a closure has no explicit params and its
        // body references `it` (not inside a nested closure), inject `it` as a param.
        // `{ it + 1 }` becomes `{ (it) in it + 1 }`.
        if !has_explicit_params && params.is_empty() && closure_body_references_it(node) {
            let pat = self.alloc_pat(AstPat::Binding {
                is_mut: false,
                name: "it".to_string(),
                span: span.clone(),
            });
            params.push(ClosureParam { pattern: pat, ty: None });
        }

        // Body: the CodeBlock inside the closure, or synthesize from inner statements
        let body = node
            .children()
            .find(|c| c.kind() == SyntaxKind::CodeBlock)
            .map(|c| self.lower_block(&c))
            .unwrap_or_else(|| {
                // Closure body without explicit CodeBlock — collect inner
                // statements/expressions directly
                self.lower_closure_body(node)
            });

        self.alloc_expr(AstExpr::Closure { params, body, span })
    }

    /// Lower closure body items when there's no explicit CodeBlock wrapper.
    fn lower_closure_body(&mut self, node: &SyntaxNode) -> AstBlock {
        let mut stmts = Vec::new();
        let mut tail_expr = None;

        for child in node.children() {
            match child.kind() {
                SyntaxKind::Statement => {
                    if let Some(inner) = child.children().next() {
                        let stmt_id = self.lower_stmt(&inner);
                        stmts.push(stmt_id);
                    }
                }
                SyntaxKind::Expression if !matches!(child.kind(), SyntaxKind::ClosureParams) => {
                    // Convert previous tail to statement
                    if let Some(prev) = tail_expr.take() {
                        let prev_span = match &self.exprs[prev] {
                            AstExpr::Error { span } => span.clone(),
                            _ => Span::synthetic(self.file_id),
                        };
                        let stmt = self.alloc_stmt(AstStmt::Expr {
                            expr: prev,
                            span: prev_span,
                        });
                        stmts.push(stmt);
                    }
                    tail_expr = Some(self.lower_expr(&child));
                }
                // Skip ClosureParams, In, LBrace, RBrace
                _ => {}
            }
        }

        AstBlock { stmts, tail_expr }
    }

    // ----- Match -----

    fn lower_match(&mut self, node: &SyntaxNode) -> ExprId {
        let span = self.span(node);

        // Scrutinee: first expression child
        let scrutinee = node
            .children()
            .find(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()))
            .map(|c| self.lower_expr(&c))
            .unwrap_or_else(|| self.alloc_expr(AstExpr::Error { span: span.clone() }));

        // Match arms
        let arms = node
            .children()
            .filter(|c| c.kind() == SyntaxKind::MatchArm)
            .map(|arm| self.lower_match_arm(&arm))
            .collect();

        self.alloc_expr(AstExpr::Match {
            scrutinee,
            arms,
            span,
        })
    }

    fn lower_match_arm(&mut self, node: &SyntaxNode) -> MatchArm {
        // Pattern
        let pattern = node
            .children()
            .find(|c| c.kind() == SyntaxKind::Pattern || is_pattern_kind(c.kind()))
            .map(|p| self.lower_pat(&p))
            .unwrap_or_else(|| {
                self.alloc_pat(AstPat::Error {
                    span: self.span(node),
                })
            });

        // Optional guard
        let guard = find_child(node, SyntaxKind::MatchArmGuard).map(|g| {
            g.children()
                .find(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()))
                .map(|c| self.lower_expr(&c))
                .unwrap_or_else(|| {
                    self.alloc_expr(AstExpr::Error {
                        span: self.span(&g),
                    })
                })
        });

        // Body: expression after FatArrow
        // The body expression is the last expression child (after pattern + guard).
        // Special case: `{ stmts; expr }` may be parsed as a closure with no params
        // instead of a block expression. Detect and unwrap — use the closure body's
        // tail expression directly (or wrap statements + tail into an if-true block).
        let body = node
            .children()
            .filter(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()))
            .last()
            .map(|c| {
                let inner = unwrap_expr(&c);
                if inner.kind() == SyntaxKind::ExprClosure
                    && find_child(&inner, SyntaxKind::ClosureParams).is_none()
                {
                    // Parameterless closure = block expression in match arm.
                    // Lower it as a closure and extract the body.
                    self.lower_closure_as_block(&inner)
                } else {
                    self.lower_expr(&c)
                }
            })
            .unwrap_or_else(|| {
                self.alloc_expr(AstExpr::Error {
                    span: self.span(node),
                })
            });

        MatchArm {
            pattern,
            guard,
            body,
        }
    }

    // ===== Pattern lowering =====

    fn lower_pat(&mut self, node: &SyntaxNode) -> PatId {
        // Unwrap Pattern wrapper
        let node = unwrap_pattern(node);

        match node.kind() {
            SyntaxKind::WildcardPattern => {
                let span = self.span(&node);
                self.alloc_pat(AstPat::Wildcard { span })
            }
            SyntaxKind::BindingPattern => self.lower_binding_pattern(&node),
            SyntaxKind::TuplePattern => self.lower_tuple_pattern(&node),
            SyntaxKind::LiteralPattern => self.lower_literal_pattern(&node),
            SyntaxKind::RangePattern => self.lower_range_pattern(&node),
            SyntaxKind::EnumPattern => self.lower_enum_pattern(&node),
            SyntaxKind::StructPattern => self.lower_struct_pattern(&node),
            SyntaxKind::ArrayPattern => self.lower_array_pattern(&node),
            SyntaxKind::AtPattern => self.lower_at_pattern(&node),
            SyntaxKind::OrPattern => self.lower_or_pattern(&node),
            SyntaxKind::RestPattern => {
                let span = self.span(&node);
                self.alloc_pat(AstPat::Rest { span })
            }
            SyntaxKind::ErrorPattern => {
                let span = self.span(&node);
                self.alloc_pat(AstPat::Error { span })
            }
            _ => {
                let span = self.span(&node);
                self.alloc_pat(AstPat::Error { span })
            }
        }
    }

    fn lower_binding_pattern(&mut self, node: &SyntaxNode) -> PatId {
        let span = self.span(node);

        let is_mut = node
            .children_with_tokens()
            .any(|e| e.as_token().is_some_and(|t| t.kind() == SyntaxKind::Var));

        let name = node
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::Identifier)
            .map(|t| t.text().to_string())
            .unwrap_or_default();

        self.alloc_pat(AstPat::Binding { is_mut, name, span })
    }

    fn lower_tuple_pattern(&mut self, node: &SyntaxNode) -> PatId {
        let span = self.span(node);

        let elements: Vec<PatId> = node
            .children()
            .filter(|c| {
                c.kind() == SyntaxKind::TuplePatternElement
                    || c.kind() == SyntaxKind::Pattern
                    || is_pattern_kind(c.kind())
            })
            .map(|c| {
                // TuplePatternElement wraps a Pattern
                if c.kind() == SyntaxKind::TuplePatternElement {
                    c.children()
                        .find(|p| p.kind() == SyntaxKind::Pattern || is_pattern_kind(p.kind()))
                        .map(|p| self.lower_pat(&p))
                        .unwrap_or_else(|| {
                            self.alloc_pat(AstPat::Error {
                                span: self.span(&c),
                            })
                        })
                } else {
                    self.lower_pat(&c)
                }
            })
            .collect();

        self.alloc_pat(AstPat::Tuple { elements, span })
    }

    fn lower_literal_pattern(&mut self, node: &SyntaxNode) -> PatId {
        let span = self.span(node);

        // Find the literal token
        let kind = node
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| !is_trivia(t.kind()))
            .map(|t| {
                let text = t.text().to_string();
                match t.kind() {
                    SyntaxKind::Integer => LitPatKind::Integer(text),
                    SyntaxKind::Float => LitPatKind::Float(text),
                    SyntaxKind::String => LitPatKind::String(text),
                    SyntaxKind::Boolean => LitPatKind::Bool(text == "true"),
                    SyntaxKind::Char => LitPatKind::Char(text),
                    _ => LitPatKind::Integer(text), // fallback
                }
            })
            .unwrap_or(LitPatKind::Integer("0".into()));

        self.alloc_pat(AstPat::Literal { kind, span })
    }

    fn lower_range_pattern(&mut self, node: &SyntaxNode) -> PatId {
        let span = self.span(node);

        // Determine inclusivity from operator token
        let inclusive = node.children_with_tokens().any(|e| {
            e.as_token()
                .is_some_and(|t| t.kind() == SyntaxKind::DotDotEquals)
        });

        // Start and end are literal tokens before/after the range operator
        let mut before_op = true;
        let mut start = None;
        let mut end = None;

        for elem in node.children_with_tokens() {
            if let Some(token) = elem.as_token() {
                match token.kind() {
                    SyntaxKind::DotDotEquals | SyntaxKind::DotDotLess | SyntaxKind::DotDot => {
                        before_op = false;
                    }
                    kind if !is_trivia(kind) => {
                        let text = token.text().to_string();
                        let lit = match kind {
                            SyntaxKind::Integer => LitPatKind::Integer(text),
                            SyntaxKind::Float => LitPatKind::Float(text),
                            SyntaxKind::String => LitPatKind::String(text),
                            SyntaxKind::Char => LitPatKind::Char(text),
                            SyntaxKind::Boolean => LitPatKind::Bool(text == "true"),
                            _ => continue,
                        };
                        if before_op {
                            start = Some(lit);
                        } else {
                            end = Some(lit);
                        }
                    }
                    _ => {}
                }
            }
        }

        self.alloc_pat(AstPat::Range {
            start,
            end,
            inclusive,
            span,
        })
    }

    fn lower_enum_pattern(&mut self, node: &SyntaxNode) -> PatId {
        let span = self.span(node);

        // Case name: Identifier token after the Dot
        let case_name = node
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::Identifier)
            .map(|t| t.text().to_string())
            .unwrap_or_default();

        // Arguments: EnumPatternArg children
        let args: Vec<EnumPatArg> = node
            .children()
            .filter(|c| c.kind() == SyntaxKind::EnumPatternArg)
            .map(|arg_node| {
                // EnumPatternArg: [Identifier [Colon]] Pattern
                let label = self.extract_pattern_arg_label(&arg_node);

                let pattern = arg_node
                    .children()
                    .find(|c| c.kind() == SyntaxKind::Pattern || is_pattern_kind(c.kind()))
                    .map(|p| self.lower_pat(&p))
                    .unwrap_or_else(|| {
                        // If no pattern, the identifier itself may be a binding
                        let name = arg_node
                            .children_with_tokens()
                            .filter_map(|e| e.into_token())
                            .find(|t| t.kind() == SyntaxKind::Identifier)
                            .map(|t| t.text().to_string())
                            .unwrap_or_default();
                        self.alloc_pat(AstPat::Binding {
                            is_mut: false,
                            name,
                            span: self.span(&arg_node),
                        })
                    });

                EnumPatArg { label, pattern }
            })
            .collect();

        self.alloc_pat(AstPat::Enum {
            case_name,
            args,
            span,
        })
    }

    /// Extract label from an EnumPatternArg: Identifier followed by Colon.
    fn extract_pattern_arg_label(&self, node: &SyntaxNode) -> Option<String> {
        let mut iter = node
            .children_with_tokens()
            .filter(|e| !e.as_token().is_some_and(|t| is_trivia(t.kind())));

        let first = iter.next()?;
        let second = iter.next();

        if first
            .as_token()
            .is_some_and(|t| t.kind() == SyntaxKind::Identifier)
            && second
                .as_ref()
                .is_some_and(|s| s.as_token().is_some_and(|t| t.kind() == SyntaxKind::Colon))
        {
            Some(first.as_token().unwrap().text().to_string())
        } else {
            None
        }
    }

    fn lower_struct_pattern(&mut self, node: &SyntaxNode) -> PatId {
        let span = self.span(node);

        let name = node
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::Identifier)
            .map(|t| t.text().to_string())
            .unwrap_or_default();

        let fields: Vec<StructPatField> = node
            .children()
            .filter(|c| c.kind() == SyntaxKind::StructPatternField)
            .map(|field_node| {
                let field_name = field_node
                    .children_with_tokens()
                    .filter_map(|e| e.into_token())
                    .find(|t| t.kind() == SyntaxKind::Identifier)
                    .map(|t| t.text().to_string())
                    .unwrap_or_default();

                // Optional pattern after Colon
                let pattern = field_node
                    .children()
                    .find(|c| c.kind() == SyntaxKind::Pattern || is_pattern_kind(c.kind()))
                    .map(|p| self.lower_pat(&p));

                StructPatField {
                    field_name,
                    pattern,
                }
            })
            .collect();

        let has_rest = node
            .children()
            .any(|c| c.kind() == SyntaxKind::StructPatternRest);

        self.alloc_pat(AstPat::Struct {
            name,
            fields,
            has_rest,
            span,
        })
    }

    fn lower_array_pattern(&mut self, node: &SyntaxNode) -> PatId {
        let span = self.span(node);

        let mut prefix = Vec::new();
        let mut rest: Option<Option<String>> = None;
        let mut suffix = Vec::new();
        let mut seen_rest = false;

        for child in node.children() {
            match child.kind() {
                SyntaxKind::ArrayPatternElement => {
                    let pat = child
                        .children()
                        .find(|c| c.kind() == SyntaxKind::Pattern || is_pattern_kind(c.kind()))
                        .map(|p| self.lower_pat(&p))
                        .unwrap_or_else(|| {
                            self.alloc_pat(AstPat::Error {
                                span: self.span(&child),
                            })
                        });

                    if seen_rest {
                        suffix.push(pat);
                    } else {
                        prefix.push(pat);
                    }
                }
                SyntaxKind::ArrayPatternRest => {
                    seen_rest = true;
                    // Optional binding name
                    let binding = child
                        .children_with_tokens()
                        .filter_map(|e| e.into_token())
                        .find(|t| t.kind() == SyntaxKind::Identifier)
                        .map(|t| t.text().to_string());
                    rest = Some(binding);
                }
                _ => {}
            }
        }

        self.alloc_pat(AstPat::Array {
            prefix,
            rest,
            suffix,
            span,
        })
    }

    fn lower_at_pattern(&mut self, node: &SyntaxNode) -> PatId {
        let span = self.span(node);

        let is_mut = node
            .children_with_tokens()
            .any(|e| e.as_token().is_some_and(|t| t.kind() == SyntaxKind::Var));

        let name = node
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::Identifier)
            .map(|t| t.text().to_string())
            .unwrap_or_default();

        let subpattern = node
            .children()
            .find(|c| c.kind() == SyntaxKind::Pattern || is_pattern_kind(c.kind()))
            .map(|p| self.lower_pat(&p))
            .unwrap_or_else(|| self.alloc_pat(AstPat::Error { span: span.clone() }));

        self.alloc_pat(AstPat::At {
            is_mut,
            name,
            subpattern,
            span,
        })
    }

    fn lower_or_pattern(&mut self, node: &SyntaxNode) -> PatId {
        let span = self.span(node);

        let alternatives: Vec<PatId> = node
            .children()
            .filter(|c| c.kind() == SyntaxKind::Pattern || is_pattern_kind(c.kind()))
            .map(|p| self.lower_pat(&p))
            .collect();

        self.alloc_pat(AstPat::Or {
            alternatives,
            span,
        })
    }

    // ===== Shared helpers =====

    /// Lower IfLetCondition/WhileLetCondition/GuardLetCondition nodes.
    fn lower_let_conditions(
        &mut self,
        parent: &SyntaxNode,
        condition_kind: SyntaxKind,
    ) -> Vec<IfCondition> {
        let mut conditions = Vec::new();

        for child in parent.children() {
            if child.kind() == condition_kind {
                let (pat, val) = self.lower_let_condition_parts(&child);
                conditions.push(IfCondition::Let {
                    pattern: pat,
                    value: val,
                });
            } else if child.kind() == SyntaxKind::Expression || is_expr_kind(child.kind()) {
                // Mixed conditions can include plain expressions
                if appears_before_code_block(parent, &child) {
                    let expr = self.lower_expr(&child);
                    conditions.push(IfCondition::Expr(expr));
                }
            } else if child.kind() == SyntaxKind::CodeBlock {
                break;
            }
        }

        conditions
    }

    /// Extract pattern + value from a let-condition node.
    fn lower_let_condition_parts(&mut self, node: &SyntaxNode) -> (PatId, ExprId) {
        let span = self.span(node);

        let pattern = node
            .children()
            .find(|c| c.kind() == SyntaxKind::Pattern || is_pattern_kind(c.kind()))
            .map(|c| self.lower_pat(&c))
            .unwrap_or_else(|| self.alloc_pat(AstPat::Error { span: span.clone() }));

        let value = node
            .children()
            .find(|c| c.kind() == SyntaxKind::Expression || is_expr_kind(c.kind()))
            .map(|c| self.lower_expr(&c))
            .unwrap_or_else(|| self.alloc_expr(AstExpr::Error { span }));

        (pattern, value)
    }
}

// ===== Free helper functions =====

/// Unwrap Expression wrapper nodes to get the inner expression variant.
fn unwrap_expr(node: &SyntaxNode) -> SyntaxNode {
    let mut current = node.clone();
    while current.kind() == SyntaxKind::Expression {
        if let Some(inner) = current.children().next() {
            current = inner;
        } else {
            break;
        }
    }
    current
}

/// Unwrap Pattern wrapper nodes.
fn unwrap_pattern(node: &SyntaxNode) -> SyntaxNode {
    let mut current = node.clone();
    while current.kind() == SyntaxKind::Pattern {
        if let Some(inner) = current.children().next() {
            current = inner;
        } else {
            break;
        }
    }
    current
}

/// Check if a SyntaxKind is an expression node.
/// Check if an ExpressionStatement node has a trailing semicolon token.
fn has_semicolon(node: &SyntaxNode) -> bool {
    node.children_with_tokens()
        .any(|e| e.as_token().is_some_and(|t| t.kind() == SyntaxKind::Semicolon))
}

fn is_expr_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Expression
            | SyntaxKind::ExprUnit
            | SyntaxKind::ExprInteger
            | SyntaxKind::ExprFloat
            | SyntaxKind::ExprString
            | SyntaxKind::ExprRawString
            | SyntaxKind::ExprInterpolatedString
            | SyntaxKind::ExprChar
            | SyntaxKind::ExprBool
            | SyntaxKind::ExprNull
            | SyntaxKind::ExprArray
            | SyntaxKind::ExprDictionary
            | SyntaxKind::ExprTuple
            | SyntaxKind::ExprGrouping
            | SyntaxKind::ExprPath
            | SyntaxKind::ExprTupleIndex
            | SyntaxKind::ExprImplicitMemberAccess
            | SyntaxKind::ExprUnary
            | SyntaxKind::ExprPostfix
            | SyntaxKind::ExprBinary
            | SyntaxKind::ExprAssignment
            | SyntaxKind::ExprCompoundAssignment
            | SyntaxKind::ExprCall
            | SyntaxKind::ExprIf
            | SyntaxKind::ExprWhile
            | SyntaxKind::ExprFor
            | SyntaxKind::ExprLoop
            | SyntaxKind::ExprBreak
            | SyntaxKind::ExprContinue
            | SyntaxKind::ExprReturn
            | SyntaxKind::ExprThrow
            | SyntaxKind::ExprTry
            | SyntaxKind::ExprClosure
            | SyntaxKind::ExprMatch
    )
}

/// Check if a SyntaxKind is a pattern node.
fn is_pattern_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Pattern
            | SyntaxKind::WildcardPattern
            | SyntaxKind::BindingPattern
            | SyntaxKind::TuplePattern
            | SyntaxKind::LiteralPattern
            | SyntaxKind::RangePattern
            | SyntaxKind::EnumPattern
            | SyntaxKind::StructPattern
            | SyntaxKind::ArrayPattern
            | SyntaxKind::AtPattern
            | SyntaxKind::OrPattern
            | SyntaxKind::RestPattern
            | SyntaxKind::ErrorPattern
    )
}

/// Get the first non-trivia token text from a node.
fn first_token_text(node: &SyntaxNode) -> Option<String> {
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| !is_trivia(t.kind()))
        .map(|t| t.text().to_string())
}

/// Extract loop label from a LoopLabel child: `label:`
fn extract_loop_label(node: &SyntaxNode) -> Option<String> {
    find_child(node, SyntaxKind::LoopLabel).and_then(|ll| {
        ll.children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::Identifier)
            .map(|t| t.text().to_string())
    })
}

/// Extract label from break/continue: just an identifier token after the keyword.
fn extract_jump_label(node: &SyntaxNode) -> Option<String> {
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| t.text().to_string())
}

/// Extract type arguments from a TypeArgumentList node.
fn extract_type_args(node: &SyntaxNode, file_id: usize) -> Vec<AstType> {
    node.children()
        .filter(|c| is_type_kind(c.kind()))
        .filter_map(|c| ast_type_from_cst(&c, file_id))
        .collect()
}

/// Check if a child node appears before any CodeBlock sibling in parent.
fn appears_before_code_block(parent: &SyntaxNode, child: &SyntaxNode) -> bool {
    for sibling in parent.children() {
        if sibling == *child {
            return true;
        }
        if sibling.kind() == SyntaxKind::CodeBlock {
            return false;
        }
    }
    true
}

/// Check if there's a CodeBlock before this child in the parent.
fn has_sibling_code_block_before(parent: &SyntaxNode, child: &SyntaxNode) -> bool {
    for sibling in parent.children() {
        if sibling == *child {
            return false;
        }
        if sibling.kind() == SyntaxKind::CodeBlock {
            return true;
        }
    }
    false
}

// ===== Token-to-operator mappings =====

fn token_to_unary_op(kind: SyntaxKind) -> Option<UnaryOp> {
    match kind {
        SyntaxKind::Minus => Some(UnaryOp::Neg),
        SyntaxKind::Not | SyntaxKind::Bang => Some(UnaryOp::LogicalNot),
        SyntaxKind::Caret => Some(UnaryOp::BitNot),
        SyntaxKind::Plus => Some(UnaryOp::Pos),
        _ => None,
    }
}

fn token_to_binary_op(kind: SyntaxKind) -> Option<BinaryOp> {
    match kind {
        SyntaxKind::Plus => Some(BinaryOp::Add),
        SyntaxKind::Minus => Some(BinaryOp::Sub),
        SyntaxKind::Star => Some(BinaryOp::Mul),
        SyntaxKind::Slash => Some(BinaryOp::Div),
        SyntaxKind::Percent => Some(BinaryOp::Rem),
        SyntaxKind::Ampersand => Some(BinaryOp::BitAnd),
        SyntaxKind::Pipe => Some(BinaryOp::BitOr),
        SyntaxKind::Caret => Some(BinaryOp::BitXor),
        SyntaxKind::LessLess => Some(BinaryOp::Shl),
        SyntaxKind::GreaterGreater => Some(BinaryOp::Shr),
        SyntaxKind::EqualsEquals => Some(BinaryOp::Eq),
        SyntaxKind::BangEquals => Some(BinaryOp::Ne),
        SyntaxKind::Less => Some(BinaryOp::Lt),
        SyntaxKind::Greater => Some(BinaryOp::Gt),
        SyntaxKind::LessEquals => Some(BinaryOp::Le),
        SyntaxKind::GreaterEquals => Some(BinaryOp::Ge),
        SyntaxKind::And => Some(BinaryOp::And),
        SyntaxKind::Or => Some(BinaryOp::Or),
        SyntaxKind::QuestionQuestion => Some(BinaryOp::Coalesce),
        SyntaxKind::DotDotEquals => Some(BinaryOp::RangeInclusive),
        SyntaxKind::DotDotLess => Some(BinaryOp::RangeExclusive),
        _ => None,
    }
}

fn token_to_compound_assign_op(kind: SyntaxKind) -> Option<CompoundAssignOp> {
    match kind {
        SyntaxKind::PlusEquals => Some(CompoundAssignOp::AddAssign),
        SyntaxKind::MinusEquals => Some(CompoundAssignOp::SubAssign),
        SyntaxKind::StarEquals => Some(CompoundAssignOp::MulAssign),
        SyntaxKind::SlashEquals => Some(CompoundAssignOp::DivAssign),
        SyntaxKind::PercentEquals => Some(CompoundAssignOp::RemAssign),
        SyntaxKind::AmpersandEquals => Some(CompoundAssignOp::BitAndAssign),
        SyntaxKind::PipeEquals => Some(CompoundAssignOp::BitOrAssign),
        SyntaxKind::CaretEquals => Some(CompoundAssignOp::BitXorAssign),
        SyntaxKind::LessLessEquals => Some(CompoundAssignOp::ShlAssign),
        SyntaxKind::GreaterGreaterEquals => Some(CompoundAssignOp::ShrAssign),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_hecs::World;
    use crate::build::build_declarations;
    use crate::components::*;

    /// Parse source, build declarations, find the first function with a Body,
    /// and return its AstBody.
    fn lower_func_body(source: &str) -> AstBody {
        let mut world = World::new();
        world.begin_revision();
        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".to_string()));
        let file = world.spawn();

        let tokens: Vec<_> = kestrel_lexer2::lex(source, file.index())
            .filter_map(|r| r.ok())
            .collect();
        let result = kestrel_parser2::parse_source_file_from_source(
            source,
            tokens.iter().map(|t| (t.value.clone(), t.span.clone())),
        );
        build_declarations(&mut world, file, &result.tree, root, None);

        // Find first entity with a Body component
        find_body(&world, root).expect("no Body found in declarations")
    }

    /// Recursively search for an entity with a Body component.
    fn find_body(world: &World, entity: kestrel_hecs::Entity) -> Option<AstBody> {
        if let Some(body) = world.get::<Body>(entity) {
            return Some(body.0.clone());
        }
        for &child in world.children_of(entity) {
            if let Some(body) = find_body(world, child) {
                return Some(body);
            }
        }
        None
    }

    // ================================================================
    // Literals
    // ================================================================

    #[test]
    fn literal_integer() {
        let body = lower_func_body("func f() { 42; }");
        assert_eq!(body.exprs.len(), 1);
        let stmt = &body.stmts[body.statements[0]];
        if let AstStmt::Expr { expr, .. } = stmt {
            match &body.exprs[*expr] {
                AstExpr::Literal { kind: AstLiteral::Integer(v), .. } => assert_eq!(v, "42"),
                other => panic!("expected Integer literal, got {:?}", other),
            }
        } else {
            panic!("expected Expr stmt");
        }
    }

    #[test]
    fn literal_bool_string_null() {
        let body = lower_func_body("func f() { true; \"hello\"; null; }");
        assert_eq!(body.statements.len(), 3);

        let s0 = &body.stmts[body.statements[0]];
        if let AstStmt::Expr { expr, .. } = s0 {
            assert!(matches!(&body.exprs[*expr], AstExpr::Literal { kind: AstLiteral::Bool(true), .. }));
        }

        let s1 = &body.stmts[body.statements[1]];
        if let AstStmt::Expr { expr, .. } = s1 {
            assert!(matches!(&body.exprs[*expr], AstExpr::Literal { kind: AstLiteral::String(_), .. }));
        }

        let s2 = &body.stmts[body.statements[2]];
        if let AstStmt::Expr { expr, .. } = s2 {
            assert!(matches!(&body.exprs[*expr], AstExpr::Literal { kind: AstLiteral::Null, .. }));
        }
    }

    // ================================================================
    // Patterns
    // ================================================================

    #[test]
    fn pattern_binding_wildcard() {
        let body = lower_func_body("func f() { let x = 1; let _ = 2; }");
        assert_eq!(body.statements.len(), 2);

        if let AstStmt::Let { pattern, is_mut, .. } = &body.stmts[body.statements[0]] {
            assert!(!is_mut);
            match &body.pats[*pattern] {
                AstPat::Binding { name, is_mut, .. } => {
                    assert_eq!(name, "x");
                    assert!(!is_mut);
                }
                other => panic!("expected Binding, got {:?}", other),
            }
        }

        if let AstStmt::Let { pattern, .. } = &body.stmts[body.statements[1]] {
            assert!(matches!(&body.pats[*pattern], AstPat::Wildcard { .. }));
        }
    }

    #[test]
    fn pattern_var_binding() {
        let body = lower_func_body("func f() { var x = 1; }");
        if let AstStmt::Let { is_mut, pattern, .. } = &body.stmts[body.statements[0]] {
            assert!(*is_mut);
            match &body.pats[*pattern] {
                AstPat::Binding { name, .. } => assert_eq!(name, "x"),
                other => panic!("expected Binding, got {:?}", other),
            }
        }
    }

    // ================================================================
    // Statements
    // ================================================================

    #[test]
    fn let_with_type_and_value() {
        let body = lower_func_body("func f() { let x: Int64 = 42; }");
        if let AstStmt::Let { ty, value, .. } = &body.stmts[body.statements[0]] {
            assert!(ty.is_some(), "should have type annotation");
            assert!(value.is_some(), "should have initializer");
        } else {
            panic!("expected Let stmt");
        }
    }

    #[test]
    fn expression_statement() {
        let body = lower_func_body("func f() { foo; }");
        assert_eq!(body.statements.len(), 1);
        assert!(matches!(&body.stmts[body.statements[0]], AstStmt::Expr { .. }));
    }

    // ================================================================
    // Operators
    // ================================================================

    #[test]
    fn binary_operators() {
        let body = lower_func_body("func f() { 1 + 2; }");
        if let AstStmt::Expr { expr, .. } = &body.stmts[body.statements[0]] {
            match &body.exprs[*expr] {
                AstExpr::Binary { op, .. } => assert_eq!(*op, BinaryOp::Add),
                other => panic!("expected Binary, got {:?}", other),
            }
        }
    }

    #[test]
    fn unary_operator() {
        let body = lower_func_body("func f() { -x; }");
        if let AstStmt::Expr { expr, .. } = &body.stmts[body.statements[0]] {
            match &body.exprs[*expr] {
                AstExpr::Unary { op, .. } => assert_eq!(*op, UnaryOp::Neg),
                other => panic!("expected Unary, got {:?}", other),
            }
        }
    }

    #[test]
    fn compound_assignment() {
        let body = lower_func_body("func f() { x += 1; }");
        if let AstStmt::Expr { expr, .. } = &body.stmts[body.statements[0]] {
            match &body.exprs[*expr] {
                AstExpr::CompoundAssignment { op, .. } => {
                    assert_eq!(*op, CompoundAssignOp::AddAssign);
                }
                other => panic!("expected CompoundAssignment, got {:?}", other),
            }
        }
    }

    // ================================================================
    // Path vs Member Access
    // ================================================================

    #[test]
    fn simple_path() {
        let body = lower_func_body("func f() { foo; }");
        if let AstStmt::Expr { expr, .. } = &body.stmts[body.statements[0]] {
            match &body.exprs[*expr] {
                AstExpr::Path { segments, .. } => {
                    assert_eq!(segments.len(), 1);
                    assert_eq!(segments[0].name, "foo");
                }
                other => panic!("expected Path, got {:?}", other),
            }
        }
    }

    // ================================================================
    // Calls
    // ================================================================

    #[test]
    fn call_with_labeled_args() {
        let body = lower_func_body("func f() { foo(x: 1, y: 2); }");
        if let AstStmt::Expr { expr, .. } = &body.stmts[body.statements[0]] {
            match &body.exprs[*expr] {
                AstExpr::Call { arguments, .. } => {
                    assert_eq!(arguments.len(), 2);
                    assert_eq!(arguments[0].label.as_deref(), Some("x"));
                    assert_eq!(arguments[1].label.as_deref(), Some("y"));
                }
                other => panic!("expected Call, got {:?}", other),
            }
        }
    }

    #[test]
    fn call_no_labels() {
        let body = lower_func_body("func f() { foo(1, 2); }");
        if let AstStmt::Expr { expr, .. } = &body.stmts[body.statements[0]] {
            match &body.exprs[*expr] {
                AstExpr::Call { arguments, .. } => {
                    assert_eq!(arguments.len(), 2);
                    assert!(arguments[0].label.is_none());
                    assert!(arguments[1].label.is_none());
                }
                other => panic!("expected Call, got {:?}", other),
            }
        }
    }

    // ================================================================
    // Control flow
    // ================================================================

    #[test]
    fn if_else() {
        let body = lower_func_body("func f() { if true { 1; } else { 2; } }");
        if let AstStmt::Expr { expr, .. } = &body.stmts[body.statements[0]] {
            match &body.exprs[*expr] {
                AstExpr::If { conditions, then_body, else_body, .. } => {
                    assert_eq!(conditions.len(), 1);
                    assert!(matches!(&conditions[0], IfCondition::Expr(_)));
                    assert!(!then_body.stmts.is_empty() || then_body.tail_expr.is_some());
                    assert!(else_body.is_some());
                }
                other => panic!("expected If, got {:?}", other),
            }
        }
    }

    #[test]
    fn while_loop() {
        let body = lower_func_body("func f() { while true { break; } }");
        if let AstStmt::Expr { expr, .. } = &body.stmts[body.statements[0]] {
            match &body.exprs[*expr] {
                AstExpr::While { label, .. } => assert!(label.is_none()),
                other => panic!("expected While, got {:?}", other),
            }
        }
    }

    #[test]
    fn for_loop() {
        let body = lower_func_body("func f() { for x in items { x; } }");
        if let AstStmt::Expr { expr, .. } = &body.stmts[body.statements[0]] {
            match &body.exprs[*expr] {
                AstExpr::For { pattern, label, .. } => {
                    assert!(label.is_none());
                    match &body.pats[*pattern] {
                        AstPat::Binding { name, .. } => assert_eq!(name, "x"),
                        other => panic!("expected Binding pattern, got {:?}", other),
                    }
                }
                other => panic!("expected For, got {:?}", other),
            }
        }
    }

    #[test]
    fn loop_with_break_continue() {
        let body = lower_func_body("func f() { loop { break; continue; } }");
        if let AstStmt::Expr { expr, .. } = &body.stmts[body.statements[0]] {
            match &body.exprs[*expr] {
                AstExpr::Loop { body: loop_body, .. } => {
                    assert_eq!(loop_body.stmts.len(), 2);
                }
                other => panic!("expected Loop, got {:?}", other),
            }
        }
    }

    #[test]
    fn return_with_value() {
        let body = lower_func_body("func f() { return 42; }");
        if let AstStmt::Expr { expr, .. } = &body.stmts[body.statements[0]] {
            match &body.exprs[*expr] {
                AstExpr::Return { value, .. } => assert!(value.is_some()),
                other => panic!("expected Return, got {:?}", other),
            }
        }
    }

    #[test]
    fn return_void() {
        let body = lower_func_body("func f() { return; }");
        if let AstStmt::Expr { expr, .. } = &body.stmts[body.statements[0]] {
            match &body.exprs[*expr] {
                AstExpr::Return { value, .. } => assert!(value.is_none()),
                other => panic!("expected Return, got {:?}", other),
            }
        }
    }

    // ================================================================
    // Match
    // ================================================================

    #[test]
    fn match_expression() {
        let body = lower_func_body("func f() { match x { .A => 1, .B => 2 } }");
        // Match may be tail expr or statement depending on parser
        let expr_id = body.tail_expr.or_else(|| {
            body.statements.first().and_then(|s| {
                if let AstStmt::Expr { expr, .. } = &body.stmts[*s] {
                    Some(*expr)
                } else {
                    None
                }
            })
        }).expect("match should be in body");

        match &body.exprs[expr_id] {
            AstExpr::Match { arms, .. } => {
                assert_eq!(arms.len(), 2);
            }
            other => panic!("expected Match, got {:?}", other),
        }
    }

    // ================================================================
    // Collections
    // ================================================================

    #[test]
    fn array_literal() {
        let body = lower_func_body("func f() { [1, 2, 3]; }");
        if let AstStmt::Expr { expr, .. } = &body.stmts[body.statements[0]] {
            match &body.exprs[*expr] {
                AstExpr::Array { elements, .. } => assert_eq!(elements.len(), 3),
                other => panic!("expected Array, got {:?}", other),
            }
        }
    }

    #[test]
    fn tuple_literal() {
        let body = lower_func_body("func f() { (1, 2); }");
        if let AstStmt::Expr { expr, .. } = &body.stmts[body.statements[0]] {
            match &body.exprs[*expr] {
                AstExpr::Tuple { elements, .. } => assert_eq!(elements.len(), 2),
                other => panic!("expected Tuple, got {:?}", other),
            }
        }
    }

    // ================================================================
    // Assignment
    // ================================================================

    #[test]
    fn assignment() {
        let body = lower_func_body("func f() { x = 42; }");
        if let AstStmt::Expr { expr, .. } = &body.stmts[body.statements[0]] {
            assert!(matches!(&body.exprs[*expr], AstExpr::Assignment { .. }));
        }
    }

    // ================================================================
    // Implicit member
    // ================================================================

    #[test]
    fn implicit_member() {
        let body = lower_func_body("func f() { .None; }");
        if let AstStmt::Expr { expr, .. } = &body.stmts[body.statements[0]] {
            match &body.exprs[*expr] {
                AstExpr::ImplicitMember { member, arguments, .. } => {
                    assert_eq!(member, "None");
                    assert!(arguments.is_none());
                }
                other => panic!("expected ImplicitMember, got {:?}", other),
            }
        }
    }

    // ================================================================
    // Tail expression
    // ================================================================

    #[test]
    fn tail_expression() {
        let body = lower_func_body("func f() { 42 }");
        assert!(body.tail_expr.is_some(), "should have tail expression");
        assert!(body.statements.is_empty(), "no statements");
        match &body.exprs[body.tail_expr.unwrap()] {
            AstExpr::Literal { kind: AstLiteral::Integer(v), .. } => assert_eq!(v, "42"),
            other => panic!("expected Integer, got {:?}", other),
        }
    }

    // ================================================================
    // Field defaults
    // ================================================================

    #[test]
    fn field_default_value() {
        let mut world = World::new();
        world.begin_revision();
        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".to_string()));
        let file = world.spawn();

        let source = "module M\nstruct S { var x: Int64 = 42 }";
        let tokens: Vec<_> = kestrel_lexer2::lex(source, file.index())
            .filter_map(|r| r.ok())
            .collect();
        let result = kestrel_parser2::parse_source_file_from_source(
            source,
            tokens.iter().map(|t| (t.value.clone(), t.span.clone())),
        );
        build_declarations(&mut world, file, &result.tree, root, None);

        let body = find_body(&world, root).expect("field should have Body");
        assert!(body.tail_expr.is_some(), "default value should be tail expr");
        match &body.exprs[body.tail_expr.unwrap()] {
            AstExpr::Literal { kind: AstLiteral::Integer(v), .. } => assert_eq!(v, "42"),
            other => panic!("expected Integer literal, got {:?}", other),
        }
    }

    // ================================================================
    // Integration: real stdlib file
    // ================================================================

    #[test]
    fn ordering_ks_bodies_lowered() {
        let source = include_str!("../../../lang/std/core/ordering.ks");
        let mut world = World::new();
        world.begin_revision();
        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".to_string()));
        let file = world.spawn();

        let tokens: Vec<_> = kestrel_lexer2::lex(source, file.index())
            .filter_map(|r| r.ok())
            .collect();
        let result = kestrel_parser2::parse_source_file_from_source(
            source,
            tokens.iter().map(|t| (t.value.clone(), t.span.clone())),
        );
        build_declarations(&mut world, file, &result.tree, root, None);

        // Count entities with Body component
        let mut body_count = 0;
        count_bodies(&world, root, &mut body_count);
        assert!(body_count >= 6, "ordering.ks has 6 methods, got {} bodies", body_count);

        // Verify at least one body has non-trivial content (match expression)
        let body = find_body(&world, root).unwrap();
        assert!(!body.exprs.is_empty(), "body should have expressions");
    }

    fn count_bodies(world: &World, entity: kestrel_hecs::Entity, count: &mut usize) {
        if world.has::<Body>(entity) {
            *count += 1;
        }
        for &child in world.children_of(entity) {
            count_bodies(world, child, count);
        }
    }

    // ================================================================
    // Postfix / Try / Throw
    // ================================================================

    #[test]
    fn postfix_unwrap() {
        let body = lower_func_body("func f() { x!; }");
        if let AstStmt::Expr { expr, .. } = &body.stmts[body.statements[0]] {
            match &body.exprs[*expr] {
                AstExpr::Postfix { op, .. } => assert_eq!(*op, PostfixOp::Unwrap),
                other => panic!("expected Postfix, got {:?}", other),
            }
        }
    }

    // ================================================================
    // Pretty-print: visual AST dump
    // ================================================================

    #[test]
    fn pretty_print_ordering_ks() {
        use kestrel_ast::pretty::pretty_print;

        let source = include_str!("../../../lang/std/core/ordering.ks");
        let mut world = World::new();
        world.begin_revision();
        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".to_string()));
        let file = world.spawn();

        let tokens: Vec<_> = kestrel_lexer2::lex(source, file.index())
            .filter_map(|r| r.ok())
            .collect();
        let result = kestrel_parser2::parse_source_file_from_source(
            source,
            tokens.iter().map(|t| (t.value.clone(), t.span.clone())),
        );
        build_declarations(&mut world, file, &result.tree, root, None);

        // Collect all (name, body) pairs
        let mut bodies = Vec::new();
        collect_named_bodies(&world, root, &mut bodies);

        for (name, body) in &bodies {
            eprintln!("=== {name} ===");
            eprintln!("{}", pretty_print(body));
            eprintln!();
        }

        // Just verify we found some bodies
        assert!(!bodies.is_empty());
    }

    fn collect_named_bodies(
        world: &World,
        entity: kestrel_hecs::Entity,
        out: &mut Vec<(String, AstBody)>,
    ) {
        if let Some(body) = world.get::<Body>(entity) {
            let name = world.get::<Name>(entity)
                .map(|n| n.0.clone())
                .unwrap_or_else(|| "<anon>".to_string());
            out.push((name, body.0.clone()));
        }
        for &child in world.children_of(entity) {
            collect_named_bodies(world, child, out);
        }
    }
}

/// Check if a closure body (the CST node for ExprClosure) references `it` as
/// an identifier in expression position. Does NOT descend into nested closures —
/// an `it` inside `{ list.map { it } }` belongs to the inner closure only.
fn closure_body_references_it(node: &SyntaxNode) -> bool {
    fn walk(node: &SyntaxNode) -> bool {
        for child in node.children_with_tokens() {
            match &child {
                rowan::NodeOrToken::Token(token) => {
                    if token.kind() == SyntaxKind::Identifier && token.text() == "it" {
                        return true;
                    }
                }
                rowan::NodeOrToken::Node(child_node) => {
                    // Don't descend into nested closures — their `it` is their own
                    if child_node.kind() == SyntaxKind::ExprClosure {
                        continue;
                    }
                    // Skip ClosureParams (the explicit param list, not the body)
                    if child_node.kind() == SyntaxKind::ClosureParams {
                        continue;
                    }
                    if walk(child_node) {
                        return true;
                    }
                }
            }
        }
        false
    }
    walk(node)
}
