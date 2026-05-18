//! MIR type representation.
//!
//! Types are by value — no interning, no `Id<Ty>` indirection.
//! Entity references point into the ECS for struct/enum/protocol/type-param identity.

use std::collections::HashMap;

use crate::MirModule;
use crate::item::{CopyBehavior, WhereClause, WhereConstraint};
use kestrel_hecs::Entity;

/// MIR type representation.
///
/// Used by value wherever types appear. `Entity` references resolve to names
/// via `MirModule.entity_names` for display.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MirTy {
    // === Primitives ===
    I8,
    I16,
    I32,
    I64,
    F16,
    F32,
    F64,
    Bool,
    Never,
    Str,

    // === Pointers and references ===
    /// `p[T]` — raw pointer
    Pointer(Box<MirTy>),
    /// `&T` — immutable reference
    Ref(Box<MirTy>),
    /// `&var T` — mutable reference
    RefMut(Box<MirTy>),

    // === Compound ===
    /// `(T1, T2, ...)` — tuple type
    Tuple(Vec<MirTy>),

    /// Named type (struct, enum, protocol) with optional type arguments.
    Named {
        entity: Entity,
        type_args: Vec<MirTy>,
    },

    // === Generics ===
    /// Type parameter reference.
    TypeParam(Entity),

    /// `Self` — the implementing type in a protocol context.
    SelfType,

    /// Associated type projection: `T.Element` where `T: Container`.
    /// Resolved during monomorphization via witness table lookup.
    AssociatedProjection {
        base: Box<MirTy>,
        protocol: Entity,
        name: String,
    },

    // === Function types ===
    /// Thin function pointer (no environment, FFI-safe).
    FuncThin {
        params: Vec<MirTy>,
        ret: Box<MirTy>,
    },

    /// Thick callable (has environment, can escape).
    FuncThick {
        params: Vec<MirTy>,
        ret: Box<MirTy>,
    },

    /// Error/poison type — used when lowering fails.
    Error,
}

impl MirTy {
    /// Canonical unit value — the empty tuple.
    ///
    /// MIR has no `Unit` variant; `()` *is* `Tuple([])`. HIR already uses this
    /// representation (`AstType::Unit → HirTy::Tuple(Vec::new(), …)`), so
    /// keeping MIR in sync removes a class of "which form did this come through
    /// as?" bugs at the HIR→MIR boundary.
    pub fn unit() -> Self {
        MirTy::Tuple(Vec::new())
    }

    /// Check if this is the unit type (empty tuple).
    pub fn is_unit(&self) -> bool {
        matches!(self, MirTy::Tuple(elems) if elems.is_empty())
    }

    /// Check if this is a primitive integer type.
    pub fn is_integer(&self) -> bool {
        matches!(self, MirTy::I8 | MirTy::I16 | MirTy::I32 | MirTy::I64)
    }

    /// Check if this is a primitive float type.
    pub fn is_float(&self) -> bool {
        matches!(self, MirTy::F16 | MirTy::F32 | MirTy::F64)
    }

    /// Check if this is a reference type (immutable or mutable).
    pub fn is_reference(&self) -> bool {
        matches!(self, MirTy::Ref(_) | MirTy::RefMut(_))
    }

    /// Check if this is a pointer type.
    pub fn is_pointer(&self) -> bool {
        matches!(self, MirTy::Pointer(_))
    }

