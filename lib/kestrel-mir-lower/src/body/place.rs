//! THE place resolver (references "Option C"): one walk that turns a HIR
//! expression into a [`Place`] — where a value LIVES — instead of each
//! consumer (ref returns, `&` bindings, call-arg prep, assignment targets,
//! pattern scrutinees) re-deriving addresses ad hoc.
//!
//! Provenance is NOT duplicated here: `emit_field_addr` inherits its base's
//! root and the `alloc_value` funnel carries it, so a place's root is always
//! `body.value(v).root`. This module owns the SHAPE question only: address
//! vs view, and which projections are places at all.
//!
//! Resolves: var locals (Addr), SSA locals / params / named ref bindings
//! (View), captured projected places (View), plain stored-field chains over
//! those (Addr via FieldAddr; @guaranteed StructExtract views under
//! [`FieldViews::Allow`]), and ref-slot fields (the loaded ref IS the place
//! view). Everything else — computed members, accessor subscripts, statics,
//! calls — returns `None`; consumers compose their existing fallbacks
//! (accessor place fabrication, writeback, value lowering).

use kestrel_hir::body::{HirExpr, HirExprId};
use kestrel_mir::inst::InstKind;
use kestrel_mir::value::Ownership;
use kestrel_mir::{FieldIdx, MirTy, TyId, ValueId};

use super::{LocalBinding, OssaBodyCtx};

/// Whether stored-field chains may resolve through @guaranteed StructExtract
/// VIEWS when no address exists (e.g. a borrowing receiver's fields). Only
/// ret_borrow return sites allow views — that makes resolution total for
/// stored-field shapes there. Call-arg/assignment consumers forbid them so
/// Copyable-field snapshot semantics and clone counts stay byte-identical.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum FieldViews {
    Forbid,
    Allow,
}

pub(crate) enum PlaceRepr {
    /// Real storage address: a `Pointer[pointee]`-typed value (var slot or
    /// FieldAddr chain). Borrow via `Begin(Mut)BorrowAddr`; stores write
    /// through it.
    Addr(ValueId),
    /// The value IS the place: an SSA local/param (@owned or @guaranteed),
    /// a named ref binding's @guaranteed value, a captured place's env
    /// value, or a @guaranteed extraction view.
    View(ValueId),
}

pub(crate) struct Place {
    pub repr: PlaceRepr,
    /// The place's value type (what a read yields; refs already peeled).
    pub pointee: TyId,
}

impl OssaBodyCtx<'_, '_> {
    /// Resolve an expression to a place, or `None` when it isn't one under
    /// this policy (rvalue, call, computed member, accessor subscript, …) —
    /// the caller composes its own fallback. Read-count-neutral: callers
    /// that meter named-binding reads keep doing so themselves.
    pub(crate) fn lower_place(&mut self, expr_id: HirExprId, views: FieldViews) -> Option<Place> {
        // Captured projected place (`self.cap` in a closure): the env value
        // loaded at entry is the view. Checked first, like
        // `lower_expr_for_borrow`.
        if let Some(v) = self.captured_place_value(expr_id) {
            let pointee = self.body.value(v).ty;
            return Some(Place {
                repr: PlaceRepr::View(v),
                pointee,
            });
        }
        let expr = self.hir.exprs[expr_id].clone();
        match expr {
            HirExpr::Local(hir_local, _) => match self.local_map.get(&hir_local).copied() {
                Some(LocalBinding::Var(addr)) => {
                    let pointee = self.resolve_local_type(hir_local);
                    Some(Place {
                        repr: PlaceRepr::Addr(addr),
                        pointee,
                    })
                },
                // SSA local / param / named ref binding: the value is the
                // place (raw — owned SSA locals must come back unborrowed;
                // borrowing is `borrow_place`'s job).
                _ => {
                    let v = self.map_local(hir_local);
                    let pointee = self.body.value(v).ty;
                    Some(Place {
                        repr: PlaceRepr::View(v),
                        pointee,
                    })
                },
            },
            HirExpr::Field { base, ref name, .. } => {
                let field_name = name.as_str_or_empty().to_string();
                self.lower_field_place(expr_id, base, &field_name, views)
            },
            _ => None,
        }
    }

