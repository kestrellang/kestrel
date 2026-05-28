use std::collections::HashMap;
use std::fmt;

use kestrel_hecs::Entity;
use kestrel_span::Span;

use crate::item::function::FunctionDef;
use crate::item::protocol::ProtocolDef;
use crate::item::witness::WitnessDef;
use crate::item::witness::WitnessMethodKey;
use crate::substitute::{SubstMap, substitute};
use crate::ty::{MirTy, TyArena};
use crate::TyId;

// -- Error type --

#[derive(Debug, Clone)]
pub enum MonoError {
    WitnessNotFound {
        protocol_name: String,
        type_description: String,
        source_entity: Entity,
        span: Option<Span>,
    },
    MethodNotFound {
        protocol_name: String,
        method: String,
        type_description: String,
        source_entity: Entity,
        span: Option<Span>,
    },
    TypeArgArityMismatch {
        function: String,
        expected: usize,
        got: usize,
        source_entity: Entity,
        span: Option<Span>,
    },
}

impl fmt::Display for MonoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MonoError::WitnessNotFound {
                protocol_name,
                type_description,
                ..
            } => write!(
                f,
                "no witness for {type_description}: {protocol_name}"
            ),
            MonoError::MethodNotFound {
                protocol_name,
                method,
                type_description,
                ..
            } => write!(
                f,
                "method {method} not found in witness for {type_description}: {protocol_name}"
            ),
            MonoError::TypeArgArityMismatch {
                function,
                expected,
                got,
                ..
            } => write!(
                f,
                "type arg arity mismatch for {function}: expected {expected}, got {got}"
            ),
        }
    }
}

impl MonoError {
    pub fn source_entity(&self) -> Entity {
        match self {
            MonoError::WitnessNotFound { source_entity, .. }
            | MonoError::MethodNotFound { source_entity, .. }
            | MonoError::TypeArgArityMismatch { source_entity, .. } => *source_entity,
        }
    }

    pub fn span(&self) -> Option<&Span> {
        match self {
            MonoError::WitnessNotFound { span, .. }
            | MonoError::MethodNotFound { span, .. }
            | MonoError::TypeArgArityMismatch { span, .. } => span.as_ref(),
        }
    }
}

// -- Resolved witness call --

#[derive(Debug, Clone)]
pub struct ResolvedWitnessCall {
    pub func_entity: Entity,
    pub type_args: Vec<TyId>,
    pub self_type: Option<TyId>,
}

// -- Pattern matching --

/// Structural pattern matching for witness type resolution.
///
/// The `pattern` type may contain `TypeParam` entries that act as wildcards,
/// binding to the corresponding part of the `concrete` type.
pub fn match_pattern(
    arena: &TyArena,
    pattern: TyId,
    concrete: TyId,
    bindings: &mut HashMap<Entity, TyId>,
) -> bool {
    if pattern == concrete {
        return true;
    }

    match (arena.get(pattern).clone(), arena.get(concrete).clone()) {
        // TypeParam in pattern: wildcard that binds
        (MirTy::TypeParam(entity), _) => {
            if let Some(&existing) = bindings.get(&entity) {
                existing == concrete
            } else {
                bindings.insert(entity, concrete);
                true
            }
        }

        // Named types: entity must match, recurse on type_args
        (
            MirTy::Named {
                entity: e1,
                type_args: args1,
            },
            MirTy::Named {
                entity: e2,
                type_args: args2,
            },
        ) => {
            e1 == e2
                && args1.len() == args2.len()
                && args1
                    .iter()
                    .zip(args2.iter())
                    .all(|(&p, &c)| match_pattern(arena, p, c, bindings))
        }

        // Structural recursion on wrapper types
        (MirTy::Pointer(a), MirTy::Pointer(b)) => match_pattern(arena, a, b, bindings),

        (MirTy::Tuple(a), MirTy::Tuple(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b.iter())
                    .all(|(&p, &c)| match_pattern(arena, p, c, bindings))
        }

        (
            MirTy::FuncThin {
                params: p1,
                ret: r1,
            },
            MirTy::FuncThin {
                params: p2,
                ret: r2,
            },
        )
        | (
            MirTy::FuncThick {
                params: p1,
                ret: r1,
            },
            MirTy::FuncThick {
                params: p2,
                ret: r2,
            },
        ) => {
            p1.len() == p2.len()
                && p1
                    .iter()
                    .zip(p2.iter())
                    .all(|((pt, _), (ct, _))| match_pattern(arena, *pt, *ct, bindings))
                && match_pattern(arena, r1, r2, bindings)
        }

        // Primitives and other leaves: exact equality (already handled by TyId == at top)
        _ => false,
    }
}

