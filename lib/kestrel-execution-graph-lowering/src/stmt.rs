//! Statement lowering - converts semantic statements to MIR.

use kestrel_semantic_tree::stmt::{Statement, StatementKind};

use crate::context::LoweringContext;
use crate::error::LoweringError;
use crate::expr::lower_expression;
use crate::pattern::lower_pattern;

/// Lower a statement to MIR.
///
/// This handles let/var bindings and expression statements.
/// The generated MIR statements are added to the current block.
pub fn lower_statement(ctx: &mut LoweringContext, stmt: &Statement) {
    // Don't process if block is already terminated
    if ctx.is_block_terminated() {
        return;
    }

    match &stmt.kind {
        StatementKind::Binding { pattern, value } => {
            // Lower the value expression first
            if let Some(init_expr) = value {
                let init_value = lower_expression(ctx, init_expr);

                // If block got terminated during expression lowering (e.g., return),
                // don't try to do the pattern binding
                if !ctx.is_block_terminated() {
                    lower_pattern(ctx, pattern, init_value);
                }
            } else {
                // No initializer - the local is created but uninitialized
                // In MIR, we don't need to do anything here - the local was
                // already created during function setup
                // 
                // TODO: Consider emitting an undef or zero initialization
            }
        }

        StatementKind::Expr(expr) => {
            // Lower the expression for its side effects
            // The result is discarded
            let _value = lower_expression(ctx, expr);
        }

        StatementKind::GuardLet {
            conditions: _,
            else_block: _,
        } => {
            // TODO: Guard let requires control flow for the else block
            ctx.emit_error(LoweringError::unsupported_stmt(
                "guard let",
                stmt.span.clone(),
            ));
        }
    }
}
