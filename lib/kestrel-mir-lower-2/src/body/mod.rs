//! BodyCtx — per-function lowering context with block management and emit helpers.

pub mod call;
pub mod closure;
pub mod control;
pub mod expr;
pub mod literal;
pub mod pattern;
pub mod stmt;

use std::collections::HashMap;

use kestrel_hecs::Entity;
use kestrel_hir::body::{HirBody, HirExpr};
use kestrel_hir::res::LocalId as HirLocalId;
use kestrel_hir_lower::LowerBody;
use kestrel_mir_2::body::{BasicBlock, LocalDef, MirBody};
use kestrel_mir_2::operand::{ArgMode, Operand, UseMode};
use kestrel_mir_2::statement::{Callee, Rvalue, Statement, StatementKind};
use kestrel_mir_2::terminator::{SwitchCase, Terminator, TerminatorKind};
use kestrel_mir_2::{
    BlockId, CopyBehavior, FieldIdx, Immediate, LocalId, MirTy, Op, ParamConvention, Place, TyId,
    VariantIdx,
};
use kestrel_name_res::ExtensionTargetEntity;
use kestrel_span::Span;
use kestrel_type_infer::InferBody;
use kestrel_type_infer::result::TypedBody;

use crate::context::LowerCtx;
use crate::ty::lower_resolved_ty;

/// Tracks loop blocks for break/continue resolution.
pub(crate) struct LoopInfo {
    pub header_block: BlockId,
    pub exit_block: BlockId,
    pub label: Option<String>,
}

/// Per-function body lowering context.
pub(crate) struct BodyCtx<'a, 'w> {
    pub ctx: &'a mut LowerCtx<'w>,
    pub hir: &'a HirBody,
    pub typed: Option<&'a TypedBody>,
    pub func_entity: Entity,
    pub func_idx: usize,
    pub in_protocol_extension: bool,
    pub body: MirBody,
    pub current_block: Option<BlockId>,
    pub local_map: HashMap<HirLocalId, LocalId>,
    pub loop_stack: Vec<LoopInfo>,
    pub temp_counter: u32,
    pub current_span: Option<Span>,
}

impl<'a, 'w> BodyCtx<'a, 'w> {
    pub fn new(
        ctx: &'a mut LowerCtx<'w>,
        hir: &'a HirBody,
        typed: Option<&'a TypedBody>,
        func_entity: Entity,
        func_idx: usize,
        in_protocol_extension: bool,
    ) -> Self {
        Self {
            ctx,
            hir,
            typed,
            func_entity,
            func_idx,
            in_protocol_extension,
            body: MirBody::new(),
            current_block: None,
            local_map: HashMap::new(),
            loop_stack: Vec::new(),
            temp_counter: 0,
            current_span: None,
        }
    }

    /// Main entry: lower the function body into MIR blocks.
    pub fn lower_body(&mut self) {
        // Create locals for all HIR locals (params + user locals)
        for (hir_id, local) in self.hir.locals.iter() {
            let ty = self.resolve_local_type(hir_id);
            let mir_local = LocalDef::new(&local.name, ty);
            let mir_id = self.body.add_local(mir_local);
            self.local_map.insert(hir_id, mir_id);
        }
        self.body.param_count = self.hir.params.len();

        let entry = self.new_block();
        self.body.entry = entry;
        self.current_block = Some(entry);

        // Lower top-level statements
        for &stmt_id in &self.hir.statements {
            self.lower_stmt(stmt_id);
            if self.is_terminated() {
                break;
            }
        }

        // Lower tail expression
        if !self.is_terminated() {
            if let Some(tail) = self.hir.tail_expr {
                let tail_span = expr_span(self.hir, tail);
                let value = self.lower_expr(tail);
                if !self.is_terminated() {
                    let prev = self.current_span.replace(tail_span);
                    self.emit_ret(Operand::from(value));
                    self.current_span = prev;
                }
            } else {
                self.emit_ret(Operand::Const(Immediate::unit()));
            }
        }
    }

    /// Consume the context and return the built MIR body.
    pub fn finish(self) -> MirBody {
        self.body
    }

    // === Block management ===

    pub fn new_block(&mut self) -> BlockId {
        self.body.add_block(BasicBlock::new())
    }

    pub fn switch_to(&mut self, block: BlockId) {
        self.current_block = Some(block);
    }