// -- Witness lookup --

/// Find a witness for `(protocol, self_type)` that has the given method.
/// Returns the witness index + pattern match bindings.
///
/// Two-pass:
/// 1. Exact protocol match (filtered by protocol type args)
/// 2. Descendant protocol (protocol inheritance fallback)
pub fn find_witness_with_method(
    arena: &TyArena,
    witnesses: &[WitnessDef],
    protocols: &indexmap::IndexMap<Entity, ProtocolDef>,
    protocol: Entity,
    method: &WitnessMethodKey,
    self_type: TyId,
    method_type_args: &[TyId],
) -> Result<(usize, HashMap<Entity, TyId>), MonoError> {
    // Extract the protocol's type args from the call-site method_type_args.
    // For `Convertible[Int64].init(from:)`, method_type_args[0] = Int64.
    let proto_param_count = protocols.get(&protocol)
        .map(|p| p.type_params.len())
        .unwrap_or(0);
    let expected_proto_args = &method_type_args[..method_type_args.len().min(proto_param_count)];

    // Pass 1: exact protocol match with protocol type arg filtering
    for (i, witness) in witnesses.iter().enumerate() {
        if witness.protocol != protocol {
            continue;
        }
        if !witness_proto_args_match(arena, witness, expected_proto_args) {
            continue;
        }
        let mut bindings = HashMap::new();
        if match_pattern(arena, witness.implementing_type, self_type, &mut bindings)
            && witness.methods.iter().any(|m| m.key == *method)
        {
            return Ok((i, bindings));
        }
    }

    // Pass 2: descendant protocol (inheritance) — no proto arg filter
    // since inherited witnesses carry the descendant's type args.
    for (i, witness) in witnesses.iter().enumerate() {
        if witness.protocol == protocol {
            continue;
        }
        if !protocol_inherits(protocols, witness.protocol, protocol) {
            continue;
        }
        if !witness.methods.iter().any(|m| m.key == *method) {
            continue;
        }
        let mut bindings = HashMap::new();
        if match_pattern(arena, witness.implementing_type, self_type, &mut bindings) {
            return Ok((i, bindings));
        }
    }

    Err(MonoError::MethodNotFound {
        protocol_name: format!("{protocol:?}"),
        method: method.name.clone(),
        type_description: format!("{:?}", arena.get(self_type)),
        source_entity: Entity::from_raw(0),
        span: None,
    })
}

/// Check whether a witness's protocol type args match the expected concrete
/// types from the call site. Empty expected matches any witness (back-compat
/// for non-generic protocols). Witness args that are TypeParam wildcards
/// (from `extend T: Proto[FreeParam]`) match anything.
fn witness_proto_args_match(arena: &TyArena, witness: &WitnessDef, expected: &[TyId]) -> bool {
    if expected.is_empty() {
        return true;
    }
    if witness.proto_type_args.is_empty() {
        return true;
    }
    if witness.proto_type_args.len() != expected.len() {
        return false;
    }
    witness.proto_type_args.iter().zip(expected.iter()).all(|(&w, &e)| {
        matches!(arena.get(w), MirTy::TypeParam(_)) || w == e
    })
}