    /// The `Field` arm of `lower_place`: plain stored fields only.
    fn lower_field_place(
        &mut self,
        expr_id: HirExprId,
        base: HirExprId,
        field_name: &str,
        views: FieldViews,
    ) -> Option<Place> {
        // Computed members and statics aren't storage — accessors/globals
        // (the consumers' fallbacks) own them.
        let resolved = self
            .typed
            .as_ref()
            .and_then(|t| t.resolutions.get(&expr_id))
            .copied();
        let plain_stored = resolved.is_none_or(|e| {
            self.ctx.world.get::<kestrel_ast_builder::Callable>(e).is_none()
                && self.ctx.world.get::<kestrel_ast_builder::Static>(e).is_none()
        });
        if !plain_stored {
            return None;
        }
        let base_ty = self.resolve_expr_type(base);
        let struct_entity = match self.ctx.module.ty_arena.get(base_ty) {
            MirTy::Named { entity, .. } => Some(*entity),
            _ => None,
        };
        let field_idx = struct_entity.and_then(|e| self.ctx.resolve_field_idx(e, field_name))?;

        // Ref slot (stage 2b): the place is the POINTEE storage the slot
        // points at — extract the ref (a load of the stored address); the
        // @guaranteed result IS the place view, rooted at the aggregate.
        let slot_tys = self.struct_field_tys(base_ty);
        if let Some(&slot_ty) = slot_tys.get(field_idx.index())
            && matches!(self.ctx.module.ty_arena.get(slot_ty), MirTy::Ref { .. })
        {
            let base_val = self.lower_expr_for_borrow(base);
            let v = self.emit_struct_extract(base_val, field_idx, slot_ty);
            let pointee = self.body.value(v).ty;
            return Some(Place {
                repr: PlaceRepr::View(v),
                pointee,
            });
        }

        // Address route: project the field's address off an addressable base.
        if let Some(base_place) = self.lower_place(base, views) {
            match base_place.repr {
                PlaceRepr::Addr(base_addr) => {
                    let field_addr = self.emit_field_addr(base_addr, base_ty, field_idx);
                    let pointee = match self.ctx.module.ty_arena.get(self.body.value(field_addr).ty)
                    {
                        MirTy::Pointer(inner) => *inner,
                        _ => unreachable!("FieldAddr result must be Pointer-typed"),
                    };
                    return Some(Place {
                        repr: PlaceRepr::Addr(field_addr),
                        pointee,
                    });
                },
                PlaceRepr::View(base_val) if views == FieldViews::Allow => {
                    let result_ty = self.resolve_expr_type(expr_id);
                    let v = self.extract_field_view(base_val, field_idx, result_ty);
                    return Some(Place {
                        repr: PlaceRepr::View(v),
                        pointee: result_ty,
                    });
                },
                PlaceRepr::View(_) => return None,
            }
        }
        // Base isn't a Local/Field place (call result, …): under Allow,
        // resolution stays total — borrow whatever the base lowers to and
        // project a view (ret_borrow's historical recursion tail).
        if views == FieldViews::Allow {
            let base_val = self.lower_expr_for_borrow(base);
            let result_ty = self.resolve_expr_type(expr_id);
            let v = self.extract_field_view(base_val, field_idx, result_ty);
            return Some(Place {
                repr: PlaceRepr::View(v),
                pointee: result_ty,
            });
        }
        None
    }

    /// No-copy @guaranteed field projection: borrow an owned base in place
    /// and `StructExtract` a view (never the Copyable-field snapshot —
    /// that's `read_place`'s value-context job).
    fn extract_field_view(
        &mut self,
        base_val: ValueId,
        field: FieldIdx,
        result_ty: TyId,
    ) -> ValueId {
        let base_ref = if self.body.value(base_val).ownership == Ownership::Owned {
            self.emit_begin_borrow(base_val)
        } else {
            base_val
        };
        let result = self.alloc_guaranteed(result_ty, base_ref);
        self.push_inst(InstKind::StructExtract {
            result,
            operand: base_ref,
            field,
        });
        result
    }

    // NOTE: there is deliberately no `borrow_place` helper. How a place is
    // borrowed is the CONSUMER's semantic: call args go through
    // `prepare_call_arg` (sub-borrows protect named-binding multi-use;
    // @guaranteed mutating receivers pass through), ret_borrow returns
    // borrow Addr results directly and forward Views. A one-size helper
    // papered over exactly that difference.

    /// Value-context read of a place. Addr + Copyable = the LOAD-BEARING
    /// snapshot (`let v = self.x; self.x += 1` must see the old value);
    /// Addr + non-Copyable = an in-place view (copying is illegal); views
    /// read as themselves.
    pub(crate) fn read_place(&mut self, place: &Place) -> ValueId {
        match place.repr {
            PlaceRepr::Addr(addr) => {
                if self.is_non_copyable(place.pointee) {
                    self.emit_begin_borrow_addr(addr, place.pointee)
                } else {
                    self.emit_copy_addr(addr, place.pointee)
                }
            },
            PlaceRepr::View(v) => v,
        }
    }

    /// Walk a `Local`/`Field` chain to a var-rooted ADDRESS (the historical
    /// `try_field_addr_chain`, now owned by the place resolver). Stops at
    /// ref slots: a `&T` field's storage is its POINTEE, not the slot — a
    /// FieldAddr there would alias the stored pointer's BITS as the pointee
    /// (the lost-write/corruption class); callers' fallbacks route through
    /// `emit_struct_extract`'s ref arm instead.
    pub fn try_field_addr_chain(&mut self, expr_id: HirExprId) -> Option<ValueId> {
        let expr = self.hir.exprs[expr_id].clone();
        match expr {
            HirExpr::Local(hir_local, _) => match self.local_map.get(&hir_local).copied() {
                Some(LocalBinding::Var(addr)) => Some(addr),
                _ => None,
            },
            HirExpr::Field { base, name, .. } => {
                let base_addr = self.try_field_addr_chain(base)?;
                let base_ty = self.resolve_expr_type(base);
                let field_name = name.as_str_or_empty();
                let struct_entity = match self.ctx.module.ty_arena.get(base_ty) {
                    MirTy::Named { entity, .. } => Some(*entity),
                    _ => None,
                };
                let field_idx =
                    struct_entity.and_then(|e| self.ctx.resolve_field_idx(e, field_name))?;
                let slot_tys = self.struct_field_tys(base_ty);
                if self.slot_is_ref(&slot_tys, field_idx.index()) {
                    return None;
                }
                Some(self.emit_field_addr(base_addr, base_ty, field_idx))
            },
            _ => None,
        }
    }

    /// If `expr_id` resolves to a var local (possibly through a field chain),
    /// return its address.
    pub fn try_var_addr(&mut self, expr_id: HirExprId) -> Option<ValueId> {
        self.try_field_addr_chain(expr_id)
    }
}