    pub fn is_terminated(&self) -> bool {
        let Some(block_id) = self.current_block else {
            return true;
        };
        !matches!(
            self.body.block(block_id).terminator.kind,
            TerminatorKind::Unreachable
        )
    }

    // === Locals ===

    pub fn fresh_temp(&mut self, ty: TyId) -> LocalId {
        let name = format!("_t{}", self.temp_counter);
        self.temp_counter += 1;
        self.body.add_local(LocalDef::new(name, ty))
    }

    pub fn map_local(&self, hir_id: HirLocalId) -> LocalId {
        self.local_map
            .get(&hir_id)
            .copied()
            .unwrap_or_else(|| LocalId::new(0))
    }

    pub fn resolve_local_type(&mut self, hir_id: HirLocalId) -> TyId {
        if let Some(typed) = self.typed
            && let Some(resolved) = typed.local_types.get(&hir_id)
        {
            return lower_resolved_ty(self.ctx, resolved);
        }
        self.ctx.module.ty_arena.error()
    }

    pub fn resolve_expr_type(&mut self, expr_id: kestrel_hir::body::HirExprId) -> TyId {
        if let Some(typed) = self.typed
            && let Some(resolved) = typed.expr_types.get(&expr_id)
        {
            return lower_resolved_ty(self.ctx, resolved);
        }
        self.ctx.module.ty_arena.error()
    }

    // === Emit statements (auto-stamps current span) ===

    fn push_stmt(&mut self, kind: StatementKind) {
        let span = self.current_span.clone();
        let stmt = match span {
            Some(s) => Statement::with_span(kind, s),
            None => Statement::new(kind),
        };
        if let Some(block_id) = self.current_block {
            self.body.block_mut(block_id).stmts.push(stmt);
        }
    }

    pub fn emit_assign(&mut self, dest: Place, rvalue: Rvalue) {
        self.push_stmt(StatementKind::Assign { dest, rvalue });
    }

    pub fn emit_use_copy(&mut self, dest: Place, src: Place) {
        self.emit_assign(dest, Rvalue::Use(Operand::Place(src), UseMode::Copy));
    }

    pub fn emit_use_move(&mut self, dest: Place, src: Place) {
        self.emit_assign(dest, Rvalue::Use(Operand::Place(src), UseMode::Move));
    }

    pub fn emit_assign_const(&mut self, dest: Place, imm: Immediate) {
        self.emit_assign(
            dest,
            Rvalue::Use(Operand::Const(imm), UseMode::Copy),
        );
    }

    pub fn emit_assign_op1(&mut self, dest: Place, op: Op, arg: Operand) {
        self.emit_assign(dest, Rvalue::Op1 { op, arg });
    }

    pub fn emit_assign_op2(&mut self, dest: Place, op: Op, lhs: Operand, rhs: Operand) {
        self.emit_assign(dest, Rvalue::Op2 { op, lhs, rhs });
    }

    pub fn emit_assign_op3(&mut self, dest: Place, op: Op, a: Operand, b: Operand, c: Operand) {
        self.emit_assign(dest, Rvalue::Op3 { op, a, b, c });
    }

    pub fn emit_construct(
        &mut self,
        dest: Place,
        ty: TyId,
        fields: Vec<(FieldIdx, Operand, UseMode)>,
    ) {
        self.emit_assign(dest, Rvalue::Construct { ty, fields });
    }

    pub fn emit_tuple(&mut self, dest: Place, elems: Vec<(Operand, UseMode)>) {
        self.emit_assign(dest, Rvalue::Tuple(elems));
    }

    pub fn emit_enum_variant(
        &mut self,
        dest: Place,
        enum_ty: TyId,
        variant: VariantIdx,
        payload: Vec<(Operand, UseMode)>,
    ) {
        self.emit_assign(
            dest,
            Rvalue::EnumVariant {
                enum_ty,
                variant,
                payload,
            },
        );
    }

