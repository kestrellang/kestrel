//! Type substitution for monomorphization.
//!
//! This module provides the `Substitution` type which maps type parameters
//! to concrete types, and methods to apply this substitution to types.

use super::error::MonomorphizeError;
use kestrel_execution_graph::{Id, MirContext, MirTy, Ty, TypeParam};
use std::collections::HashMap;

/// A substitution mapping type parameters to concrete types.
///
/// During monomorphization, when we instantiate a generic function like
/// `identity[Int]`, we create a substitution `{T → Int}` and apply it
/// throughout the function body.
#[derive(Debug, Clone, Default)]
pub struct Substitution {
    /// Mapping from type parameter IDs to concrete types.
    mapping: HashMap<Id<TypeParam>, Id<Ty>>,
    /// Optional mapping for `Self` type (used in witness contexts).
    self_type: Option<Id<Ty>>,
}

impl Substitution {
    /// Create a new empty substitution.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if this substitution is empty.
    pub fn is_empty(&self) -> bool {
        self.mapping.is_empty() && self.self_type.is_none()
    }

    /// Insert a type parameter mapping.
    pub fn insert(&mut self, tp: Id<TypeParam>, ty: Id<Ty>) {
        self.mapping.insert(tp, ty);
    }

    /// Get the substitution for a type parameter.
    pub fn get(&self, tp: Id<TypeParam>) -> Option<Id<Ty>> {
        self.mapping.get(&tp).copied()
    }

    /// Set the substitution for `Self` type.
    pub fn set_self_type(&mut self, ty: Id<Ty>) {
        self.self_type = Some(ty);
    }

    /// Get the substitution for `Self` type.
    pub fn get_self_type(&self) -> Option<Id<Ty>> {
        self.self_type
    }

    /// Apply this substitution to a type, interning new types as needed.
    ///
    /// This is used during the collection phase when we need to create
    /// new interned types for substituted type expressions.
    pub fn apply_ty(&self, mir: &mut MirContext, ty: Id<Ty>) -> Id<Ty> {
        let mir_ty = mir.ty(ty).clone();

        match mir_ty {
            // Primitives - no substitution needed
            MirTy::I8
            | MirTy::I16
            | MirTy::I32
            | MirTy::I64
            | MirTy::F16
            | MirTy::F32
            | MirTy::F64
            | MirTy::Bool
            | MirTy::Unit
            | MirTy::Never
            | MirTy::Str
            | MirTy::Error => ty,

            // Type parameter - look up in substitution
            MirTy::TypeParam(tp) => self.mapping.get(&tp).copied().unwrap_or(ty),

            // Self type - look up in self_type mapping
            MirTy::SelfType => self.self_type.unwrap_or(ty),

            // Pointer types - recurse into inner type
            MirTy::Pointer(inner) => {
                let new_inner = self.apply_ty(mir, inner);
                if new_inner == inner {
                    ty
                } else {
                    mir.intern_type(MirTy::Pointer(new_inner))
                }
            }

            MirTy::Ref(inner) => {
                let new_inner = self.apply_ty(mir, inner);
                if new_inner == inner {
                    ty
                } else {
                    mir.intern_type(MirTy::Ref(new_inner))
                }
            }

            MirTy::RefMut(inner) => {
                let new_inner = self.apply_ty(mir, inner);
                if new_inner == inner {
                    ty
                } else {
                    mir.intern_type(MirTy::RefMut(new_inner))
                }
            }

            // Tuple - recurse into each element
            MirTy::Tuple(elems) => {
                let new_elems: Vec<_> = elems.iter().map(|e| self.apply_ty(mir, *e)).collect();
                if new_elems == elems {
                    ty
                } else {
                    mir.intern_type(MirTy::Tuple(new_elems))
                }
            }

            // Named type - recurse into type arguments
            MirTy::Named { name, type_args } => {
                let new_args: Vec<_> = type_args.iter().map(|a| self.apply_ty(mir, *a)).collect();
                if new_args == type_args {
                    ty
                } else {
                    mir.intern_type(MirTy::Named {
                        name,
                        type_args: new_args,
                    })
                }
            }

            // Function types - recurse into params and return type
            MirTy::FuncThin { params, ret } => {
                let new_params: Vec<_> = params.iter().map(|p| self.apply_ty(mir, *p)).collect();
                let new_ret = self.apply_ty(mir, ret);
                if new_params == params && new_ret == ret {
                    ty
                } else {
                    mir.intern_type(MirTy::FuncThin {
                        params: new_params,
                        ret: new_ret,
                    })
                }
            }

            MirTy::FuncThick { params, ret } => {
                let new_params: Vec<_> = params.iter().map(|p| self.apply_ty(mir, *p)).collect();
                let new_ret = self.apply_ty(mir, ret);
                if new_params == params && new_ret == ret {
                    ty
                } else {
                    mir.intern_type(MirTy::FuncThick {
                        params: new_params,
                        ret: new_ret,
                    })
                }
            }

            // Associated type projection - substitute the base type
            // Note: Full resolution of associated types requires witness lookup,
            // which is handled separately. Here we just substitute the base.
            MirTy::AssociatedTypeProjection {
                base,
                protocol,
                associated,
            } => {
                let new_base = self.apply_ty(mir, base);
                if new_base == base {
                    ty
                } else {
                    mir.intern_type(MirTy::AssociatedTypeProjection {
                        base: new_base,
                        protocol,
                        associated,
                    })
                }
            }
        }
    }

