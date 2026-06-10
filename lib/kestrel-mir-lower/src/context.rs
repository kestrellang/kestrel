//! Lowering context — central state during HIR → MIR lowering.

use kestrel_ast_builder::{Callable, EnclosingContainer, Intrinsic, Name, NodeKind, Subscript};
use kestrel_hecs::{Entity, QueryContext, QueryFn, World};
use kestrel_hir::body::{HirExpr, HirExprId};
use kestrel_mir::{FieldIdx, MirModule, MirTy, TyId, VariantIdx, WitnessMethodKey};
use kestrel_name_res::ExtensionTargetEntity;

use crate::name::qualified_name;

/// Central state maintained during lowering.
pub struct LowerCtx<'w> {
    pub world: &'w World,
    pub query: QueryContext<'w>,
    pub root: Entity,
    pub module: MirModule,
    pub closure_counter: u32,
    synthetic_counter: u32,
}

impl<'w> LowerCtx<'w> {
    pub fn new(world: &'w World, root: Entity, name: &str) -> Self {
        let query = world.query_context();
        Self {
            world,
            query,
            root,
            module: MirModule::new(name),
            closure_counter: 0,
            synthetic_counter: 0,
        }
    }

    /// Register an entity's qualified name in the module's name map.
    pub fn register_name(&mut self, entity: Entity) -> String {
        let name = qualified_name(self.world, entity);
        self.module.register_name(entity, name.clone());
        name
    }

    /// Intern a MirTy into the arena, returning its TyId.
    pub fn intern(&mut self, ty: MirTy) -> TyId {
        self.module.ty_arena.intern(ty)
    }

    /// Generate a unique synthetic entity for closures, thunks, etc.
    /// Uses the high end of the u32 range to avoid collisions with real entities.
    pub fn next_synthetic_entity(&mut self) -> Entity {
        let id = self.synthetic_counter;
        self.synthetic_counter += 1;
        let entity = Entity::from_raw(u32::MAX / 2 - id);
        debug_assert!(
            entity.index() > 1_000_000,
            "synthetic entity space exhausted or colliding with real entities"
        );
        entity
    }

    /// Resolve a stored field name to its FieldIdx within a lowered StructDef.
    ///
    /// The struct must already be lowered (items phase completes before bodies).
    /// Panics if the struct or field is not found — field names are validated
    /// upstream by name resolution.
    pub fn resolve_field_idx(&self, struct_entity: Entity, field_name: &str) -> Option<FieldIdx> {
        let def = self.module.structs.get(&struct_entity)?;
        let idx = def.fields.iter().position(|f| f.name == field_name)?;
        Some(FieldIdx::new(idx))
    }

    pub fn resolve_field_ty(&self, struct_entity: Entity, field_idx: FieldIdx) -> Option<TyId> {
        let def = self.module.structs.get(&struct_entity)?;
        def.fields.get(field_idx.index()).map(|f| f.ty)
    }

    /// Resolve an enum case name to its VariantIdx within a lowered EnumDef.
    pub fn resolve_variant_idx(&self, enum_entity: Entity, case_name: &str) -> Option<VariantIdx> {
        let def = self.module.enums.get(&enum_entity)?;
        let idx = def.cases.iter().position(|c| c.name == case_name)?;
        Some(VariantIdx::new(idx))
    }

    // --- hECS query wrappers ---

    /// Check if an entity is a protocol method (direct or extension default).
    /// Returns the protocol entity if so.
    pub fn is_protocol_method(&self, entity: Entity) -> Option<Entity> {
        self.query.query(IsProtocolMethod {
            entity,
            root: self.root,
        })
    }

    /// Build a WitnessMethodKey for a protocol method entity.
    ///
    /// Uses the entity's Name + Callable param labels to construct a key
    /// that matches the witness table's binding keys.
    pub fn witness_method_key(&self, entity: Entity) -> WitnessMethodKey {
        let name = self
            .world
            .get::<Name>(entity)
            .map(|n| n.0.clone())
            .unwrap_or_else(|| {
                if self.world.get::<Subscript>(entity).is_some() {
                    "subscript".to_string()
                } else {
                    "init".to_string()
                }
            });
        let labels = self
            .world
            .get::<Callable>(entity)
            .map(|c| c.params.iter().map(|p| p.label.clone()).collect())
            .unwrap_or_default();
        WitnessMethodKey::new(name, labels)
    }

    /// Build a WitnessMethodKey for a Setter entity by deriving
    /// `"{parent_name}.set"` from the parent Field/Subscript.
    /// Uses the parent's labels (not the setter's own Callable params)
    /// to match the witness table key built by witness_lower.rs.
    pub fn witness_setter_key(&self, setter: Entity) -> WitnessMethodKey {
        let parent = self.world.parent_of(setter);
        let parent_key = parent
            .map(|p| self.witness_method_key(p))
            .unwrap_or_else(|| WitnessMethodKey::simple("unknown"));
        WitnessMethodKey::new(format!("{}.set", parent_key.name), parent_key.labels)
    }