    pub fn emit_call(
        &mut self,
        dest: Option<Place>,
        callee: Callee,
        mut args: Vec<(Operand, ArgMode)>,
    ) {
        for (operand, mode) in &mut args {
            if matches!(mode, ArgMode::Ref | ArgMode::RefMut)
                && let Operand::Const(imm) = operand
            {
                // Ref/RefMut args must be Places — materialize Consts into temps
                let ty = imm.ty(&mut self.ctx.module.ty_arena);
                let temp = self.fresh_temp(ty);
                let imm_clone = imm.clone();
                self.emit_assign_const(Place::local(temp), imm_clone);
                *operand = Operand::Place(Place::local(temp));
            } else if *mode == ArgMode::RefMut
                && let Operand::Place(place) = operand
            {
                // RefMut args are initialized by the callee (e.g. init calls) —
                // mark them Live for init-state analysis
                self.push_stmt(StatementKind::Uninit { dest: place.clone() });
            }
        }
        self.push_stmt(StatementKind::Call { dest, callee, args });
    }

    pub fn emit_drop(&mut self, place: Place) {
        self.push_stmt(StatementKind::Drop { place });
    }

    pub fn emit_drop_if(&mut self, place: Place, flag: LocalId) {
        self.push_stmt(StatementKind::DropIf { place, flag });
    }

    pub fn emit_set_drop_flag(&mut self, flag: LocalId, value: bool) {
        self.push_stmt(StatementKind::SetDropFlag { flag, value });
    }

    pub fn emit_scope_live(&mut self, local: LocalId) {
        self.push_stmt(StatementKind::ScopeLive(local));
    }

    // === Emit terminators (auto-stamps current span) ===

    fn set_terminator(&mut self, kind: TerminatorKind) {
        let span = self.current_span.clone();
        let term = match span {
            Some(s) => Terminator::with_span(kind, s),
            None => Terminator::new(kind),
        };
        if let Some(block_id) = self.current_block {
            self.body.block_mut(block_id).terminator = term;
        }
    }

    pub fn emit_ret(&mut self, operand: Operand) {
        self.set_terminator(TerminatorKind::Return(operand));
    }

    pub fn emit_ret_unit(&mut self) {
        self.emit_ret(Operand::Const(Immediate::unit()));
    }

    pub fn emit_jump(&mut self, target: BlockId) {
        self.set_terminator(TerminatorKind::Jump(target));
    }

    pub fn emit_branch(&mut self, cond: Operand, then_block: BlockId, else_block: BlockId) {
        self.set_terminator(TerminatorKind::Branch {
            condition: cond,
            then_block,
            else_block,
        });
    }

    pub fn emit_switch(&mut self, disc: Place, cases: Vec<(SwitchCase, BlockId)>) {
        self.set_terminator(TerminatorKind::Switch {
            discriminant: disc,
            cases,
        });
    }

    pub fn emit_panic(&mut self, msg: &str) {
        self.set_terminator(TerminatorKind::Panic(msg.to_string()));
    }

    pub fn emit_unreachable(&mut self) {
        self.set_terminator(TerminatorKind::Unreachable);
    }

    // === Mode decisions (single source of truth) ===

    /// Determine UseMode (Copy or Move) for a type.
    pub fn use_mode_for(&self, ty: TyId) -> UseMode {
        if self.is_copy_type(ty) {
            UseMode::Copy
        } else {
            UseMode::Move
        }
    }

    /// Determine ArgMode for passing a value with a given calling convention.
    pub fn arg_mode_for(&self, ty: TyId, convention: ParamConvention) -> ArgMode {
        match convention {
            ParamConvention::Borrow => ArgMode::Ref,
            ParamConvention::MutBorrow => ArgMode::RefMut,
            ParamConvention::Consuming => {
                if self.is_copy_type(ty) {
                    ArgMode::Copy
                } else {
                    ArgMode::Move
                }
            }
        }
    }

    /// Check if a type is bitwise-copyable.
    fn is_copy_type(&self, ty: TyId) -> bool {
        match self.ctx.module.ty_arena.get(ty) {
            MirTy::I8
            | MirTy::I16
            | MirTy::I32
            | MirTy::I64
            | MirTy::F16
            | MirTy::F32
            | MirTy::F64
            | MirTy::Bool
            | MirTy::Never
            | MirTy::Error => true,
            MirTy::Pointer(_) => true,
            MirTy::Tuple(elems) => elems.iter().all(|e| self.is_copy_type(*e)),
            MirTy::Named { entity, .. } => {
                // Check if the struct/enum is Bitwise-copyable
                if let Some(s) = self.ctx.module.structs.iter().find(|s| s.entity == *entity) {
                    matches!(s.type_info.copy, CopyBehavior::Bitwise)
                } else if let Some(e) = self.ctx.module.enums.iter().find(|e| e.entity == *entity) {
                    matches!(e.type_info.copy, CopyBehavior::Bitwise)
                } else {
                    false
                }
            }
            MirTy::TypeParam(_) | MirTy::SelfType | MirTy::AssociatedProjection { .. } => {
                // Generic types default to non-copyable (conservative)
                false
            }
            MirTy::FuncThin { .. } => true,
            MirTy::FuncThick { .. } => false,
            MirTy::Str => false,
        }
    }