    /// Recursively check whether this type contains `MirTy::Error` anywhere.
    /// Used by MIR-lower's validate pass (hard ICE backstop) and anywhere else
    /// that needs to detect an unresolved-type leak from an upstream phase.
    pub fn contains_error(&self) -> bool {
        match self {
            MirTy::Error => true,
            MirTy::Named { type_args, .. } => type_args.iter().any(MirTy::contains_error),
            MirTy::Tuple(tys) => tys.iter().any(MirTy::contains_error),
            MirTy::Pointer(inner) | MirTy::Ref(inner) | MirTy::RefMut(inner) => {
                inner.contains_error()
            },
            MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
                params.iter().any(MirTy::contains_error) || ret.contains_error()
            },
            MirTy::AssociatedProjection { base, .. } => base.contains_error(),
            MirTy::TypeParam(_)
            | MirTy::SelfType
            | MirTy::Never
            | MirTy::Bool
            | MirTy::I8
            | MirTy::I16
            | MirTy::I32
            | MirTy::I64
            | MirTy::F16
            | MirTy::F32
            | MirTy::F64
            | MirTy::Str => false,
        }
    }

    /// Check if this type is trivially copyable (passed by value, no ownership transfer).
    /// Includes primitives, refs, pointers, and thin function pointers.
    pub fn is_trivially_copyable(&self) -> bool {
        if self.is_unit() {
            return true;
        }
        matches!(
            self,
            MirTy::I8
                | MirTy::I16
                | MirTy::I32
                | MirTy::I64
                | MirTy::F16
                | MirTy::F32
                | MirTy::F64
                | MirTy::Bool
                | MirTy::Never
                | MirTy::Ref(_)
                | MirTy::RefMut(_)
                | MirTy::Pointer(_)
                | MirTy::FuncThin { .. }
        )
    }

    /// Compute this type's [`CopyBehavior`] using its definition in `module`.
    ///
    /// - Primitives, refs (`&T`/`&var T`), raw pointers, thin function pointers,
    ///   `Str`, `Never`, and unit tuples are [`CopyBehavior::Bitwise`].
    /// - [`MirTy::Named`] resolves to the matching `StructDef`/`EnumDef`'s
    ///   `copy_behavior` field. Returns [`CopyBehavior::None`] if no def is
    ///   found (so unresolved references fail safe).
    /// - [`MirTy::Tuple`] composes structurally: any `None` element makes the
    ///   tuple `None`; any non-`Bitwise` element makes the tuple non-`Bitwise`
    ///   (collapsed to a synthetic `Clone` marker — TODO: distinguish
    ///   structural compound clones from method clones).
    /// - [`MirTy::TypeParam`], [`MirTy::SelfType`], [`MirTy::AssociatedProjection`]
    ///   conservatively return `None`; a constraint-aware variant will be added
    ///   in Stage 6.
    /// - [`MirTy::FuncThick`] is composed from its env behavior — for now,
    ///   conservatively `None` until closure-env propagation lands.
    /// - [`MirTy::Error`] returns `None`.
    pub fn copy_behavior(&self, module: &MirModule) -> CopyBehavior {
        self.copy_behavior_inner(module, 0)
    }

    /// Inner recursive implementation with a depth guard. Direct
    /// nominal recursion is illegal in well-formed MIR (sizes would be
    /// infinite), but cycles through pointer/ref break naturally because
    /// those are always Bitwise and don't recurse. The depth guard
    /// defends against pathological inputs we haven't yet rejected.
    fn copy_behavior_inner(&self, module: &MirModule, depth: usize) -> CopyBehavior {
        if depth > 64 {
            return CopyBehavior::None;
        }
        match self {
            // Primitives + bitwise types. References — both `Ref(_)`
            // (`&T`) and `RefMut(_)` (`&var T`) — are Bitwise: copying a
            // reference duplicates the pointer, and Kestrel deliberately
            // allows reference aliasing (no Rust-style exclusivity
            // check). The borrow itself is just a pointer; mutating
            // through it does what mutating through any pointer does.
            // Two `&var T`s to the same place are legal at the type
            // system level — region/lifetime analysis is a separate
            // Phase 2 problem.
            MirTy::I8
            | MirTy::I16
            | MirTy::I32
            | MirTy::I64
            | MirTy::F16
            | MirTy::F32
            | MirTy::F64
            | MirTy::Bool
            | MirTy::Never
            | MirTy::Str
            | MirTy::Ref(_)
            | MirTy::RefMut(_)
            | MirTy::Pointer(_)
            | MirTy::FuncThin { .. } => CopyBehavior::Bitwise,

            // Tuples compose structurally. Unit (empty tuple) is Bitwise.
            MirTy::Tuple(elems) => {
                let mut kind = CopyBehavior::Bitwise;
                for elem in elems {
                    match elem.copy_behavior_inner(module, depth + 1) {
                        CopyBehavior::None => return CopyBehavior::None,
                        CopyBehavior::Clone(e) => {
                            // First non-Bitwise child wins the marker; later
                            // structural-clone work uses the per-element
                            // behavior, not this marker.
                            if matches!(kind, CopyBehavior::Bitwise) {
                                kind = CopyBehavior::Clone(e);
                            }
                        },
                        CopyBehavior::Bitwise => {},
                    }
                }
                kind
            },

            // Named types: look up the matching def, then refine using the
            // actual type_args. The def's `copy_behavior` is computed for
            // the *uninstantiated* form (`Result[T, E]` where T/E are
            // TypeParam) and assumes type params default to Copyable — so
            // `Result[Thing, Int64]` would inherit Result's Bitwise marker
            // even though Thing is `not Copyable`. Substitute the args into
            // the field/payload types and recurse so the instantiation's
            // real behavior surfaces.
            MirTy::Named { entity, type_args } => {
                named_copy_behavior(*entity, type_args, module, depth)
            },

            // Conservatively non-copyable. Generic-aware classification
            // happens via [`Self::copy_behavior_with_constraints`].
            MirTy::TypeParam(_)
            | MirTy::SelfType
            | MirTy::AssociatedProjection { .. }
            | MirTy::FuncThick { .. }
            | MirTy::Error => CopyBehavior::None,
        }
    }

    /// Same as [`Self::copy_behavior`], but consults a [`WhereClause`] so
    /// `TypeParam(T)` constrained by `Copyable`/`Cloneable` reports a non-
    /// `None` behavior.
    ///
    /// Used by the MIR verifier to decide whether `Value::Move(p)` is legal
    /// in a generic body: `let f = move x` on `x: T where T: Copyable` is
    /// rejected because `T` may resolve to a copyable type.
    pub fn copy_behavior_with_constraints(
        &self,
        module: &MirModule,
        where_clause: Option<&WhereClause>,
    ) -> CopyBehavior {
        if let MirTy::TypeParam(entity) = self {
            return type_param_copy_behavior(*entity, module, where_clause);
        }
        // Tuples recurse with constraints; everything else reuses the
        // non-constraint path (no TypeParam at the leaves).
        if let MirTy::Tuple(elems) = self {
            let mut kind = CopyBehavior::Bitwise;
            for elem in elems {
                match elem.copy_behavior_with_constraints(module, where_clause) {
                    CopyBehavior::None => return CopyBehavior::None,
                    CopyBehavior::Clone(e) => {
                        if matches!(kind, CopyBehavior::Bitwise) {
                            kind = CopyBehavior::Clone(e);
                        }
                    },
                    CopyBehavior::Bitwise => {},
                }
            }
            return kind;
        }
        self.copy_behavior(module)
    }
}

