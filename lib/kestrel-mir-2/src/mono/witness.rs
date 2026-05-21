use std::collections::HashMap;
use std::fmt;

use kestrel_hecs::Entity;

use crate::item::function::FunctionDef;
use crate::item::protocol::ProtocolDef;
use crate::item::witness::WitnessDef;
use crate::statement::WitnessMethodKey;
use crate::substitute::{SubstMap, substitute};
use crate::ty::{MirTy, TyArena};
use crate::TyId;

// -- Error type --

#[derive(Debug, Clone)]
pub enum MonoError {
    WitnessNotFound {
        protocol_name: String,
        type_description: String,
    },
    MethodNotFound {
        protocol_name: String,
        method: String,
        type_description: String,
    },
    TypeArgArityMismatch {
        function: String,
        expected: usize,
        got: usize,
    },
}

impl fmt::Display for MonoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MonoError::WitnessNotFound {
                protocol_name,
                type_description,
            } => write!(
                f,
                "no witness for {type_description}: {protocol_name}"
            ),
            MonoError::MethodNotFound {
                protocol_name,
                method,
                type_description,
            } => write!(
                f,
                "method {method} not found in witness for {type_description}: {protocol_name}"
            ),
            MonoError::TypeArgArityMismatch {
                function,
                expected,
                got,
            } => write!(
                f,
                "type arg arity mismatch for {function}: expected {expected}, got {got}"
            ),
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
/// 1. Exact protocol match
/// 2. Descendant protocol (protocol inheritance fallback)
pub fn find_witness_with_method(
    arena: &TyArena,
    witnesses: &[WitnessDef],
    protocols: &[ProtocolDef],
    protocol: Entity,
    method: &WitnessMethodKey,
    self_type: TyId,
) -> Result<(usize, HashMap<Entity, TyId>), MonoError> {
    // Pass 1: exact protocol match
    for (i, witness) in witnesses.iter().enumerate() {
        if witness.protocol != protocol {
            continue;
        }
        let mut bindings = HashMap::new();
        if match_pattern(arena, witness.implementing_type, self_type, &mut bindings)
            && witness.methods.iter().any(|m| m.key == *method)
        {
            return Ok((i, bindings));
        }
    }

    // Pass 2: descendant protocol (inheritance)
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
    })
}

/// Resolve a `Callee::Witness` to a concrete function entity + type_args + self_type.
pub fn resolve_witness_call(
    arena: &mut TyArena,
    witnesses: &[WitnessDef],
    protocols: &[ProtocolDef],
    functions: &[FunctionDef],
    entity_names: &indexmap::IndexMap<Entity, String>,
    protocol: Entity,
    method: &WitnessMethodKey,
    self_type: TyId,
    method_type_args: &[TyId],
) -> Result<ResolvedWitnessCall, MonoError> {
    let (witness_idx, bindings) =
        find_witness_with_method(arena, witnesses, protocols, protocol, method, self_type)?;

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

    // Substitute the binding's type_args (protocol type arg expressions)
    // through the bindings to get concrete type args.
    // e.g., binding.type_args = [TypeParam(T_array)] → substitute → [Int64]
    let mut type_args: Vec<TyId> = binding
        .type_args
        .iter()
        .map(|&ta| substitute(arena, ta, &subst))
        .collect();

    // Append any method-level type args past the protocol's param count
    let proto_param_count = protocols
        .iter()
        .find(|p| p.entity == protocol)
        .map(|p| p.type_params.len())
        .unwrap_or(0);
    let method_level_args = method_type_args.get(proto_param_count..).unwrap_or(&[]);
    type_args.extend_from_slice(method_level_args);

    // Cap to the concrete function's param count
    let concrete_func = functions.iter().find(|f| f.entity == binding.func);
    if let Some(func) = concrete_func {
        type_args.truncate(func.type_params.len());
    }

    // Determine if self_type should be propagated.
    // Extension methods (default impls on the protocol itself) need self_type.
    let needs_self = if let Some(func) = concrete_func {
        use crate::item::function::FunctionKind;
        matches!(
            &func.kind,
            FunctionKind::Method { parent, .. } | FunctionKind::StaticMethod { parent }
            if protocols.iter().any(|p| p.entity == *parent)
        )
    } else {
        false
    };

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
    for witness in witnesses {
        if witness.protocol != protocol {
            continue;
        }
        let mut bindings = HashMap::new();
        if !match_pattern(arena, witness.implementing_type, self_type, &mut bindings) {
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
    protocols: &[ProtocolDef],
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
        let Some(def) = protocols.iter().find(|p| p.entity == proto) else {
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
    use crate::item::protocol::ProtocolDef;
    use crate::item::witness::{WitnessDef, WitnessMethodBinding};
    use crate::statement::WitnessMethodKey;

    fn entity(id: u32) -> Entity {
        Entity::from_raw(id)
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
        let protos: Vec<ProtocolDef> = vec![];
        assert!(protocol_inherits(&protos, entity(1), entity(1)));
    }

    #[test]
    fn inherits_direct_parent() {
        let mut comparable = ProtocolDef::new(entity(1), "Comparable");
        comparable.parent_protocols.push(entity(2));
        let equatable = ProtocolDef::new(entity(2), "Equatable");
        let protos = vec![comparable, equatable];
        assert!(protocol_inherits(&protos, entity(1), entity(2)));
    }

    #[test]
    fn inherits_transitive() {
        let mut a = ProtocolDef::new(entity(1), "A");
        a.parent_protocols.push(entity(2));
        let mut b = ProtocolDef::new(entity(2), "B");
        b.parent_protocols.push(entity(3));
        let c = ProtocolDef::new(entity(3), "C");
        let protos = vec![a, b, c];
        assert!(protocol_inherits(&protos, entity(1), entity(3)));
    }

    #[test]
    fn no_inheritance() {
        let a = ProtocolDef::new(entity(1), "A");
        let b = ProtocolDef::new(entity(2), "B");
        let protos = vec![a, b];
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
            WitnessMethodKey::simple("equals"),
            impl_func,
            vec![],
        ));

        let witnesses = vec![witness];
        let protocols = vec![ProtocolDef::new(proto, "Equatable")];

        let (idx, bindings) = find_witness_with_method(
            &a,
            &witnesses,
            &protocols,
            proto,
            &WitnessMethodKey::simple("equals"),
            i64,
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
            WitnessMethodKey::simple("equals"),
            impl_func,
            vec![],
        ));

        let witnesses = vec![witness];
        let protocols = vec![ProtocolDef::new(proto, "Equatable")];

        let (idx, bindings) = find_witness_with_method(
            &a,
            &witnesses,
            &protocols,
            proto,
            &WitnessMethodKey::simple("equals"),
            concrete,
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
        let protocols = vec![comparable_def, equatable_def];

        // Witness is for Comparable but has the Equatable method
        let mut witness = WitnessDef::new(comparable, i64);
        witness.add_method(WitnessMethodBinding::new(
            WitnessMethodKey::simple("equals"),
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
            &WitnessMethodKey::simple("equals"),
            i64,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn find_witness_not_found() {
        let mut a = TyArena::new();
        let proto = entity(1);
        let i64 = a.i64();
        let witnesses: Vec<WitnessDef> = vec![];
        let protocols = vec![ProtocolDef::new(proto, "Equatable")];

        let result = find_witness_with_method(
            &a,
            &witnesses,
            &protocols,
            proto,
            &WitnessMethodKey::simple("equals"),
            i64,
        );
        assert!(result.is_err());
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
