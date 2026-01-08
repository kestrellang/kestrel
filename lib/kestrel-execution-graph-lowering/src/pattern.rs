//! Pattern lowering - converts semantic patterns to MIR assignments.
//!
//! Patterns are used in let bindings and match expressions. This module handles
//! lowering them by generating the appropriate local assignments.
//!
//! # Irrefutable vs Refutable Patterns
//!
//! This module handles *irrefutable* patterns - patterns that always match.
//! These are used in `let` and `var` bindings:
//! - `let x = value` (binding)
//! - `let (a, b) = tuple` (tuple destructuring)
//! - `let Point { x, y } = point` (struct destructuring)
//! - `let _ = value` (wildcard)
//!
//! Refutable patterns (enum variants, literals, ranges) are handled by
//! match/if-let/guard-let lowering, which uses decision trees.

use kestrel_execution_graph::{Place, Rvalue, Value};
use kestrel_semantic_tree::pattern::{
    EnumPatternBinding, Pattern, PatternKind, StructPatternField,
};

use crate::context::LoweringContext;
use crate::error::LoweringError;
use crate::ty::lower_type;

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
            name,
        } => {
            // Simple local binding - assign the value to the local
            // If the local doesn't exist yet (e.g., in closure bodies), create it
            let mir_local = if let Some(existing) = ctx.get_local(*local_id) {
                existing
            } else {
                // Create the local on demand (happens in closure bodies)
                let local_ty = lower_type(ctx, &pattern.ty);
                let new_local = ctx.create_local(name, local_ty);
                ctx.map_local(*local_id, new_local);
                new_local
            };
            ctx.emit_assign_value(Place::local(mir_local), value);
        }

        PatternKind::Wildcard => {
            // Wildcard pattern - discard the value, nothing to do
            // The value is simply not used
        }

        PatternKind::Tuple {
            prefix,
            has_rest,
            suffix,
        } => {
            lower_tuple_pattern(ctx, prefix, *has_rest, suffix, value, pattern);
        }

        PatternKind::Struct {
            struct_id: _,
            struct_name: _,
            fields,
            has_rest: _,
        } => {
            lower_struct_pattern(ctx, fields, value, pattern);
        }

        PatternKind::EnumVariant {
            case_id: _,
            case_name,
            bindings,
        } => {
            // For single-variant enums, this is irrefutable
            // We need to downcast and extract bindings
            lower_enum_variant_pattern(ctx, case_name, bindings, value, pattern);
        }

        PatternKind::Literal { value: lit_value } => {
            // Literal patterns in let bindings are refutable (error caught by semantic analysis)
            // We emit an error here as a safety measure
            ctx.emit_error(LoweringError::unsupported_pattern(
                format!("Literal pattern '{:?}' in let binding", lit_value),
                pattern.span.clone(),
            ));
        }

        PatternKind::Range { .. } => {
            // Range patterns are refutable
            ctx.emit_error(LoweringError::unsupported_pattern(
                "Range pattern in let binding",
                pattern.span.clone(),
            ));
        }

        PatternKind::Array {
            prefix,
            rest: _,
            suffix,
        } => {
            lower_array_pattern(ctx, prefix, suffix, value, pattern);
        }

        PatternKind::Or { alternatives: _ } => {
            // Or patterns in let bindings - should be caught by semantic analysis
            ctx.emit_error(LoweringError::unsupported_pattern(
                "Or pattern in let binding",
                pattern.span.clone(),
            ));
        }

        PatternKind::At {
            name: _,
            local_id,
            mutability: _,
            subpattern,
        } => {
            // @ pattern: bind the whole value AND match subpattern
            lower_at_pattern(ctx, *local_id, subpattern, value, pattern);
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

/// Lower a tuple destructuring pattern.
///
/// `let (a, b, c) = tuple` becomes:
/// - a = tuple.0
/// - b = tuple.1
/// - c = tuple.2
fn lower_tuple_pattern(
    ctx: &mut LoweringContext,
    prefix: &[Pattern],
    has_rest: bool,
    suffix: &[Pattern],
    value: Value,
    pattern: &Pattern,
) {
    // We need the value in a place to project from
    let tuple_place = ensure_place(ctx, value, &pattern.ty, "tuple");

    // Lower prefix elements (from the start)
    for (i, sub_pattern) in prefix.iter().enumerate() {
        let element_place = tuple_place.clone().index(i);
        lower_pattern(ctx, sub_pattern, Value::Place(element_place));
    }

    // If there's a rest pattern and suffix, we need to handle them
    // For now, we only support patterns without rest or with empty suffix
    if has_rest && !suffix.is_empty() {
        // Need to compute suffix indices from the end
        // For a tuple of length N with suffix of length S:
        // suffix[0] is at index N - S
        // suffix[1] is at index N - S + 1
        // etc.
        //
        // We need to know the tuple length at compile time, which we can get from the type
        if let kestrel_semantic_tree::ty::TyKind::Tuple(element_types) = pattern.ty.kind() {
            let tuple_len = element_types.len();
            let suffix_start = tuple_len - suffix.len();

            for (i, sub_pattern) in suffix.iter().enumerate() {
                let element_place = tuple_place.clone().index(suffix_start + i);
                lower_pattern(ctx, sub_pattern, Value::Place(element_place));
            }
        }
    }
}

/// Lower a struct destructuring pattern.
///
/// `let Point { x, y } = point` becomes:
/// - x = point.x
/// - y = point.y
fn lower_struct_pattern(
    ctx: &mut LoweringContext,
    fields: &[StructPatternField],
    value: Value,
    pattern: &Pattern,
) {
    // We need the value in a place to project from
    let struct_place = ensure_place(ctx, value, &pattern.ty, "struct");

    // Lower each field pattern
    for field in fields {
        let field_place = struct_place.clone().field(&field.field_name);
        lower_pattern(ctx, &field.pattern, Value::Place(field_place));
    }
}

/// Lower an enum variant pattern (for single-variant enums, which are irrefutable).
///
/// `let .Value(inner) = wrapper` becomes:
/// - inner = wrapper.downcast(Value).0
fn lower_enum_variant_pattern(
    ctx: &mut LoweringContext,
    case_name: &str,
    bindings: &[EnumPatternBinding],
    value: Value,
    pattern: &Pattern,
) {
    // We need the value in a place to downcast from
    let enum_place = ensure_place(ctx, value, &pattern.ty, "enum");

    // Downcast to the variant
    let variant_place = enum_place.downcast(case_name);

    // Lower each binding (associated values)
    for (i, binding) in bindings.iter().enumerate() {
        let binding_place = variant_place.clone().index(i);
        lower_pattern(ctx, &binding.pattern, Value::Place(binding_place));
    }
}

/// Lower an array pattern.
///
/// `let [a, b, c] = array` becomes:
/// - a = array[0]
/// - b = array[1]
/// - c = array[2]
fn lower_array_pattern(
    ctx: &mut LoweringContext,
    prefix: &[Pattern],
    suffix: &[Pattern],
    value: Value,
    pattern: &Pattern,
) {
    // We need the value in a place to index from
    let array_place = ensure_place(ctx, value, &pattern.ty, "array");

    // Lower prefix elements
    for (i, sub_pattern) in prefix.iter().enumerate() {
        let element_place = array_place.clone().index(i);
        lower_pattern(ctx, sub_pattern, Value::Place(element_place));
    }

    // For suffix elements, we'd need runtime length computation
    // For now, only support patterns without suffix or where array length is known
    if !suffix.is_empty() {
        ctx.emit_error(LoweringError::unsupported_pattern(
            "Array pattern with suffix elements",
            pattern.span.clone(),
        ));
    }
}

/// Lower an @ pattern.
///
/// `let x @ (a, b) = tuple` becomes:
/// - x = tuple (the whole value)
/// - a = tuple.0
/// - b = tuple.1
fn lower_at_pattern(
    ctx: &mut LoweringContext,
    local_id: kestrel_semantic_tree::symbol::local::LocalId,
    subpattern: &Pattern,
    value: Value,
    pattern: &Pattern,
) {
    // First, we need the value in a place (we might need to copy it)
    let value_place = ensure_place(ctx, value, &pattern.ty, "at_value");

    // Bind the whole value to the local
    let mir_local = ctx.get_local_unwrap(local_id);
    ctx.emit_copy(Place::local(mir_local), value_place.clone());

    // Then lower the subpattern with the same value
    lower_pattern(ctx, subpattern, Value::Place(value_place));
}

/// Ensure a value is in a place (not an immediate).
///
/// If the value is already a place, return it.
/// If it's an immediate, store it in a temporary and return that place.
fn ensure_place(
    ctx: &mut LoweringContext,
    value: Value,
    ty: &kestrel_semantic_tree::ty::Ty,
    name_hint: &str,
) -> Place {
    match value {
        Value::Place(p) => p,
        Value::Immediate(imm) => {
            // Store the immediate in a temporary
            let mir_ty = lower_type(ctx, ty);
            let temp_local = ctx.create_temp(name_hint, mir_ty);
            let place = Place::local(temp_local);
            ctx.emit_assign(place.clone(), Rvalue::Use(imm));
            place
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
        } => prefix.iter().all(is_irrefutable) && suffix.iter().all(is_irrefutable),
        PatternKind::Struct { fields, .. } => fields.iter().all(|f| is_irrefutable(&f.pattern)),
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