    /// Find a `NodeKind::Setter` child of a Field or Subscript entity.
    pub fn find_setter_child(&self, parent: Entity) -> Option<Entity> {
        self.world
            .children_of(parent)
            .iter()
            .copied()
            .find(|&e| self.world.get::<NodeKind>(e) == Some(&NodeKind::Setter))
    }

    /// Consume the context and return the built MIR module.
    pub fn finish(self) -> MirModule {
        self.module
    }
}

// === IsProtocolMethod query ===

/// Cached query: does `entity` live on a protocol (as a direct member or
/// a protocol extension default)?
///
/// Returns `Some(protocol_entity)` if so, `None` otherwise. This replaces
/// 8 scattered parent-chain walks in the old lowerer with a single memoized
/// query.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct IsProtocolMethod {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for IsProtocolMethod {
    type Output = Option<Entity>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<Entity> {
        // EnclosingContainer jumps straight to the container for setters;
        // everything else uses the direct parent.
        let parent = ctx
            .get::<EnclosingContainer>(self.entity)
            .map(|ec| ec.0)
            .or_else(|| ctx.parent_of(self.entity))?;
        match ctx.get::<NodeKind>(parent)? {
            NodeKind::Protocol => Some(parent),
            NodeKind::Extension => {
                let target = ctx.query(ExtensionTargetEntity {
                    extension: parent,
                    root: self.root,
                })?;
                match ctx.get::<NodeKind>(target)? {
                    NodeKind::Protocol => Some(target),
                    _ => None,
                }
            },
            _ => None,
        }
    }
}

// === RetRefPointerDerived query ===

/// Cached query: does this ref-returning callable fabricate its reference
/// DIRECTLY from `lang.ptr_ref` / `lang.ptr_mut_ref`?
///
/// `PointerDerived` trust originates at the intrinsic, not at any nominal
/// type. A ref-returning call's result normally re-roots at the callee's
/// borrow source (the receiver) — but a callee that is a thin intrinsic
/// wrapper (`Pointer.value` / `.mutatingValue`: every return-position
/// expression is a direct `ptr_ref`/`ptr_mut_ref` call) returns a view whose
/// validity inherits the raw pointer's contract, not the receiver temp's
/// lifetime. Without this, the wrapper's receiver temp re-roots the view as
/// `Local` and `Array.at` hits a false E494. One seam is enough: a wrapper
/// of the wrapper re-roots at its own borrowable arg, which is exactly the
/// verified discipline callers should see.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RetRefPointerDerived {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for RetRefPointerDerived {
    type Output = bool;

    fn execute(&self, ctx: &QueryContext<'_>) -> bool {
        let Some(hir) = ctx.query(kestrel_hir_lower::LowerBody {
            entity: self.entity,
            root: self.root,
        }) else {
            return false;
        };
        let Some(typed) = ctx.query(kestrel_type_infer::InferBody {
            entity: self.entity,
            root: self.root,
        }) else {
            return false;
        };

        // Return-position exprs: the body's tail plus every explicit
        // `return v` (a Return inside a nested closure would over-collect,
        // which only makes the answer conservatively false).
        let mut rets: Vec<HirExprId> = hir.tail_expr.into_iter().collect();
        for (_, expr) in hir.exprs.iter() {
            if let HirExpr::Return { value: Some(v), .. } = expr {
                rets.push(*v);
            }
        }
        !rets.is_empty()
            && rets
                .iter()
                .all(|&e| is_ptr_ref_intrinsic_call(ctx, &hir, &typed, e))
    }
}

fn is_ptr_ref_intrinsic_call(
    ctx: &QueryContext<'_>,
    hir: &kestrel_hir::body::HirBody,
    typed: &kestrel_type_infer::result::TypedBody,
    expr: HirExprId,
) -> bool {
    let HirExpr::Call { callee, .. } = &hir.exprs[expr] else {
        return false;
    };
    // Mirror of body lowering's resolve_callee_entity_from_expr.
    let entity = typed.resolutions.get(callee).copied().or_else(|| {
        match &hir.exprs[*callee] {
            HirExpr::Def(e, _, _) => Some(*e),
            _ => None,
        }
    });
    let Some(e) = entity else {
        return false;
    };
    if ctx.get::<Intrinsic>(e).is_none() {
        return false;
    }
    matches!(
        ctx.get::<Name>(e).map(|n| n.0.as_str()),
        Some("ptr_ref" | "ptr_mut_ref")
    )
}
