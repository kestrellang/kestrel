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

use kestrel_execution_graph::{
    BinOp, CallArg, Callee, Id, Place, QualifiedName, QualifiedNameData, Rvalue, Ty as MirTyMarker,
    Value,
};
use kestrel_semantic_model::SymbolFor;
use kestrel_semantic_tree::pattern::{
    EnumPatternBinding, Pattern, PatternKind, StructPatternField,
};
use kestrel_semantic_tree::symbol::local::LocalId;
use kestrel_semantic_tree::ty::IntBits;

use crate::context::LoweringContext;
use crate::error::LoweringError;
use crate::name::qualified_name_for_symbol;
use crate::ty::{lower_type, make_int_immediate};

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
            // Use copy for copyable types, move for non-copyable types
            ctx.emit_copy_or_move_value(Place::local(mir_local), value, &pattern.ty);
        },

        PatternKind::Wildcard => {
            // Wildcard pattern - discard the value, nothing to do
            // The value is simply not used
        },

        PatternKind::Tuple {
            prefix,
            has_rest,
            suffix,
        } => {
            lower_tuple_pattern(ctx, prefix, *has_rest, suffix, value, pattern);
        },

        PatternKind::Struct {
            struct_id: _,
            struct_name: _,
            fields,
            has_rest: _,
        } => {
            lower_struct_pattern(ctx, fields, value, pattern);
        },

        PatternKind::EnumVariant {
            case_id: _,
            case_name,
            bindings,
        } => {
            // For single-variant enums, this is irrefutable
            // We need to downcast and extract bindings
            lower_enum_variant_pattern(ctx, case_name, bindings, value, pattern);
        },

        PatternKind::Literal { value: lit_value } => {
            // Literal patterns in let bindings are refutable (error caught by semantic analysis)
            // We emit an error here as a safety measure
            ctx.emit_error(LoweringError::unsupported_pattern(
                format!("Literal pattern '{:?}' in let binding", lit_value),
                pattern.span.clone(),
            ));
        },

        PatternKind::Range { .. } => {
            // Range patterns are refutable
            ctx.emit_error(LoweringError::unsupported_pattern(
                "Range pattern in let binding",
                pattern.span.clone(),
            ));
        },

        PatternKind::Array {
            prefix,
            rest,
            suffix,
        } => {
            lower_array_pattern(ctx, prefix, rest, suffix, value, pattern);
        },

        PatternKind::Or { alternatives: _ } => {
            // Or patterns in let bindings - should be caught by semantic analysis
            ctx.emit_error(LoweringError::unsupported_pattern(
                "Or pattern in let binding",
                pattern.span.clone(),
            ));
        },

        PatternKind::At {
            name: _,
            local_id,
            mutability: _,
            subpattern,
        } => {
            // @ pattern: bind the whole value AND match subpattern
            lower_at_pattern(ctx, *local_id, subpattern, value, pattern);
        },

        PatternKind::Rest => {
            // Rest pattern by itself shouldn't appear at top level
            // It's handled as part of tuple/array patterns
            ctx.emit_error(LoweringError::unsupported_pattern(
                "Rest pattern",
                pattern.span.clone(),
            ));
        },

        PatternKind::Error => {
            // Error pattern - skip (error already reported)
        },
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
///
/// `let [a, ..rest, z] = array` becomes:
/// - a = array.matchGet(0)
/// - len = array.matchLength()
/// - z = array.matchGet(len - 1)
/// - rest = array.matchSlice(1, len - 1)
fn lower_array_pattern(
    ctx: &mut LoweringContext,
    prefix: &[Pattern],
    rest: &Option<(Option<String>, Option<LocalId>)>,
    suffix: &[Pattern],
    value: Value,
    pattern: &Pattern,
) {
    // We need the value in a place to index from
    let array_place = ensure_place(ctx, value, &pattern.ty, "array");

    let needs_length = !suffix.is_empty() || rest.is_some();

    if !needs_length {
        // Simple case: prefix-only pattern, use direct indexing
        for (i, sub_pattern) in prefix.iter().enumerate() {
            let element_place = array_place.clone().index(i);
            lower_pattern(ctx, sub_pattern, Value::Place(element_place));
        }
        return;
    }

    // Complex case: we need length computation for suffix/rest
    // Get the ArrayMatchable protocol name
    let array_matchable_protocol_name = get_array_matchable_protocol_name(ctx);

    // Get the type for the witness call
    let for_type = lower_type(ctx, &pattern.ty);
    let i64_mir_ty = ctx.mir.ty_i64();

    // Get length: let len = array.matchLength()
    let len_local = ctx.create_temp("array_len", i64_mir_ty);
    let len_place = Place::local(len_local);
    let length_callee = Callee::witness(
        array_matchable_protocol_name,
        "matchLength",
        for_type,
        vec![],
    );
    let length_args = vec![CallArg::borrow(Value::Place(array_place.clone()))];
    ctx.emit_call_with_modes(len_place.clone(), length_callee, length_args);

    // Lower prefix elements using witness calls
    for (i, sub_pattern) in prefix.iter().enumerate() {
        let element_value = emit_array_match_get(
            ctx,
            &array_place,
            Value::Immediate(make_int_immediate(IntBits::I64, i as i64)),
            &for_type,
            array_matchable_protocol_name,
            sub_pattern,
        );
        lower_pattern(ctx, sub_pattern, element_value);
    }

    // Lower suffix elements (from end)
    let suffix_len = suffix.len();
    for (i, sub_pattern) in suffix.iter().enumerate() {
        // Index from end: len - suffix_len + i
        let offset_from_end = suffix_len - i;
        let index_local = ctx.create_temp("suffix_idx", i64_mir_ty);
        let index_place = Place::local(index_local);

        // index = len - offset_from_end
        ctx.emit_assign(
            index_place.clone(),
            Rvalue::BinaryOp {
                op: BinOp::SubSigned,
                lhs: Value::Place(len_place.clone()),
                rhs: Value::Immediate(make_int_immediate(IntBits::I64, offset_from_end as i64)),
            },
        );

        let element_value = emit_array_match_get(
            ctx,
            &array_place,
            Value::Place(index_place),
            &for_type,
            array_matchable_protocol_name,
            sub_pattern,
        );
        lower_pattern(ctx, sub_pattern, element_value);
    }

    // Lower rest binding if present
    if let Some((name, local_id_opt)) = rest
        && let Some(local_id) = local_id_opt
    {
        // We have a named rest binding: ..rest
        // rest = array.matchSlice(prefix_len, len - suffix_len)
        let prefix_len = prefix.len();

        // Calculate end index: len - suffix_len
        let end_local = ctx.create_temp("rest_end", i64_mir_ty);
        let end_place = Place::local(end_local);
        ctx.emit_assign(
            end_place.clone(),
            Rvalue::BinaryOp {
                op: BinOp::SubSigned,
                lhs: Value::Place(len_place.clone()),
                rhs: Value::Immediate(make_int_immediate(IntBits::I64, suffix_len as i64)),
            },
        );

        // Get or create the MIR local for the rest binding
        let mir_local = if let Some(existing) = ctx.get_local(*local_id) {
            existing
        } else {
            // Get the rest pattern's type (should be Slice[T])
            // We need to infer it from the array's element type
            let rest_name = name.as_deref().unwrap_or("rest");
            let rest_ty = get_rest_slice_type(ctx, &pattern.ty);
            let new_local = ctx.create_local(rest_name, rest_ty);
            ctx.map_local(*local_id, new_local);
            new_local
        };
        let rest_place = Place::local(mir_local);

        // Call matchSlice(prefix_len, end)
        let slice_callee = Callee::witness(
            array_matchable_protocol_name,
            "matchSlice",
            for_type,
            vec![],
        );
        let slice_args = vec![
            CallArg::borrow(Value::Place(array_place.clone())),
            CallArg::copy(Value::Immediate(make_int_immediate(
                IntBits::I64,
                prefix_len as i64,
            ))),
            CallArg::copy(Value::Place(end_place)),
        ];
        ctx.emit_call_with_modes(rest_place, slice_callee, slice_args);
    }
    // If local_id is None, it's anonymous rest `..` - nothing to bind
}