/// Resolve a `Callee::Witness` to a concrete function entity + type_args + self_type.
pub fn resolve_witness_call(
    arena: &mut TyArena,
    witnesses: &[WitnessDef],
    protocols: &indexmap::IndexMap<Entity, ProtocolDef>,
    functions: &indexmap::IndexMap<Entity, FunctionDef>,
    entity_names: &indexmap::IndexMap<Entity, String>,
    protocol: Entity,
    method: &WitnessMethodKey,
    self_type: TyId,
    method_type_args: &[TyId],
) -> Result<ResolvedWitnessCall, MonoError> {
    let (witness_idx, bindings) =
        find_witness_with_method(arena, witnesses, protocols, protocol, method, self_type, method_type_args)?;

    let witness = &witnesses[witness_idx];
    let binding = witness
        .methods
        .iter()
        .find(|m| m.key == *method)
        .unwrap();

    // Build substitution from pattern match bindings.
    // match_pattern(witness.implementing_type, self_type) gives us
    // impl-type-param → concrete (e.g., T_array → Int64).
    let mut subst = SubstMap::new();
    for (entity, ty) in &bindings {
        subst.type_params.insert(*entity, *ty);
    }

    // Map protocol type params to their concrete values from the call site.
    // For `extend Int64: SeqIndex[T]`, proto_type_args = [TypeParam(T_ext)]
    // and method_type_args = [String]. This creates T_ext → String.
    let proto_tp_entities = protocols
        .get(&protocol)
        .map(|p| &p.type_params[..])
        .unwrap_or(&[]);
    for (i, proto_tp) in proto_tp_entities.iter().enumerate() {
        if let Some(&concrete_arg) = method_type_args.get(i) {
            // The witness's proto_type_args[i] is the expression that maps
            // this protocol type param to the witness context (e.g., TypeParam(T_ext)).
            if let Some(&proto_expr) = witness.proto_type_args.get(i) {
                if let MirTy::TypeParam(ext_entity) = arena.get(proto_expr) {
                    if !subst.type_params.contains_key(ext_entity) {
                        subst.type_params.insert(*ext_entity, concrete_arg);
                    }
                }
            }
            // Also map the protocol's own type param entity directly
            subst.type_params.entry(proto_tp.entity).or_insert(concrete_arg);
        }
    }

    let concrete_func = functions.get(&binding.func);

    // Determine if self_type should be propagated. Protocol default methods
    // need self_type because their Self param is TypeParam(protocol_entity).
    // Detect by checking if the first param is a TypeParam not in the
    // function's type_params list.
    let needs_self = if let Some(func) = concrete_func {
        let known_tps: std::collections::HashSet<Entity> =
            func.type_params.iter().map(|tp| tp.entity).collect();
        func.params.first().is_some_and(|p| {
            matches!(arena.get(p.ty), MirTy::TypeParam(e) if !known_tps.contains(e))
        })
    } else {
        false
    };

    let proto_param_count = proto_tp_entities.len();
    let mut type_args: Vec<TyId> = if needs_self {
        // Protocol-extension default methods receive Self via `self_type`.
        // Their function type params are the protocol-level args followed by
        // the method-level args, already laid out that way at the call site.
        // The witness implementation type params describe the concrete Self
        // pattern and must not be prepended here.
        method_type_args.to_vec()
    } else {
        // Direct implementation: prepend witness implementation type args,
        // then append method-level type args past the protocol's own params.
        let mut args: Vec<TyId> = binding
            .type_args
            .iter()
            .map(|&ta| substitute(arena, ta, &subst))
            .collect();
        let method_level_args = method_type_args.get(proto_param_count..).unwrap_or(&[]);
        args.extend_from_slice(method_level_args);
        args
    };

    // Cap to the concrete function's param count.
    if let Some(func) = concrete_func {
        type_args.truncate(func.type_params.len());
    }

    Ok(ResolvedWitnessCall {
        func_entity: binding.func,
        type_args,
        self_type: if needs_self {
            Some(self_type)
        } else {
            let _ = entity_names;
            None
        },
    })
}

