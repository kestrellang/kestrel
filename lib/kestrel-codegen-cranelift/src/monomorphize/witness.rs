//! Witness resolution for monomorphization.
//!
//! This module provides functions to find and resolve witnesses during
//! monomorphization. When we encounter a `Callee::Witness` in the MIR,
//! we need to:
//!
//! 1. Find the witness that proves the type implements the protocol
//! 2. Extract any type parameter bindings from pattern matching
//! 3. Look up the implementing method
//! 4. Return the direct call target with its type arguments

use super::error::MonomorphizeError;
use super::substitute::Substitution;
use kestrel_execution_graph::{Id, MirContext, MirTy, QualifiedName, Ty, TypeParam, Witness};
use std::collections::HashMap;

/// Result of finding a witness.
pub struct WitnessMatch {
    /// The witness that was found.
    pub witness_id: Id<Witness>,
    /// Type argument bindings extracted from pattern matching.
    /// For example, if the witness is for `Box[T]: Cloneable` and we're
    /// looking for `Box[Int]`, this will be `{T → Int}`.
    pub type_bindings: HashMap<Id<TypeParam>, Id<Ty>>,
}

/// Find a witness that proves `for_type` implements `protocol`.
///
/// This performs a linear scan over all witnesses, trying to pattern match
/// each witness's `implementing_type` against `for_type`.
///
/// For generic witnesses like `Box[T]: Cloneable`, we use pattern matching
/// to extract the type parameter bindings.
pub fn find_witness(
    mir: &MirContext,
    protocol: Id<QualifiedName>,
    for_type: Id<Ty>,
) -> Result<WitnessMatch, MonomorphizeError> {
    for (witness_id, witness_def) in mir.witnesses.iter() {
        // Check if this witness is for the right protocol
        if witness_def.protocol != protocol {
            continue;
        }

        // Try to match the implementing type against for_type
        let mut bindings = HashMap::new();
        if match_pattern(mir, witness_def.implementing_type, for_type, &mut bindings).is_ok() {
            return Ok(WitnessMatch {
                witness_id,
                type_bindings: bindings,
            });
        }
    }

    // Get human-readable names for the error message
    let protocol_name = Some(mir.name(protocol).to_string());
    let type_name = Some(format!("{}", mir.ty(for_type).display(mir)));

    Err(MonomorphizeError::WitnessNotFound {
        protocol,
        for_type,
        protocol_name,
        type_name,
    })
}

