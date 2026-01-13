//! ExecutableBehavior for function bodies.
//!
//! This behavior is attached to functions during the bind phase and contains
//! the resolved code block (statements and yield expression).

use semantic_tree::behavior::Behavior;

use crate::behavior::KestrelBehaviorKind;
use crate::expr::Expression;
use crate::language::KestrelLanguage;
use crate::stmt::Statement;

/// A code block containing statements and an optional yield expression.
///
/// Code blocks are the bodies of functions, if/else branches, loops, etc.
/// They contain a sequence of statements and optionally yield a value.
#[derive(Debug, Clone)]
pub struct CodeBlock {
    /// The statements in this block
    pub statements: Vec<Statement>,
    /// The optional yield expression (the value this block evaluates to)
    pub yield_expr: Option<Box<Expression>>,
}

impl CodeBlock {
    /// Create a new code block.
    pub fn new(statements: Vec<Statement>, yield_expr: Option<Expression>) -> Self {
        CodeBlock {
            statements,
            yield_expr: yield_expr.map(Box::new),
        }
    }

    /// Create an empty code block.
    pub fn empty() -> Self {
        CodeBlock {
            statements: Vec::new(),
            yield_expr: None,
        }
    }

    /// Check if this block is empty (no statements and no yield).
    pub fn is_empty(&self) -> bool {
        self.statements.is_empty() && self.yield_expr.is_none()
    }

    /// Get the yield expression if present.
    pub fn yield_expr(&self) -> Option<&Expression> {
        self.yield_expr.as_deref()
    }
}

/// Behavior indicating that a symbol has an executable body.
///
/// This is attached to functions during the bind phase after their
/// bodies have been resolved.
#[derive(Debug, Clone)]
pub struct ExecutableBehavior {
    /// The resolved code block
    body: CodeBlock,
}

impl Behavior<KestrelLanguage> for ExecutableBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::Executable
    }
}

impl ExecutableBehavior {
    /// Create a new ExecutableBehavior with the given body.
    pub fn new(body: CodeBlock) -> Self {
        ExecutableBehavior { body }
    }

    /// Get the code block body.
    pub fn body(&self) -> &CodeBlock {
        &self.body
    }

    /// Get a mutable reference to the code block body.
    pub fn body_mut(&mut self) -> &mut CodeBlock {
        &mut self.body
    }
}

/// Behavior indicating that a symbol has a type-resolved executable body.
///
/// This is attached to functions after type inference has completed.
/// The body has all `TyKind::Infer` placeholders resolved to concrete types.
#[derive(Debug, Clone)]
pub struct ResolvedExecutableBehavior {
    /// The type-resolved code block
    body: CodeBlock,
}

impl Behavior<KestrelLanguage> for ResolvedExecutableBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::ResolvedExecutable
    }
}

impl ResolvedExecutableBehavior {
    /// Create a new ResolvedExecutableBehavior with the given body.
    pub fn new(body: CodeBlock) -> Self {
        ResolvedExecutableBehavior { body }
    }

    /// Get the code block body.
    pub fn body(&self) -> &CodeBlock {
        &self.body
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::Expression;
    use crate::pattern::{Mutability, Pattern};
    use crate::stmt::Statement;
    use crate::symbol::local::LocalId;
    use crate::ty::{IntBits, Ty};
    use kestrel_span::Span;

    #[test]
    fn test_empty_code_block() {
        let block = CodeBlock::empty();
        assert!(block.is_empty());
        assert!(block.yield_expr().is_none());
    }

    #[test]
    fn test_code_block_with_statements() {
        let pattern = Pattern::local(
            LocalId(0),
            Mutability::Immutable,
            "x".to_string(),
            Ty::int(IntBits::I64, Span::new(0, 10..12)),
            Span::new(0, 0..1),
        );
        let init = Expression::integer(42, Span::new(0, 10..12));
        let stmt = Statement::binding(pattern, Some(init), Span::new(0, 0..13));

        let block = CodeBlock::new(vec![stmt], None);
        assert!(!block.is_empty());
        assert_eq!(block.statements.len(), 1);
    }

    #[test]
    fn test_code_block_with_yield() {
        let yield_expr = Expression::integer(42, Span::new(0, 0..2));
        let block = CodeBlock::new(vec![], Some(yield_expr));

        assert!(!block.is_empty());
        assert!(block.yield_expr().is_some());
    }

    #[test]
    fn test_executable_behavior() {
        let block = CodeBlock::empty();
        let behavior = ExecutableBehavior::new(block);

        assert!(behavior.body().is_empty());
        assert_eq!(behavior.kind(), KestrelBehaviorKind::Executable);
    }
}