/// Resolve an associated type projection via witness lookup.
///
/// For a concrete `self_type` conforming to `protocol`, find the
/// associated type binding for `assoc_entity`.
pub fn resolve_associated_type(
    arena: &mut TyArena,
    witnesses: &[WitnessDef],
    protocol: Entity,
    self_type: TyId,
    assoc_entity: Entity,
) -> Option<TyId> {
    let verbose = std::env::var("VERBOSE_DEBUG_OUTPUT").is_ok();
    if verbose {
        eprintln!("  resolve_assoc_type: proto={:?} self={:?} ({:?}) assoc={:?}",
            protocol, self_type, arena.get(self_type).clone(), assoc_entity);
    }
    for witness in witnesses {
        if witness.protocol != protocol {
            continue;
        }
        if verbose {
            eprintln!("    witness: impl={:?} ({:?}) bindings={:?}",
                witness.implementing_type, arena.get(witness.implementing_type).clone(),
                witness.type_bindings);
        }
        let mut bindings = HashMap::new();
        if !match_pattern(arena, witness.implementing_type, self_type, &mut bindings) {
            if verbose { eprintln!("    -> no match"); }
            continue;
        }
        for &(entity, bound_ty) in &witness.type_bindings {
            if entity == assoc_entity {
                if bindings.is_empty() {
                    return Some(bound_ty);
                }
                // Substitute pattern-match bindings into the bound type.
                // e.g., witness Array[T]: Iterator { Element = T }
                // with bindings {T → Int64} → Element = Int64
                let mut subst = SubstMap::new();
                for (param_entity, ty) in &bindings {
                    subst.type_params.insert(*param_entity, *ty);
                }
                return Some(substitute(arena, bound_ty, &subst));
            }
        }
    }
    None
}

// -- Protocol inheritance --