/// Pattern match a witness type against a concrete type to extract type parameter bindings.
///
/// For example:
/// - Pattern `Box[T]`, Concrete `Box[Int]` → binds `T → Int`
/// - Pattern `Int`, Concrete `Int` → matches, no bindings
/// - Pattern `T`, Concrete `String` → binds `T → String`
/// - Pattern `Box[T]`, Concrete `Option[Int]` → fails
fn match_pattern(
    mir: &MirContext,
    pattern: Id<Ty>,
    concrete: Id<Ty>,
    bindings: &mut HashMap<Id<TypeParam>, Id<Ty>>,
) -> Result<(), MonomorphizeError> {
    let pattern_ty = mir.ty(pattern);
    let concrete_ty = mir.ty(concrete);

    match (pattern_ty, concrete_ty) {
        // Type parameter in pattern - bind it to the concrete type
        (MirTy::TypeParam(tp), _) => {
            // Check for conflicting bindings
            if let Some(&existing) = bindings.get(tp) {
                if existing != concrete {
                    return Err(MonomorphizeError::TypeMismatch {
                        expected: existing,
                        found: concrete,
                    });
                }
            } else {
                bindings.insert(*tp, concrete);
            }
            Ok(())
        },

        // SelfType in pattern - matches any concrete type
        // (SelfType is used in protocol method signatures)
        (MirTy::SelfType, _) => Ok(()),

        // Named types - match name and recurse into type args
        (
            MirTy::Named {
                name: n1,
                type_args: args1,
            },
            MirTy::Named {
                name: n2,
                type_args: args2,
            },
        ) => {
            if n1 != n2 {
                return Err(MonomorphizeError::TypeMismatch {
                    expected: pattern,
                    found: concrete,
                });
            }
            if args1.len() != args2.len() {
                return Err(MonomorphizeError::TypeMismatch {
                    expected: pattern,
                    found: concrete,
                });
            }
            for (a1, a2) in args1.iter().zip(args2.iter()) {
                match_pattern(mir, *a1, *a2, bindings)?;
            }
            Ok(())
        },

        // Structural types - recurse
        (MirTy::Ref(a), MirTy::Ref(b)) => match_pattern(mir, *a, *b, bindings),
        (MirTy::RefMut(a), MirTy::RefMut(b)) => match_pattern(mir, *a, *b, bindings),
        (MirTy::Pointer(a), MirTy::Pointer(b)) => match_pattern(mir, *a, *b, bindings),

        // Tuples - match element-wise
        (MirTy::Tuple(elems1), MirTy::Tuple(elems2)) => {
            if elems1.len() != elems2.len() {
                return Err(MonomorphizeError::TypeMismatch {
                    expected: pattern,
                    found: concrete,
                });
            }
            for (e1, e2) in elems1.iter().zip(elems2.iter()) {
                match_pattern(mir, *e1, *e2, bindings)?;
            }
            Ok(())
        },

        // Function types - match params and return
        (
            MirTy::FuncThin {
                params: p1,
                ret: r1,
            },
            MirTy::FuncThin {
                params: p2,
                ret: r2,
            },
        ) => {
            if p1.len() != p2.len() {
                return Err(MonomorphizeError::TypeMismatch {
                    expected: pattern,
                    found: concrete,
                });
            }
            for (p1, p2) in p1.iter().zip(p2.iter()) {
                match_pattern(mir, *p1, *p2, bindings)?;
            }
            match_pattern(mir, *r1, *r2, bindings)
        },

        (
            MirTy::FuncThick {
                params: p1,
                ret: r1,
            },
            MirTy::FuncThick {
                params: p2,
                ret: r2,
            },
        ) => {
            if p1.len() != p2.len() {
                return Err(MonomorphizeError::TypeMismatch {
                    expected: pattern,
                    found: concrete,
                });
            }
            for (p1, p2) in p1.iter().zip(p2.iter()) {
                match_pattern(mir, *p1, *p2, bindings)?;
            }
            match_pattern(mir, *r1, *r2, bindings)
        },

        // Primitives - must be equal
        (MirTy::I8, MirTy::I8) => Ok(()),
        (MirTy::I16, MirTy::I16) => Ok(()),
        (MirTy::I32, MirTy::I32) => Ok(()),
        (MirTy::I64, MirTy::I64) => Ok(()),
        (MirTy::F16, MirTy::F16) => Ok(()),
        (MirTy::F32, MirTy::F32) => Ok(()),
        (MirTy::F64, MirTy::F64) => Ok(()),
        (MirTy::Bool, MirTy::Bool) => Ok(()),
        (MirTy::Unit, MirTy::Unit) => Ok(()),
        (MirTy::Never, MirTy::Never) => Ok(()),
        (MirTy::Str, MirTy::Str) => Ok(()),
        (MirTy::Error, MirTy::Error) => Ok(()),

        // Associated type projections - not expected in witness patterns,
        // but handle for completeness
        (
            MirTy::AssociatedTypeProjection {
                base: b1,
                protocol: p1,
                associated: a1,
            },
            MirTy::AssociatedTypeProjection {
                base: b2,
                protocol: p2,
                associated: a2,
            },
        ) => {
            if p1 != p2 || a1 != a2 {
                return Err(MonomorphizeError::TypeMismatch {
                    expected: pattern,
                    found: concrete,
                });
            }
            match_pattern(mir, *b1, *b2, bindings)
        },

        // Anything else doesn't match
        _ => Err(MonomorphizeError::TypeMismatch {
            expected: pattern,
            found: concrete,
        }),
    }
}

