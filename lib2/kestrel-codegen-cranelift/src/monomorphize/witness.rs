//! Protocol witness resolution.
//!
//! Resolves `Callee::Witness` calls to concrete function instantiations
//! by searching the witness table for matching implementations.
//!
//! Key improvement over lib1: uses `MethodSource::Extension` enum directly
//! instead of the fragile `impl_func_name.contains(protocol_name)` heuristic.

use super::error::MonomorphizeError;
use kestrel_codegen2::substitute_type;
use kestrel_hecs::Entity;
use kestrel_mir::{MethodSource, MirModule, MirTy, WitnessDef};
use std::collections::HashMap;

/// Result of resolving a witness method call.
#[derive(Debug, Clone)]
pub struct ResolvedWitnessCall {
    /// The concrete function entity.
    pub func_entity: Entity,
    /// Type arguments for the function.
    pub type_args: Vec<MirTy>,
    /// Self type for protocol extension methods.
    pub self_type: Option<MirTy>,
}

/// Resolve a witness method call to a concrete function.
pub fn resolve_witness_call(
    module: &MirModule,
    protocol: Entity,
    method: &str,
    self_type: &MirTy,
    method_type_args: &[MirTy],
) -> Result<ResolvedWitnessCall, MonomorphizeError> {
    // Find a witness that matches the self type and protocol, and has the method.
    // First try exact protocol, then any witness for this type that has the method
    // (handles protocol inheritance: Comparable witness contains Less.lessThan).
    let (witness, bindings) = find_witness_with_method(module, protocol, method, self_type)?;

    let method_binding = witness.method_bindings.get(method).unwrap();


    // Build the type args for the concrete function.
    //
    // The incoming `method_type_args` may contain BOTH protocol-level type params
    // (e.g., Convertible[From] → From=Int64) and method-level type params.
    // Only the method-level ones should be passed through — protocol-level params
    // are already resolved through the witness bindings.
    let concrete_func = module.functions.iter()
        .find(|f| f.entity == method_binding.implementation);
    let concrete_param_count = concrete_func
        .map(|f| f.type_params.len())
        .unwrap_or(0);

    let mut type_args = Vec::new();

    match &method_binding.source {
        MethodSource::Direct => {
            // Direct implementation: prepend witness type param bindings,
            // then append the method-level type args
            for tp in &witness.type_params {
                if let Some(bound) = bindings.get(&tp.entity) {
                    type_args.push(bound.clone());
                }
            }
            type_args.extend(method_type_args.iter().cloned());
        }
        MethodSource::Extension { .. } => {
            // Extension method: only use method-level type args
            // The self_type is passed separately for mangling
            type_args.extend(method_type_args.iter().cloned());
        }
    }

    // Also include the binding's own type_args
    for ta in &method_binding.type_args {
        let substituted = substitute_type(ta, &bindings);
        type_args.push(substituted);
    }

    // Cap to the concrete function's actual type param count — protocol-level
    // type params (e.g., Convertible[From]) get resolved through bindings and
    // shouldn't leak into the concrete function's instantiation
    type_args.truncate(concrete_param_count);

    let needs_self = matches!(method_binding.source, MethodSource::Extension { .. });

    Ok(ResolvedWitnessCall {
        func_entity: method_binding.implementation,
        type_args,
        self_type: if needs_self {
            Some(self_type.clone())
        } else {
            None
        },
    })
}

/// Resolve an associated type through a witness table.
pub fn resolve_associated_type(
    module: &MirModule,
    protocol: Entity,
    self_type: &MirTy,
    assoc_name: &str,
) -> Result<MirTy, MonomorphizeError> {
    let (witness, bindings) = find_witness(module, protocol, self_type)?;

    let bound_ty = witness.type_bindings.get(assoc_name).ok_or_else(|| {
        MonomorphizeError::MethodNotFound {
            protocol_name: module.resolve_name(protocol).to_string(),
            method: assoc_name.to_string(),
            type_description: format!("{self_type:?}"),
        }
    })?;

    // Apply witness type param bindings to the associated type
    Ok(substitute_type(bound_ty, &bindings))
}

/// Find a witness for `self_type` that has the given method.
/// First tries exact protocol match, then falls back to any witness that
/// has the method (handles protocol inheritance — e.g., Comparable witness
/// contains Less.lessThan because witness generation collects inherited methods).
fn find_witness_with_method<'a>(
    module: &'a MirModule,
    protocol: Entity,
    method: &str,
    self_type: &MirTy,
) -> Result<(&'a WitnessDef, HashMap<Entity, MirTy>), MonomorphizeError> {
    // Pass 1: exact protocol match
    for witness in &module.witnesses {
        if witness.protocol != protocol {
            continue;
        }
        let mut bindings = HashMap::new();
        if match_pattern(&witness.implementing_type, self_type, &mut bindings) {
            if witness.method_bindings.contains_key(method) {
                return Ok((witness, bindings));
            }
        }
    }

    // Pass 2: a witness for a descendant protocol on the same type that has
    // the method (e.g. Comparable witness contains Less.lessThan).
    for witness in &module.witnesses {
        if witness.protocol == protocol {
            continue;
        }
        if !protocol_inherits(module, witness.protocol, protocol) {
            continue;
        }
        if !witness.method_bindings.contains_key(method) {
            continue;
        }
        let mut bindings = HashMap::new();
        if match_pattern(&witness.implementing_type, self_type, &mut bindings) {
            return Ok((witness, bindings));
        }
    }

    // Include entity name and debug info for debugging
    let type_name = match self_type {
        MirTy::Named { entity, type_args } => {
            let name = module.resolve_name(*entity);
            format!("{} (entity {:?}, {} type_args)", name, entity, type_args.len())
        },
        other => format!("{other:?}"),
    };
    Err(MonomorphizeError::MethodNotFound {
        protocol_name: module.resolve_name(protocol).to_string(),
        method: method.to_string(),
        type_description: type_name,
    })
}