/// Compute copy behavior for `MirTy::Named { entity, type_args }` by
/// looking up the matching struct/enum def and folding over its fields
/// (or its cases' payload structs) with `type_args` substituted in.
///
/// An explicit `CopyBehavior::None` on the def always wins — those are
/// types that declared `: not Copyable`, and no choice of type args can
/// recover Copyable. For any other declared behavior, the structural
/// fold over the instantiated payload types decides: any None field
/// makes the whole thing None; any non-Bitwise field bumps the result
/// to Clone (with the def's clone-method marker preserved).
fn named_copy_behavior(
    entity: Entity,
    type_args: &[MirTy],
    module: &MirModule,
    depth: usize,
) -> CopyBehavior {
    // Struct path.
    if let Some(s) = module.structs.iter().find(|s| s.entity == entity) {
        if matches!(s.copy_behavior, CopyBehavior::None) {
            return CopyBehavior::None;
        }
        let subst = build_type_param_subst(&s.type_params.iter().map(|p| p.entity).collect::<Vec<_>>(), type_args);
        return fold_field_copy_behavior(
            s.copy_behavior.clone(),
            s.fields.iter().map(|f| &f.ty),
            &subst,
            module,
            depth,
        );
    }
    // Enum path: fold across every case's payload struct's fields.
    if let Some(e) = module.enums.iter().find(|e| e.entity == entity) {
        if matches!(e.copy_behavior, CopyBehavior::None) {
            return CopyBehavior::None;
        }
        let subst = build_type_param_subst(&e.type_params.iter().map(|p| p.entity).collect::<Vec<_>>(), type_args);
        let mut payload_tys: Vec<MirTy> = Vec::new();
        for case in &e.cases {
            let idx = case.payload_struct.index();
            if let Some(payload_struct) = module.structs.get(idx) {
                for f in &payload_struct.fields {
                    payload_tys.push(f.ty.clone());
                }
            }
        }
        return fold_field_copy_behavior(
            e.copy_behavior.clone(),
            payload_tys.iter(),
            &subst,
            module,
            depth,
        );
    }
    // Unknown nominal — fail safe.
    CopyBehavior::None
}

/// Substitute type-params in `ty` using the (param-entity → arg) map.
/// Returns `ty` unchanged when no substitution applies.
fn substitute_type_params(ty: &MirTy, subst: &HashMap<Entity, MirTy>) -> MirTy {
    match ty {
        MirTy::TypeParam(e) => subst.get(e).cloned().unwrap_or_else(|| ty.clone()),
        MirTy::Pointer(inner) => MirTy::Pointer(Box::new(substitute_type_params(inner, subst))),
        MirTy::Ref(inner) => MirTy::Ref(Box::new(substitute_type_params(inner, subst))),
        MirTy::RefMut(inner) => MirTy::RefMut(Box::new(substitute_type_params(inner, subst))),
        MirTy::Tuple(elems) => MirTy::Tuple(
            elems.iter().map(|t| substitute_type_params(t, subst)).collect(),
        ),
        MirTy::Named { entity, type_args } => MirTy::Named {
            entity: *entity,
            type_args: type_args.iter().map(|t| substitute_type_params(t, subst)).collect(),
        },
        MirTy::FuncThin { params, ret } => MirTy::FuncThin {
            params: params.iter().map(|t| substitute_type_params(t, subst)).collect(),
            ret: Box::new(substitute_type_params(ret, subst)),
        },
        MirTy::FuncThick { params, ret } => MirTy::FuncThick {
            params: params.iter().map(|t| substitute_type_params(t, subst)).collect(),
            ret: Box::new(substitute_type_params(ret, subst)),
        },
        _ => ty.clone(),
    }
}