/// Check if `candidate` protocol inherits from `target` protocol (BFS).
pub fn protocol_inherits(
    protocols: &indexmap::IndexMap<Entity, ProtocolDef>,
    candidate: Entity,
    target: Entity,
) -> bool {
    if candidate == target {
        return true;
    }

    let mut stack = vec![candidate];
    let mut seen = std::collections::HashSet::new();
    while let Some(proto) = stack.pop() {
        if !seen.insert(proto) {
            continue;
        }
        let Some(def) = protocols.get(&proto) else {
            continue;
        };
        for &parent in &def.parent_protocols {
            if parent == target {
                return true;
            }
            stack.push(parent);
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::function::{FunctionDef, ParamDef};
    use crate::item::protocol::ProtocolDef;
    use crate::item::witness::{WitnessDef, WitnessMethodBinding, WitnessMethodKey};
    use crate::{ParamConvention, TypeParamDef, ValueId};

    fn entity(id: u32) -> Entity {
        Entity::from_raw(id)
    }

    fn proto_map(protos: Vec<ProtocolDef>) -> indexmap::IndexMap<Entity, ProtocolDef> {
        protos.into_iter().map(|p| (p.entity, p)).collect()
    }

    fn func_map(funcs: Vec<FunctionDef>) -> indexmap::IndexMap<Entity, FunctionDef> {
        funcs.into_iter().map(|f| (f.entity, f)).collect()
    }

    // -- match_pattern --

    #[test]
    fn match_concrete_equal() {
        let mut a = TyArena::new();
        let i64 = a.i64();
        let mut bindings = HashMap::new();
        assert!(match_pattern(&a, i64, i64, &mut bindings));
        assert!(bindings.is_empty());
    }

    #[test]
    fn match_concrete_different() {
        let mut a = TyArena::new();
        let i64 = a.i64();
        let i32 = a.i32();
        let mut bindings = HashMap::new();
        assert!(!match_pattern(&a, i64, i32, &mut bindings));
    }

    #[test]
    fn match_type_param_binds() {
        let mut a = TyArena::new();
        let t = a.intern(MirTy::TypeParam(entity(1)));
        let i64 = a.i64();
        let mut bindings = HashMap::new();
        assert!(match_pattern(&a, t, i64, &mut bindings));
        assert_eq!(bindings.get(&entity(1)), Some(&i64));
    }

    #[test]
    fn match_type_param_consistent() {
        let mut a = TyArena::new();
        let t = a.intern(MirTy::TypeParam(entity(1)));
        let i64 = a.i64();
        let i32 = a.i32();
        let mut bindings = HashMap::new();
        assert!(match_pattern(&a, t, i64, &mut bindings));
        // Same param must bind to same type
        assert!(match_pattern(&a, t, i64, &mut bindings));
        // Different type fails
        assert!(!match_pattern(&a, t, i32, &mut bindings));
    }

    #[test]
    fn match_named_with_type_args() {
        let mut a = TyArena::new();
        let array_e = entity(1);
        let t_param = a.intern(MirTy::TypeParam(entity(2)));
        let i64 = a.i64();
        let pattern = a.named(array_e, vec![t_param]);
        let concrete = a.named(array_e, vec![i64]);
        let mut bindings = HashMap::new();
        assert!(match_pattern(&a, pattern, concrete, &mut bindings));
        assert_eq!(bindings.get(&entity(2)), Some(&i64));
    }

    #[test]
    fn match_named_entity_mismatch() {
        let mut a = TyArena::new();
        let i64 = a.i64();
        let p = a.named(entity(1), vec![i64]);
        let c = a.named(entity(2), vec![i64]);
        let mut bindings = HashMap::new();
        assert!(!match_pattern(&a, p, c, &mut bindings));
    }

    #[test]
    fn match_pointer() {
        let mut a = TyArena::new();
        let t_param = a.intern(MirTy::TypeParam(entity(1)));
        let i32 = a.i32();
        let p = a.pointer(t_param);
        let c = a.pointer(i32);
        let mut bindings = HashMap::new();
        assert!(match_pattern(&a, p, c, &mut bindings));
        assert_eq!(bindings.get(&entity(1)), Some(&i32));
    }

    #[test]
    fn match_tuple() {
        let mut a = TyArena::new();
        let t = a.intern(MirTy::TypeParam(entity(1)));
        let i64 = a.i64();
        let b = a.bool();
        let p = a.tuple(vec![t, b]);
        let c = a.tuple(vec![i64, b]);
        let mut bindings = HashMap::new();
        assert!(match_pattern(&a, p, c, &mut bindings));
        assert_eq!(bindings.get(&entity(1)), Some(&i64));
    }

    #[test]
    fn match_tuple_length_mismatch() {
        let mut a = TyArena::new();
        let i64 = a.i64();
        let b = a.bool();
        let p = a.tuple(vec![i64]);
        let c = a.tuple(vec![i64, b]);
        let mut bindings = HashMap::new();
        assert!(!match_pattern(&a, p, c, &mut bindings));
    }

    // -- protocol_inherits --

    #[test]
    fn inherits_self() {
        let protos = proto_map(vec![]);
        assert!(protocol_inherits(&protos, entity(1), entity(1)));
    }

    #[test]
    fn inherits_direct_parent() {
        let mut comparable = ProtocolDef::new(entity(1), "Comparable");
        comparable.parent_protocols.push(entity(2));
        let equatable = ProtocolDef::new(entity(2), "Equatable");
        let protos = proto_map(vec![comparable, equatable]);
        assert!(protocol_inherits(&protos, entity(1), entity(2)));
    }

    #[test]
    fn inherits_transitive() {
        let mut a = ProtocolDef::new(entity(1), "A");
        a.parent_protocols.push(entity(2));
        let mut b = ProtocolDef::new(entity(2), "B");
        b.parent_protocols.push(entity(3));
        let c = ProtocolDef::new(entity(3), "C");
        let protos = proto_map(vec![a, b, c]);
        assert!(protocol_inherits(&protos, entity(1), entity(3)));
    }

    #[test]
    fn no_inheritance() {
        let a = ProtocolDef::new(entity(1), "A");
        let b = ProtocolDef::new(entity(2), "B");
        let protos = proto_map(vec![a, b]);
        assert!(!protocol_inherits(&protos, entity(1), entity(2)));
    }

    // -- find_witness_with_method --

    #[test]
    fn find_witness_exact_protocol() {
        let mut a = TyArena::new();
        let i64 = a.i64();
        let proto = entity(1);
        let impl_func = entity(10);

        let mut witness = WitnessDef::new(proto, i64);
        witness.add_method(WitnessMethodBinding::new(
            WitnessMethodKey::new("isEqual", vec![Some("to".into())]),
            impl_func,
            vec![],
        ));

        let witnesses = vec![witness];
        let protocols = proto_map(vec![ProtocolDef::new(proto, "Equatable")]);

        let (idx, bindings) = find_witness_with_method(
            &a,
            &witnesses,
            &protocols,
            proto,
            &WitnessMethodKey::new("isEqual", vec![Some("to".into())]),
            i64,
            &[],
        )
        .unwrap();
        assert_eq!(idx, 0);
        assert!(bindings.is_empty());
    }

    #[test]
    fn find_witness_generic_pattern() {
        let mut a = TyArena::new();
        let proto = entity(1);
        let array_e = entity(2);
        let impl_func = entity(10);

        let t_param = a.intern(MirTy::TypeParam(entity(3)));
        let i64 = a.i64();
        let pattern = a.named(array_e, vec![t_param]);
        let concrete = a.named(array_e, vec![i64]);

        let mut witness = WitnessDef::new(proto, pattern);
        witness.add_method(WitnessMethodBinding::new(
            WitnessMethodKey::new("isEqual", vec![Some("to".into())]),
            impl_func,
            vec![],
        ));

        let witnesses = vec![witness];
        let protocols = proto_map(vec![ProtocolDef::new(proto, "Equatable")]);

        let (idx, bindings) = find_witness_with_method(
            &a,
            &witnesses,
            &protocols,
            proto,
            &WitnessMethodKey::new("isEqual", vec![Some("to".into())]),
            concrete,
            &[],
        )
        .unwrap();
        assert_eq!(idx, 0);
        assert_eq!(bindings.get(&entity(3)), Some(&i64));
    }

    #[test]
    fn find_witness_inheritance_fallback() {
        let mut a = TyArena::new();
        let i64 = a.i64();
        let equatable = entity(1);
        let comparable = entity(2);
        let impl_func = entity(10);

        // Comparable inherits Equatable
        let mut comparable_def = ProtocolDef::new(comparable, "Comparable");
        comparable_def.parent_protocols.push(equatable);
        let equatable_def = ProtocolDef::new(equatable, "Equatable");
        let protocols = proto_map(vec![comparable_def, equatable_def]);

        // Witness is for Comparable but has the Equatable method
        let mut witness = WitnessDef::new(comparable, i64);
        witness.add_method(WitnessMethodBinding::new(
            WitnessMethodKey::new("isEqual", vec![Some("to".into())]),
            impl_func,
            vec![],
        ));
        let witnesses = vec![witness];

        // Looking for Equatable.equals on i64 — finds it via Comparable
        let result = find_witness_with_method(
            &a,
            &witnesses,
            &protocols,
            equatable,
            &WitnessMethodKey::new("isEqual", vec![Some("to".into())]),
            i64,
            &[],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn find_witness_not_found() {
        let mut a = TyArena::new();
        let proto = entity(1);
        let i64 = a.i64();
        let witnesses: Vec<WitnessDef> = vec![];
        let protocols = proto_map(vec![ProtocolDef::new(proto, "Equatable")]);

        let result = find_witness_with_method(
            &a,
            &witnesses,
            &protocols,
            proto,
            &WitnessMethodKey::new("isEqual", vec![Some("to".into())]),
            i64,
            &[],
        );
        assert!(result.is_err());
    }

    #[test]
    fn protocol_default_method_uses_call_site_method_type_args() {
        let mut a = TyArena::new();
        let iterator = entity(1);
        let flat_map = entity(2);
        let array_slice_iter = entity(3);
        let impl_t = entity(4);
        let method_u = entity(5);

        let impl_t_ty = a.intern(MirTy::TypeParam(impl_t));
        let pattern = a.named(array_slice_iter, vec![impl_t_ty]);
        let i64 = a.i64();
        let self_ty = a.named(array_slice_iter, vec![i64]);
        let returned_iter = a.named(entity(6), vec![i64]);

        let mut witness = WitnessDef::new(iterator, pattern);
        witness.add_method(WitnessMethodBinding::new(
            WitnessMethodKey::new("flatMap", vec![Some("as".into())]),
            flat_map,
            vec![impl_t_ty],
        ));

        let self_param = a.intern(MirTy::TypeParam(iterator));
        let mut func = FunctionDef::new(flat_map, "Iterator.flatMap", a.tuple(vec![]));
        func.type_params.push(TypeParamDef::new(method_u, "U"));
        func.params.push(ParamDef::new(
            "self",
            ValueId::new(0),
            self_param,
            ParamConvention::Borrow,
        ));

        let resolved = resolve_witness_call(
            &mut a,
            &[witness],
            &proto_map(vec![ProtocolDef::new(iterator, "Iterator")]),
            &func_map(vec![func]),
            &indexmap::IndexMap::new(),
            iterator,
            &WitnessMethodKey::new("flatMap", vec![Some("as".into())]),
            self_ty,
            &[returned_iter],
        )
        .unwrap();

        assert_eq!(resolved.type_args, vec![returned_iter]);
        assert_eq!(resolved.self_type, Some(self_ty));
    }

    // -- resolve_associated_type --

    #[test]
    fn resolve_assoc_type_direct() {
        let mut a = TyArena::new();
        let proto = entity(1);
        let assoc = entity(2);
        let i64 = a.i64();

        let mut witness = WitnessDef::new(proto, i64);
        witness.add_type_binding(assoc, a.str_ty());
        let witnesses = vec![witness];

        let result = resolve_associated_type(&mut a, &witnesses, proto, i64, assoc);
        assert_eq!(result, Some(a.str_ty()));
    }

    #[test]
    fn resolve_assoc_type_generic() {
        let mut a = TyArena::new();
        let proto = entity(1);
        let assoc = entity(2);
        let array_e = entity(3);

        let t_param = a.intern(MirTy::TypeParam(entity(4)));
        let i64 = a.i64();
        let pattern = a.named(array_e, vec![t_param]);
        let concrete = a.named(array_e, vec![i64]);

        // Witness says Array[T]: Iterator where Element = T
        let mut witness = WitnessDef::new(proto, pattern);
        witness.add_type_binding(assoc, t_param);
        let witnesses = vec![witness];

        // For Array[Int64], Element should resolve to the TypeParam binding
        // (actual substitution to Int64 happens in Phase 2 via SubstMap)
        let result = resolve_associated_type(&mut a, &witnesses, proto, concrete, assoc);
        assert!(result.is_some());
    }

    #[test]
    fn resolve_assoc_type_not_found() {
        let mut a = TyArena::new();
        let i64 = a.i64();
        let witnesses: Vec<WitnessDef> = vec![];
        let result = resolve_associated_type(&mut a, &witnesses, entity(1), i64, entity(2));
        assert!(result.is_none());
    }
}