    /// Apply this substitution to a type, looking up types without interning.
    ///
    /// This is used during the codegen phase when we don't want to mutate
    /// the MIR context. All types needed should have been interned during
    /// the collection phase.
    ///
    /// Returns an error if a required type hasn't been interned.
    pub fn apply_ty_readonly(
        &self,
        mir: &MirContext,
        ty: Id<Ty>,
    ) -> Result<Id<Ty>, MonomorphizeError> {
        let mir_ty = mir.ty(ty);

        match mir_ty {
            // Primitives - no substitution needed
            MirTy::I8
            | MirTy::I16
            | MirTy::I32
            | MirTy::I64
            | MirTy::F16
            | MirTy::F32
            | MirTy::F64
            | MirTy::Bool
            | MirTy::Unit
            | MirTy::Never
            | MirTy::Str
            | MirTy::Error => Ok(ty),

            // Type parameter - look up in substitution
            MirTy::TypeParam(tp) => Ok(self.mapping.get(tp).copied().unwrap_or(ty)),

            // Self type - look up in self_type mapping
            MirTy::SelfType => Ok(self.self_type.unwrap_or(ty)),

            // Pointer types - recurse into inner type
            MirTy::Pointer(inner) => {
                let new_inner = self.apply_ty_readonly(mir, *inner)?;
                if new_inner == *inner {
                    Ok(ty)
                } else {
                    mir.lookup_type(&MirTy::Pointer(new_inner)).ok_or_else(|| {
                        MonomorphizeError::TypeNotInterned {
                            description: format!("Pointer({:?})", new_inner),
                        }
                    })
                }
            }

            MirTy::Ref(inner) => {
                let new_inner = self.apply_ty_readonly(mir, *inner)?;
                if new_inner == *inner {
                    Ok(ty)
                } else {
                    mir.lookup_type(&MirTy::Ref(new_inner)).ok_or_else(|| {
                        MonomorphizeError::TypeNotInterned {
                            description: format!("Ref({:?})", new_inner),
                        }
                    })
                }
            }

            MirTy::RefMut(inner) => {
                let new_inner = self.apply_ty_readonly(mir, *inner)?;
                if new_inner == *inner {
                    Ok(ty)
                } else {
                    mir.lookup_type(&MirTy::RefMut(new_inner)).ok_or_else(|| {
                        MonomorphizeError::TypeNotInterned {
                            description: format!("RefMut({:?})", new_inner),
                        }
                    })
                }
            }

            // Tuple - recurse into each element
            MirTy::Tuple(elems) => {
                let new_elems: Vec<_> = elems
                    .iter()
                    .map(|e| self.apply_ty_readonly(mir, *e))
                    .collect::<Result<_, _>>()?;
                if new_elems == *elems {
                    Ok(ty)
                } else {
                    mir.lookup_type(&MirTy::Tuple(new_elems.clone()))
                        .ok_or_else(|| MonomorphizeError::TypeNotInterned {
                            description: format!("Tuple({:?})", new_elems),
                        })
                }
            }

            // Named type - recurse into type arguments
            MirTy::Named { name, type_args } => {
                let new_args: Vec<_> = type_args
                    .iter()
                    .map(|a| self.apply_ty_readonly(mir, *a))
                    .collect::<Result<_, _>>()?;
                if new_args == *type_args {
                    Ok(ty)
                } else {
                    mir.lookup_type(&MirTy::Named {
                        name: *name,
                        type_args: new_args.clone(),
                    })
                    .ok_or_else(|| MonomorphizeError::TypeNotInterned {
                        description: format!("Named({:?}, {:?})", name, new_args),
                    })
                }
            }

            // Function types - recurse into params and return type
            MirTy::FuncThin { params, ret } => {
                let new_params: Vec<_> = params
                    .iter()
                    .map(|p| self.apply_ty_readonly(mir, *p))
                    .collect::<Result<_, _>>()?;
                let new_ret = self.apply_ty_readonly(mir, *ret)?;
                if new_params == *params && new_ret == *ret {
                    Ok(ty)
                } else {
                    mir.lookup_type(&MirTy::FuncThin {
                        params: new_params.clone(),
                        ret: new_ret,
                    })
                    .ok_or_else(|| MonomorphizeError::TypeNotInterned {
                        description: format!("FuncThin({:?}, {:?})", new_params, new_ret),
                    })
                }
            }

            MirTy::FuncThick { params, ret } => {
                let new_params: Vec<_> = params
                    .iter()
                    .map(|p| self.apply_ty_readonly(mir, *p))
                    .collect::<Result<_, _>>()?;
                let new_ret = self.apply_ty_readonly(mir, *ret)?;
                if new_params == *params && new_ret == *ret {
                    Ok(ty)
                } else {
                    mir.lookup_type(&MirTy::FuncThick {
                        params: new_params.clone(),
                        ret: new_ret,
                    })
                    .ok_or_else(|| MonomorphizeError::TypeNotInterned {
                        description: format!("FuncThick({:?}, {:?})", new_params, new_ret),
                    })
                }
            }

            // Associated type projection - substitute the base type
            MirTy::AssociatedTypeProjection {
                base,
                protocol,
                associated,
            } => {
                let new_base = self.apply_ty_readonly(mir, *base)?;
                if new_base == *base {
                    Ok(ty)
                } else {
                    mir.lookup_type(&MirTy::AssociatedTypeProjection {
                        base: new_base,
                        protocol: *protocol,
                        associated: associated.clone(),
                    })
                    .ok_or_else(|| MonomorphizeError::TypeNotInterned {
                        description: format!(
                            "AssociatedTypeProjection({:?}, {:?}, {})",
                            new_base, protocol, associated
                        ),
                    })
                }
            }
        }
    }
}