fn build_type_param_subst(params: &[Entity], args: &[MirTy]) -> HashMap<Entity, MirTy> {
    params
        .iter()
        .zip(args.iter())
        .map(|(p, t)| (*p, t.clone()))
        .collect()
}

fn fold_field_copy_behavior<'a, I: IntoIterator<Item = &'a MirTy>>(
    def_kind: CopyBehavior,
    field_tys: I,
    subst: &HashMap<Entity, MirTy>,
    module: &MirModule,
    depth: usize,
) -> CopyBehavior {
    let mut kind = def_kind;
    for field_ty in field_tys {
        let substituted = substitute_type_params(field_ty, subst);
        match substituted.copy_behavior_inner(module, depth + 1) {
            CopyBehavior::None => return CopyBehavior::None,
            CopyBehavior::Clone(e) => {
                if matches!(kind, CopyBehavior::Bitwise) {
                    kind = CopyBehavior::Clone(e);
                }
            },
            CopyBehavior::Bitwise => {},
        }
    }
    kind
}

/// Walk the where clause for `T: P` constraints and check whether `P`
/// (or any of its parent protocols) is the builtin `Copyable` or
/// `Cloneable`. Defaults to `CopyBehavior::Bitwise` for unconstrained type
/// parameters — Kestrel's type system requires `T` to be `Copyable` unless
/// the user explicitly opts out via `where T: not Copyable`, so the move-
/// check treats unconstrained `T` as Copy (which avoids false positives on
/// `let y = self.t; self.t = self.t.successor()` patterns in generic stdlib
/// iterator code). An explicit `T: not Copyable` `NegativeBound` flips the
/// answer back to `None` so move-check tracks the affine binding.
fn type_param_copy_behavior(
    type_param: Entity,
    module: &MirModule,
    where_clause: Option<&WhereClause>,
) -> CopyBehavior {
    let Some(where_clause) = where_clause else {
        // Without constraint info we can't see a `: not Copyable` bound,
        // so default to the conservative "may be non-Copy" answer — this
        // path is the verifier's view, where treating an unconstrained
        // generic as affine is fine (rejecting `Value::Move` on it would
        // be over-strict but the verifier path doesn't reach here for
        // typical generic locals).
        return CopyBehavior::None;
    };
    // Explicit opt-out wins over everything: `T: not Copyable` makes the
    // type-param affine even if other clauses happen to mention Copyable.
    for constraint in &where_clause.constraints {
        if let WhereConstraint::NotImplements {
            type_param: tp,
            protocol,
        } = constraint
            && *tp == type_param
            && protocol_implies_copyable(*protocol, module)
        {
            return CopyBehavior::None;
        }
    }
    for constraint in &where_clause.constraints {
        let WhereConstraint::Implements {
            type_param: tp,
            protocol,
        } = constraint
        else {
            continue;
        };
        if *tp != type_param {
            continue;
        }
        if protocol_implies_copyable(*protocol, module) {
            return CopyBehavior::Bitwise;
        }
    }
    // Unconstrained type parameters default to Copyable in Kestrel's
    // semantics; only explicit `: not Copyable` opts out.
    CopyBehavior::Bitwise
}

/// True if `protocol` is `Copyable`/`Cloneable` or transitively extends
/// either via `parent_protocols`. Identified by qualified name suffix
/// — matches the builtin std-core protocols.
fn protocol_implies_copyable(protocol: Entity, module: &MirModule) -> bool {
    fn is_copyable_or_cloneable_name(name: &str) -> bool {
        // Match the fully-qualified `std.core.Copyable` / `.Cloneable` and
        // accept any module-suffix variant the test harness might use.
        let short = name.rsplit('.').next().unwrap_or(name);
        short == "Copyable" || short == "Cloneable"
    }
    let proto = module.protocols.iter().find(|p| p.entity == protocol);
    let Some(proto) = proto else {
        // Unknown protocol — fall back to name lookup in case it isn't in
        // the protocols vec yet (forward declarations during lowering).
        return is_copyable_or_cloneable_name(module.resolve_name(protocol));
    };
    if is_copyable_or_cloneable_name(&proto.name) {
        return true;
    }
    proto
        .parent_protocols
        .iter()
        .any(|p| protocol_implies_copyable(*p, module))
}