/// Get the ArrayMatchable protocol name for witness calls.
fn get_array_matchable_protocol_name(ctx: &mut LoweringContext) -> Id<QualifiedName> {
    if let Some(am_id) = ctx.model.builtin_registry().array_matchable_protocol()
        && let Some(am_symbol) = ctx.model.query(SymbolFor { id: am_id })
    {
        return qualified_name_for_symbol(ctx, &am_symbol);
    }
    // Fallback to manual construction
    ctx.mir.intern_name(QualifiedNameData::new(vec![
        "std".to_string(),
        "core".to_string(),
        "ArrayMatchable".to_string(),
    ]))
}

/// Emit a call to array.matchGet(index) and return the result value.
fn emit_array_match_get(
    ctx: &mut LoweringContext,
    array_place: &Place,
    index: Value,
    for_type: &Id<MirTyMarker>,
    protocol_name: Id<QualifiedName>,
    sub_pattern: &Pattern,
) -> Value {
    let element_ty = lower_type(ctx, &sub_pattern.ty);
    let element_local = ctx.create_temp("array_elem", element_ty);
    let element_place = Place::local(element_local);

    let get_callee = Callee::witness(protocol_name, "matchGet", *for_type, vec![]);
    let get_args = vec![
        CallArg::borrow(Value::Place(array_place.clone())),
        CallArg::copy(index),
    ];
    ctx.emit_call_with_modes(element_place.clone(), get_callee, get_args);

    Value::Place(element_place)
}

/// Get the Slice[T] type for the rest binding given the array type.
fn get_rest_slice_type(
    ctx: &mut LoweringContext,
    array_ty: &kestrel_semantic_tree::ty::Ty,
) -> Id<MirTyMarker> {
    // The array type should be Array[T] or similar - extract T and make Slice[T]
    // For now, we'll get the element type and create the corresponding MIR slice type
    use kestrel_semantic_tree::ty::TyKind;

    if let TyKind::Struct { substitutions, .. } = array_ty.kind() {
        // Get the element type from the Array[T] substitutions
        if let Some((_, elem_ty)) = substitutions.iter().next() {
            // Create Slice[elem_ty] using the model
            if let Some(slice_ty) = ctx
                .model
                .make_slice_type(elem_ty.clone(), array_ty.span().clone())
            {
                return lower_type(ctx, &slice_ty);
            }
        }
    }

    // Fallback: return a unit type (this shouldn't happen with valid code)
    ctx.mir.ty_unit()
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
        },
        Value::Unreachable => {
            // This shouldn't happen - callers should check for Unreachable before calling
            panic!("ensure_place called with Unreachable value");
        },
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