/// Build a substitution from type parameter definitions and concrete type arguments.
///
/// Given a generic function `func identity[T](x: T) -> T` and a call `identity[Int]`,
/// this creates a substitution `{T → Int}`.
pub fn build_substitution(
    mir: &MirContext,
    type_params: &[Id<TypeParam>],
    type_args: &[Id<Ty>],
) -> Substitution {
    debug_assert_eq!(
        type_params.len(),
        type_args.len(),
        "type params and args length mismatch"
    );

    let mut subst = Substitution::new();
    for (&param, &arg) in type_params.iter().zip(type_args.iter()) {
        subst.insert(param, arg);
    }
    subst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_substitution() {
        let subst = Substitution::new();
        assert!(subst.is_empty());
    }

    #[test]
    fn test_substitution_insert_and_get() {
        let mut subst = Substitution::new();
        let mut mir = MirContext::new();

        // Create a dummy type param
        let tp = mir
            .type_params
            .alloc(kestrel_execution_graph::TypeParamDef {
                meta: Default::default(),
                priors: vec![],
                name: "T".to_string(),
                owner: kestrel_execution_graph::TypeParamOwner::Function(
                    kestrel_execution_graph::Id::from_raw(0),
                ),
            });

        let int_ty = mir.ty_i64();

        subst.insert(tp, int_ty);
        assert!(!subst.is_empty());
        assert_eq!(subst.get(tp), Some(int_ty));
    }

    #[test]
    fn test_apply_ty_primitives_unchanged() {
        let subst = Substitution::new();
        let mut mir = MirContext::new();

        let int_ty = mir.ty_i64();
        let bool_ty = mir.ty_bool();
        let unit_ty = mir.ty_unit();

        // Primitives should be unchanged by empty substitution
        assert_eq!(subst.apply_ty(&mut mir, int_ty), int_ty);
        assert_eq!(subst.apply_ty(&mut mir, bool_ty), bool_ty);
        assert_eq!(subst.apply_ty(&mut mir, unit_ty), unit_ty);
    }

    #[test]
    fn test_apply_ty_type_param() {
        let mut subst = Substitution::new();
        let mut mir = MirContext::new();

        // Create a type param T
        let tp = mir
            .type_params
            .alloc(kestrel_execution_graph::TypeParamDef {
                meta: Default::default(),
                priors: vec![],
                name: "T".to_string(),
                owner: kestrel_execution_graph::TypeParamOwner::Function(
                    kestrel_execution_graph::Id::from_raw(0),
                ),
            });

        let type_param_ty = mir.intern_type(MirTy::TypeParam(tp));
        let int_ty = mir.ty_i64();

        // Before substitution, TypeParam returns itself
        assert_eq!(subst.apply_ty(&mut mir, type_param_ty), type_param_ty);

        // After adding T → Int, TypeParam should return Int
        subst.insert(tp, int_ty);
        assert_eq!(subst.apply_ty(&mut mir, type_param_ty), int_ty);
    }

    #[test]
    fn test_apply_ty_ref() {
        let mut subst = Substitution::new();
        let mut mir = MirContext::new();

        // Create a type param T and &T
        let tp = mir
            .type_params
            .alloc(kestrel_execution_graph::TypeParamDef {
                meta: Default::default(),
                priors: vec![],
                name: "T".to_string(),
                owner: kestrel_execution_graph::TypeParamOwner::Function(
                    kestrel_execution_graph::Id::from_raw(0),
                ),
            });

        let type_param_ty = mir.intern_type(MirTy::TypeParam(tp));
        let ref_t = mir.ty_ref(type_param_ty);

        let int_ty = mir.ty_i64();
        subst.insert(tp, int_ty);

        // &T should become &Int
        let result = subst.apply_ty(&mut mir, ref_t);
        let expected = mir.ty_ref(int_ty);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_apply_ty_self_type() {
        let mut subst = Substitution::new();
        let mut mir = MirContext::new();

        let self_ty = mir.ty_self();
        let int_ty = mir.ty_i64();

        // Before setting self_type, SelfType returns itself
        assert_eq!(subst.apply_ty(&mut mir, self_ty), self_ty);

        // After setting self_type, SelfType should return the concrete type
        subst.set_self_type(int_ty);
        assert_eq!(subst.apply_ty(&mut mir, self_ty), int_ty);
    }
}