    /// Materialize an operand into a place. If already a place, return it.
    /// If a constant, assign to a temp and return the temp place.
    pub fn operand_to_place(&mut self, operand: Operand, ty: TyId) -> Place {
        match operand {
            Operand::Place(p) => p,
            Operand::Const(imm) => {
                let temp = self.fresh_temp(ty);
                self.emit_assign_const(Place::local(temp), imm);
                Place::local(temp)
            }
        }
    }

    /// Build an Rvalue from an operand for assignment, picking Copy or Move
    /// based on the type's copy behavior.
    pub fn rvalue_for_assign(&self, operand: Operand, ty: TyId) -> Rvalue {
        let mode = self.use_mode_for(ty);
        Rvalue::Use(operand, mode)
    }

    /// Emit an assignment from operand to dest with mode auto-selected.
    pub fn emit_value_transfer(&mut self, dest: Place, operand: Operand, ty: TyId) {
        let rvalue = self.rvalue_for_assign(operand, ty);
        self.emit_assign(dest, rvalue);
    }
}

/// Lower a function entity's body into MIR basic blocks.
pub fn lower_function_body(ctx: &mut LowerCtx, entity: Entity, func_idx: usize) {
    let Some(hir) = ctx.query.query(LowerBody {
        entity,
        root: ctx.root,
    }) else {
        return;
    };

    let typed = ctx.query.query(InferBody {
        entity,
        root: ctx.root,
    });

    let in_protocol_extension = ctx.world.parent_of(entity).is_some_and(|parent| {
        matches!(
            ctx.world.get::<kestrel_ast_builder::NodeKind>(parent),
            Some(kestrel_ast_builder::NodeKind::Extension)
        ) && ctx
            .query
            .query(ExtensionTargetEntity {
                extension: parent,
                root: ctx.root,
            })
            .is_some_and(|target| {
                matches!(
                    ctx.world.get::<kestrel_ast_builder::NodeKind>(target),
                    Some(kestrel_ast_builder::NodeKind::Protocol)
                )
            })
    });

    let mut bctx = BodyCtx::new(ctx, &hir, typed.as_ref(), entity, func_idx, in_protocol_extension);
    bctx.lower_body();
    let mir_body = bctx.finish();

    // Patch ParamDefs with real local IDs and inference-resolved types
    let func = &mut ctx.module.functions[func_idx];
    for (pi, param) in func.params.iter_mut().enumerate() {
        let local_id = kestrel_mir_2::LocalId::new(pi);
        param.local = local_id;
        if pi < mir_body.locals.len() {
            param.ty = mir_body.locals[pi].ty;
        }
    }
    func.body = Some(mir_body);
}

/// Extract the span from any HirExpr variant.
pub(crate) fn expr_span(hir: &HirBody, id: kestrel_hir::body::HirExprId) -> Span {
    match &hir.exprs[id] {
        HirExpr::Literal { span, .. }
        | HirExpr::Local(_, span)
        | HirExpr::Def(_, _, span)
        | HirExpr::OverloadSet { span, .. }
        | HirExpr::Field { span, .. }
        | HirExpr::TupleIndex { span, .. }
        | HirExpr::ImplicitMember { span, .. }
        | HirExpr::Call { span, .. }
        | HirExpr::MethodCall { span, .. }
        | HirExpr::ProtocolCall { span, .. }
        | HirExpr::If { span, .. }
        | HirExpr::Loop { span, .. }
        | HirExpr::Match { span, .. }
        | HirExpr::Break { span, .. }
        | HirExpr::Continue { span, .. }
        | HirExpr::Return { span, .. }
        | HirExpr::Assign { span, .. }
        | HirExpr::Tuple { span, .. }
        | HirExpr::Array { span, .. }
        | HirExpr::Dict { span, .. }
        | HirExpr::Closure { span, .. }
        | HirExpr::Block { span, .. }
        | HirExpr::Error { span }
        | HirExpr::Sugar { span, .. } => span.clone(),
    }
}
