//! Pattern lowering - converts semantic patterns to MIR assignments.
//!
//! Patterns are used in let bindings and match expressions. This module handles
//! lowering them by generating the appropriate local assignments.

use kestrel_execution_graph::{Place, Value};
use kestrel_semantic_tree::pattern::{Pattern, PatternKind};

use crate::context::LoweringContext;
use crate::error::LoweringError;


/// Lower a pattern, assigning the value to the appropriate places.
///
/// For simple bindings like `let x = value`, this creates the local and assigns the value.
/// For more complex patterns like tuple destructuring, this generates multiple assignments.
///
/// # Arguments
///
/// * `ctx` - The lowering context
/// * `pattern` - The pattern to lower
/// * `value` - The value to assign to the pattern bindings
pub fn lower_pattern(ctx: &mut LoweringContext, pattern: &Pattern, value: Value) {
    match &pattern.kind {
        PatternKind::Local {
            local_id,
            mutability: _,
            name: _,
        } => {
            // Simple local binding - assign the value to the local
            let mir_local = ctx.get_local_unwrap(*local_id);
            ctx.emit_assign_value(Place::local(mir_local), value);
        }

        PatternKind::Wildcard => {
            // Wildcard pattern - discard the value, nothing to do
            // The value is simply not used
        }

        PatternKind::Tuple {
            prefix,
            has_rest: _,
            suffix: _,
        } => {
            // TODO: Tuple destructuring
            // For now, emit an error and continue
            ctx.emit_error(LoweringError::unsupported_pattern(
                "Tuple destructuring",
                pattern.span.clone(),
            ));

            // Still try to lower sub-patterns to create their locals
            // This helps with error recovery
            if let Value::Place(tuple_place) = value {
                for (i, sub_pattern) in prefix.iter().enumerate() {
                    let element_place = tuple_place.clone().index(i);
                    lower_pattern(ctx, sub_pattern, Value::Place(element_place));
                }
                // TODO: Handle rest pattern and suffix
            }
        }

        PatternKind::Struct {
            struct_id: _,
            struct_name,
            fields,
            has_rest: _,
        } => {
            // TODO: Struct destructuring
            ctx.emit_error(LoweringError::unsupported_pattern(
                format!("Struct pattern '{}'", struct_name),
                pattern.span.clone(),
            ));

            // Still try to lower field patterns for error recovery
            if let Value::Place(struct_place) = value {
                for field in fields {
                    let field_place = struct_place.clone().field(&field.field_name);
                    lower_pattern(ctx, &field.pattern, Value::Place(field_place));
                }
            }
        }

        PatternKind::EnumVariant {
            case_id: _,
            case_name,
            bindings: _,
        } => {
            // TODO: Enum variant pattern matching
            // This requires switch statements and downcast projections
            ctx.emit_error(LoweringError::unsupported_pattern(
                format!("Enum variant pattern '.{}'", case_name),
                pattern.span.clone(),
            ));
        }

        PatternKind::Literal { value: lit_value } => {
            // TODO: Literal patterns require comparison and branching
            // Only valid in match expressions, not let bindings
            ctx.emit_error(LoweringError::unsupported_pattern(
                format!("Literal pattern '{:?}'", lit_value),
                pattern.span.clone(),
            ));
        }

        PatternKind::Range { .. } => {
            // TODO: Range patterns require comparison logic
            ctx.emit_error(LoweringError::unsupported_pattern(
                "Range pattern",
                pattern.span.clone(),
            ));
        }

        PatternKind::Array { .. } => {
            // TODO: Array patterns
            ctx.emit_error(LoweringError::unsupported_pattern(
                "Array pattern",
                pattern.span.clone(),
            ));
        }

        PatternKind::Or { alternatives: _ } => {
            // TODO: Or patterns require generating multiple match branches
            ctx.emit_error(LoweringError::unsupported_pattern(
                "Or pattern",
                pattern.span.clone(),
            ));
        }

        PatternKind::At {
            name,
            local_id,
            mutability: _,
            subpattern: _,
        } => {
            // TODO: @ patterns bind the whole value and also match subpattern
            ctx.emit_error(LoweringError::unsupported_pattern(
                format!("{} @ pattern", name),
                pattern.span.clone(),
            ));

            // Still assign to the local for error recovery
            let mir_local = ctx.get_local_unwrap(*local_id);
            ctx.emit_assign_value(Place::local(mir_local), value.clone());
        }

        PatternKind::Rest => {
            // Rest pattern by itself shouldn't appear at top level
            // It's handled as part of tuple/array patterns
            ctx.emit_error(LoweringError::unsupported_pattern(
                "Rest pattern",
                pattern.span.clone(),
            ));
        }

        PatternKind::Error => {
            // Error pattern - skip (error already reported)
        }
    }
}

/// Check if a pattern is irrefutable (always matches).
///
/// Irrefutable patterns are used in let bindings. Refutable patterns
/// require match expressions or if-let.
#[allow(dead_code)]
pub fn is_irrefutable(pattern: &Pattern) -> bool {
    match &pattern.kind {
        PatternKind::Local { .. } => true,
        PatternKind::Wildcard => true,
        PatternKind::Tuple {
            prefix,
            has_rest: _,
            suffix,
        } => {
            prefix.iter().all(is_irrefutable) && suffix.iter().all(is_irrefutable)
        }
        PatternKind::Struct { fields, .. } => {
            fields.iter().all(|f| is_irrefutable(&f.pattern))
        }
        PatternKind::At { subpattern, .. } => is_irrefutable(subpattern),
        PatternKind::Rest => true,
        PatternKind::Error => true,

        // Refutable patterns
        PatternKind::EnumVariant { .. } => false,
        PatternKind::Literal { .. } => false,
        PatternKind::Range { .. } => false,
        PatternKind::Array { .. } => false, // Could be irrefutable in some cases
        PatternKind::Or { .. } => false,
    }
}