/// Resolve a witness method call to a direct call.
///
/// Given a witness call like `Cloneable.clone for Box[Int]`, this:
/// 1. Finds the witness `Box[T]: Cloneable`
/// 2. Extracts the binding `T → Int`
/// 3. Looks up the method binding for `clone` → `Box.clone`
/// 4. Returns `(Box.clone, [Int])` as the direct call target
///
/// The returned type arguments are the bindings for the witness's type parameters,
/// in the order they appear in the witness definition.
pub fn resolve_witness(
    mir: &MirContext,
    protocol: Id<QualifiedName>,
    method: &str,
    for_type: Id<Ty>,
) -> Result<(Id<QualifiedName>, Vec<Id<Ty>>), MonomorphizeError> {
    // Resolve associated type projections before witness lookup.
    let mut resolved_for_type = for_type;
    loop {
        match mir.ty(resolved_for_type) {
            MirTy::AssociatedTypeProjection {
                base,
                protocol,
                associated,
            } => {
                resolved_for_type = resolve_associated_type(
                    mir,
                    *base,
                    *protocol,
                    associated,
                )?;
            },
            _ => break,
        }
    }

    // Find the witness
    let witness_match = find_witness(mir, protocol, resolved_for_type)?;
    let witness_def = &mir.witnesses[witness_match.witness_id];

    // Look up the method binding
    let (impl_func_name, method_type_args) =
        witness_def.method_bindings.get(method).ok_or_else(|| {
            MonomorphizeError::MethodNotFoundInWitness {
                protocol,
                method: method.to_string(),
                for_type: resolved_for_type,
                protocol_name: Some(mir.name(protocol).to_string()),
                type_name: Some(format!("{}", mir.ty(resolved_for_type).display(mir))),
            }
        })?;

    // Build the type arguments:
    // 1. First, the witness's type params (from pattern matching the implementing type)
    // 2. Then, any method-specific type args (e.g., Self=X for protocol extension methods)
    let mut type_args: Vec<_> = witness_def
        .type_params
        .iter()
        .map(|tp| {
            witness_match
                .type_bindings
                .get(tp)
                .copied()
                // If a type param wasn't bound, it means it wasn't used in the
                // implementing type pattern (rare but possible). In this case,
                // we can't determine the type arg - this is an error.
                .unwrap_or_else(|| {
                    // This shouldn't happen with well-formed witnesses
                    panic!(
                        "witness type param {:?} not bound during matching",
                        mir.type_param(*tp).name
                    )
                })
        })
        .collect();

    // Add method-specific type arguments (e.g., Self binding for protocol extension methods)
    type_args.extend(method_type_args.iter().cloned());

    Ok((*impl_func_name, type_args))
}

/// Resolve an associated type projection to its concrete type.
///
/// Given a projection like `T.Element` where `T: Container` and `T` is substituted
/// to `MyVec`, this finds the witness `MyVec: Container` and looks up the binding
/// for `Element`.
pub fn resolve_associated_type(
    mir: &MirContext,
    base_type: Id<Ty>,
    protocol: Id<QualifiedName>,
    associated: &str,
) -> Result<Id<Ty>, MonomorphizeError> {
    // Find the witness
    let witness_match = find_witness(mir, protocol, base_type)?;
    let witness_def = &mir.witnesses[witness_match.witness_id];

    // Look up the associated type binding
    let assoc_ty = witness_def.type_bindings.get(associated).ok_or_else(|| {
        MonomorphizeError::MethodNotFoundInWitness {
            protocol,
            method: format!("type {}", associated),
            for_type: base_type,
            protocol_name: Some(mir.name(protocol).to_string()),
            type_name: Some(format!("{}", mir.ty(base_type).display(mir))),
        }
    })?;

    // If the witness has type parameters, we need to substitute them
    // in the associated type binding
    if !witness_def.type_params.is_empty() {
        // The associated type might reference the witness's type params
        // e.g., witness Box[T]: Container { type Element = T }
        // If we matched Box[Int], we need to substitute T → Int
        let mut subst = Substitution::new();
        for tp in &witness_def.type_params {
            if let Some(&binding) = witness_match.type_bindings.get(tp) {
                subst.insert(*tp, binding);
            }
        }

        // We can't intern new types here since we only have &MirContext,
        // but the associated type should already be a concrete type or
        // only reference the witness's own type params which we can substitute.
        // If the type was already interned (which it should be), lookup will work.
        subst
            .apply_ty_readonly(mir, *assoc_ty)
            .map_err(|e| MonomorphizeError::TypeNotInterned {
                description: format!(
                    "associated type {} in witness for {:?}: {}",
                    associated, protocol, e
                ),
            })
    } else {
        Ok(*assoc_ty)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_match_pattern_primitives() {

        // We can't easily test primitives without interning,
        // so this is a placeholder for more comprehensive tests
        // that would be added with a proper test fixture.
    }

    // More comprehensive tests would require setting up a MirContext
    // with witnesses, which is better done as integration tests.
}
