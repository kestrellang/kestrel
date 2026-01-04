//! Statement data types for the semantic tree.
//!
//! Statements are plain data structures (not symbols) that represent
//! resolved statements in function bodies. They are created during
//! the bind phase.

use kestrel_span::Span;

use crate::behavior::executable::CodeBlock;
use crate::expr::{Expression, IfCondition};
use crate::pattern::Pattern;
use crate::symbol::local::LocalId;

/// Represents the kind of statement.
#[derive(Debug, Clone)]
pub enum StatementKind {
    /// Variable binding: `let x: Int = 42;` or `var x: Int = 42;`
    Binding {
        /// The pattern being bound (includes mutability, local ID, name)
        pattern: Pattern,
        /// Optional initializer expression
        value: Option<Expression>,
    },
    /// Expression statement: `foo();`
    Expr(Expression),
    /// Guard-let statement: `guard let pattern = expr, ... else { block }`
    ///
    /// Supports chains: `guard let .Some(x) = a, let .Some(y) = b, x > 0 else { ... }`
    /// Pattern bindings are visible AFTER the guard statement, but NOT in the else block.
    /// The else block MUST diverge (return, break, continue, or panic).
    GuardLet {
        /// The conditions to check (at least one must be a Let condition)
        conditions: Vec<IfCondition>,
        /// The else block (must diverge)
        else_block: CodeBlock,
    },
    /// Deinit statement: `deinit x;`
    ///
    /// Explicitly runs the destructor for a variable and marks it as moved.
    /// The variable cannot be used after this point.
    Deinit {
        /// The local variable being deinited
        local_id: LocalId,
        /// The name of the variable (for diagnostics)
        name: String,
    },
}

/// A resolved statement in the semantic tree.
///
/// Unlike symbols, statements are plain data structures without SymbolId.
/// They are created during the bind phase.
#[derive(Debug, Clone)]
pub struct Statement {
    /// The kind of statement
    pub kind: StatementKind,
    /// The source span of this statement
    pub span: Span,
}

impl Statement {
    /// Create a new statement.
    pub fn new(kind: StatementKind, span: Span) -> Self {
        Statement { kind, span }
    }

    /// Create a binding statement.
    pub fn binding(pattern: Pattern, value: Option<Expression>, span: Span) -> Self {
        Statement {
            kind: StatementKind::Binding { pattern, value },
            span,
        }
    }

    /// Create an expression statement.
    pub fn expr(expr: Expression, span: Span) -> Self {
        Statement {
            kind: StatementKind::Expr(expr),
            span,
        }
    }

    /// Create a guard-let statement.
    pub fn guard_let(conditions: Vec<IfCondition>, else_block: CodeBlock, span: Span) -> Self {
        Statement {
            kind: StatementKind::GuardLet {
                conditions,
                else_block,
            },
            span,
        }
    }

    /// Create a deinit statement.
    pub fn deinit(local_id: LocalId, name: String, span: Span) -> Self {
        Statement {
            kind: StatementKind::Deinit { local_id, name },
            span,
        }
    }

    /// Check if this is a binding statement.
    pub fn is_binding(&self) -> bool {
        matches!(self.kind, StatementKind::Binding { .. })
    }

    /// Check if this is an expression statement.
    pub fn is_expr(&self) -> bool {
        matches!(self.kind, StatementKind::Expr(_))
    }

    /// Check if this is a guard-let statement.
    pub fn is_guard_let(&self) -> bool {
        matches!(self.kind, StatementKind::GuardLet { .. })
    }

    /// Check if this is a deinit statement.
    pub fn is_deinit(&self) -> bool {
        matches!(self.kind, StatementKind::Deinit { .. })
    }

    /// Get the pattern if this is a binding statement.
    pub fn pattern(&self) -> Option<&Pattern> {
        match &self.kind {
            StatementKind::Binding { pattern, .. } => Some(pattern),
            _ => None,
        }
    }

    /// Get the value if this is a binding statement.
    pub fn value(&self) -> Option<&Expression> {
        match &self.kind {
            StatementKind::Binding { value, .. } => value.as_ref(),
            _ => None,
        }
    }

    /// Get the expression if this is an expression statement.
    pub fn as_expr(&self) -> Option<&Expression> {
        match &self.kind {
            StatementKind::Expr(expr) => Some(expr),
            _ => None,
        }
    }

    /// Return a compact debug representation of this statement.
    pub fn debug_compact(&self) -> String {
        use crate::pattern::{Mutability, PatternKind};

        match &self.kind {
            StatementKind::Binding { pattern, value } => {
                let keyword = match &pattern.kind {
                    PatternKind::Local { mutability, .. } => {
                        if *mutability == Mutability::Mutable {
                            "var"
                        } else {
                            "let"
                        }
                    }
                    PatternKind::At { mutability, .. } => {
                        if *mutability == Mutability::Mutable {
                            "var"
                        } else {
                            "let"
                        }
                    }
                    PatternKind::Wildcard
                    | PatternKind::Tuple { .. }
                    | PatternKind::Literal { .. }
                    | PatternKind::EnumVariant { .. }
                    | PatternKind::Range { .. }
                    | PatternKind::Struct { .. }
                    | PatternKind::Array { .. }
                    | PatternKind::Or { .. }
                    | PatternKind::Rest
                    | PatternKind::Error => "let",
                };
                let name = pattern.name().unwrap_or("<error>");
                let value_str = value
                    .as_ref()
                    .map(|v| format!(" = {}", v.debug_compact()))
                    .unwrap_or_default();
                format!("{} {}{};", keyword, name, value_str)
            }
            StatementKind::Expr(expr) => format!("{};", expr.debug_compact()),
            StatementKind::GuardLet { conditions, .. } => {
                let conds: Vec<_> = conditions.iter().map(|c| match c {
                    IfCondition::Let { pattern, value, .. } => {
                        let name = pattern.name().unwrap_or("<pattern>");
                        format!("let {} = {}", name, value.debug_compact())
                    }
                    IfCondition::Expr(e) => e.debug_compact(),
                }).collect();
                format!("guard {} else {{ ... }}", conds.join(", "))
            }
            StatementKind::Deinit { name, .. } => {
                format!("deinit {};", name)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::Expression;
    use crate::pattern::{Mutability, Pattern};
    use crate::symbol::local::LocalId;
    use kestrel_span::Span;

    #[test]
    fn test_binding_statement() {
        let pattern = Pattern::local(
            LocalId(0),
            Mutability::Immutable,
            "x".to_string(),
            crate::ty::Ty::int(crate::ty::IntBits::I64, Span::from(5..8)),
            Span::from(0..8),
        );
        let value = Expression::integer(42, Span::from(11..13));
        let stmt = Statement::binding(pattern, Some(value), Span::from(0..14));

        assert!(stmt.is_binding());
        assert!(!stmt.is_expr());
        assert!(stmt.pattern().is_some());
        assert!(stmt.value().is_some());
    }

    #[test]
    fn test_expr_statement() {
        let expr = Expression::unit(Span::from(0..2));
        let stmt = Statement::expr(expr, Span::from(0..3));

        assert!(stmt.is_expr());
        assert!(!stmt.is_binding());
        assert!(stmt.as_expr().is_some());
    }
}