/// Find a witness that proves `self_type` implements `protocol`.
fn find_witness<'a>(
    module: &'a MirModule,
    protocol: Entity,
    self_type: &MirTy,
) -> Result<(&'a WitnessDef, HashMap<Entity, MirTy>), MonomorphizeError> {
    for witness in &module.witnesses {
        if witness.protocol != protocol {
            continue;
        }
        let mut bindings = HashMap::new();
        if match_pattern(&witness.implementing_type, self_type, &mut bindings) {
            return Ok((witness, bindings));
        }
    }

    Err(MonomorphizeError::WitnessNotFound {
        protocol_name: module.resolve_name(protocol).to_string(),
        type_description: format!("{self_type:?}"),
    })
}

fn protocol_inherits(module: &MirModule, candidate: Entity, target: Entity) -> bool {
    if candidate == target {
        return true;
    }

    let mut stack = vec![candidate];
    let mut seen = std::collections::HashSet::new();
    while let Some(protocol) = stack.pop() {
        if !seen.insert(protocol) {
            continue;
        }
        let Some(def) = module.protocols.iter().find(|p| p.entity == protocol) else {
            continue;
        };
        for parent in &def.parent_protocols {
            if *parent == target {
                return true;
            }
            stack.push(*parent);
        }
    }

    false
}

/// Structural pattern matching for witness type resolution.
///
/// The `pattern` may contain `TypeParam` entries that act as wildcards,
/// binding to the corresponding part of the `concrete` type.
fn match_pattern(
    pattern: &MirTy,
    concrete: &MirTy,
    bindings: &mut HashMap<Entity, MirTy>,
) -> bool {
    match (pattern, concrete) {
        // Type parameter in pattern: bind to the concrete type
        (MirTy::TypeParam(entity), _) => {
            if let Some(existing) = bindings.get(entity) {
                existing == concrete
            } else {
                bindings.insert(*entity, concrete.clone());
                true
            }
        }

        // Named types must match entity and recurse on type args
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
                    .zip(args2)
                    .all(|(p, c)| match_pattern(p, c, bindings))
        }

        // Structural types recurse element-wise
        (MirTy::Ref(a), MirTy::Ref(b))
        | (MirTy::RefMut(a), MirTy::RefMut(b))
        | (MirTy::Pointer(a), MirTy::Pointer(b)) => match_pattern(a, b, bindings),

        (MirTy::Tuple(a), MirTy::Tuple(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|(p, c)| match_pattern(p, c, bindings))
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
                    .zip(p2)
                    .all(|(p, c)| match_pattern(p, c, bindings))
                && match_pattern(r1, r2, bindings)
        }

        // Primitives and other leaves: exact equality
        _ => pattern == concrete,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(id: u32) -> Entity {
        Entity::from_raw(id)
    }

    #[test]
    fn match_concrete_types() {
        let mut bindings = HashMap::new();
        assert!(match_pattern(&MirTy::I64, &MirTy::I64, &mut bindings));
        assert!(!match_pattern(&MirTy::I64, &MirTy::I32, &mut bindings));
    }

    #[test]
    fn match_type_param_binds() {
        let t = entity(1);
        let mut bindings = HashMap::new();
        assert!(match_pattern(
            &MirTy::TypeParam(t),
            &MirTy::I64,
            &mut bindings
        ));
        assert_eq!(bindings.get(&t), Some(&MirTy::I64));
    }

    #[test]
    fn match_named_with_type_args() {
        let array_e = entity(1);
        let t = entity(2);
        let mut bindings = HashMap::new();

        let pattern = MirTy::Named {
            entity: array_e,
            type_args: vec![MirTy::TypeParam(t)],
        };
        let concrete = MirTy::Named {
            entity: array_e,
            type_args: vec![MirTy::I64],
        };
        assert!(match_pattern(&pattern, &concrete, &mut bindings));
        assert_eq!(bindings.get(&t), Some(&MirTy::I64));
    }

    #[test]
    fn match_ref_structural() {
        let t = entity(1);
        let mut bindings = HashMap::new();

        let pattern = MirTy::Ref(Box::new(MirTy::TypeParam(t)));
        let concrete = MirTy::Ref(Box::new(MirTy::I32));
        assert!(match_pattern(&pattern, &concrete, &mut bindings));
        assert_eq!(bindings.get(&t), Some(&MirTy::I32));
    }

    #[test]
    fn match_rejects_conflicting_bindings() {
        let t = entity(1);
        let mut bindings = HashMap::new();

        // Bind T = I64
        let pattern1 = MirTy::TypeParam(t);
        assert!(match_pattern(&pattern1, &MirTy::I64, &mut bindings));

        // Try to match T against I32 (should fail)
        assert!(!match_pattern(&pattern1, &MirTy::I32, &mut bindings));
    }
}
