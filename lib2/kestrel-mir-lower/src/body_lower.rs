//! Function body lowering — HirExpr/HirStmt → MIR basic blocks.
//!
//! Handles: literals, locals, assignments, return, if/else, loops,
//! break/continue, blocks, field access, tuple index, calls (direct,
//! method, protocol/witness).

use std::collections::HashMap;

use kestrel_hecs::Entity;
use kestrel_hir::body::{
    HirBlock, HirBody, HirCallArg, HirClosureParam, HirExpr, HirExprId, HirLiteral, HirMatchArm,
    HirStmt, HirStmtId,
};
use kestrel_hir::res::LocalId as HirLocalId;
use kestrel_hir_lower::LowerBody;
use kestrel_pattern_matching::constructor::Constructor;
use kestrel_pattern_matching::decision_tree::{Binding, DecisionTree, PathElement};
use kestrel_type_infer::InferBody;
use kestrel_type_infer::result::TypedBody;

use kestrel_mir::*;

use crate::context::LowerCtx;
use crate::resolved_ty::lower_resolved_ty;
use crate::ty::lower_type;

/// Lower a function entity's body into MIR basic blocks.
///
/// Called after the function signature has been created. Fills in the
/// `body` field of the FunctionDef with locals, blocks, and statements.
pub fn lower_function_body(ctx: &mut LowerCtx, entity: Entity, func_id: FunctionId) {
    // Get the HIR body
    let Some(hir) = ctx.query.query(LowerBody {
        entity,
        root: ctx.root,
    }) else {
        return;
    };

    // Get type inference results (may fail if inference errored)
    let typed = ctx.query.query(InferBody {
        entity,
        root: ctx.root,
    });

    let mut body_ctx = BodyLowerCtx {
        hir: &hir,
        typed: typed.as_ref(),
        func_id,
        local_map: HashMap::new(),
        body: MirBody::new(),
        current_block: None,
        loop_stack: Vec::new(),
        temp_counter: 0,
        ctx,
    };

    body_ctx.lower_body();

    // Attach the built body to the function
    let mir_body = body_ctx.body;
    body_ctx.ctx.module.functions[func_id.index()].body = Some(mir_body);
}

/// Tracks loop blocks for break/continue resolution.
struct LoopInfo {
    header_block: BlockId,
    exit_block: BlockId,
    label: Option<String>,
}

/// Per-function lowering context.
struct BodyLowerCtx<'a, 'b> {
    ctx: &'a mut LowerCtx<'b>,
    hir: &'a HirBody,
    typed: Option<&'a TypedBody>,
    func_id: FunctionId,

    // Maps HIR local IDs to MIR local IDs
    local_map: HashMap<HirLocalId, LocalId>,

    // The MIR body being built
    body: MirBody,

    // Current block we're emitting statements into
    current_block: Option<BlockId>,

    // Loop stack for break/continue
    loop_stack: Vec<LoopInfo>,

    // Counter for generating unique temp names
    temp_counter: u32,
}

impl<'a, 'b> BodyLowerCtx<'a, 'b> {
    /// Main entry point: lower the full function body.
    fn lower_body(&mut self) {
        // Create locals for all HIR locals (params + user locals)
        for (hir_id, local) in self.hir.locals.iter() {
            let ty = self.resolve_local_type(hir_id);
            let mir_local = LocalDef::new(&local.name, ty);
            let mir_id = self.body.add_local(mir_local);
            self.local_map.insert(hir_id, mir_id);
        }

        // Set param count from HIR
        self.body.param_count = self.hir.params.len();

        // Create entry block
        let entry = self.new_block();
        self.body.entry = entry;
        self.current_block = Some(entry);

        // Lower top-level statements
        for &stmt_id in &self.hir.statements {
            self.lower_stmt(stmt_id);
            // If the block was terminated (by return/break/etc), stop
            if self.is_terminated() {
                break;
            }
        }

        // Lower tail expression (the block's return value)
        if !self.is_terminated() {
            if let Some(tail) = self.hir.tail_expr {
                let value = self.lower_expr(tail);
                // The tail expression may have set a terminator itself
                // (e.g., lang.panic_unwind emits Panic). Don't overwrite it.
                if !self.is_terminated() {
                    self.set_terminator(Terminator::ret(value));
                }
            } else {
                // No tail → return unit
                self.set_terminator(Terminator::ret(Immediate::unit()));
            }
        }
    }

    // === Block management ===

    /// Create a new empty block and return its ID.
    fn new_block(&mut self) -> BlockId {
        self.body.add_block(BasicBlock::new())
    }

    /// Switch to emitting into a different block.
    fn switch_to_block(&mut self, block: BlockId) {
        self.current_block = Some(block);
    }

    /// Check if the current block has been terminated.
    fn is_terminated(&self) -> bool {
        let Some(block_id) = self.current_block else {
            return true;
        };
        // A block is "terminated" if it's not the default Unreachable
        // (which is the placeholder set by BasicBlock::new())
        let block = self.body.block(block_id);
        !matches!(block.terminator.kind, TerminatorKind::Unreachable)
    }

    /// Add a statement to the current block.
    fn emit_stmt(&mut self, stmt: Statement) {
        if let Some(block_id) = self.current_block {
            self.body.block_mut(block_id).stmts.push(stmt);
        }
    }

    /// Set the terminator of the current block.
    fn set_terminator(&mut self, term: Terminator) {
        if let Some(block_id) = self.current_block {
            self.body.block_mut(block_id).terminator = term;
        }
    }

    /// Create a temporary local with a generated name.
    fn fresh_temp(&mut self, ty: MirTy) -> LocalId {
        let name = format!("_t{}", self.temp_counter);
        self.temp_counter += 1;
        let local = LocalDef::new(name, ty);
        self.body.add_local(local)
    }

    /// Get the MIR type for a HIR local from type inference results.
    fn resolve_local_type(&mut self, hir_id: HirLocalId) -> MirTy {
        if let Some(typed) = self.typed {
            if let Some(resolved) = typed.local_types.get(&hir_id) {
                return lower_resolved_ty(self.ctx, resolved);
            }
        }
        MirTy::Error
    }

    /// Get the MIR type for a HIR expression from type inference results.
    fn resolve_expr_type(&mut self, expr_id: HirExprId) -> MirTy {
        if let Some(typed) = self.typed {
            if let Some(resolved) = typed.expr_types.get(&expr_id) {
                return lower_resolved_ty(self.ctx, resolved);
            }
        }
        MirTy::Error
    }

    /// If the assignment target is a computed property (Field or Def entity
    /// with a `NodeKind::Setter` child), emit a setter call and return the
    /// unit value. Otherwise returns None so the caller falls through to the
    /// default stored-Place assignment path.
    ///
    /// Handles the three concrete shapes exercised by
    /// `validation/properties_intended/*_get_set.ks`:
    ///   - `globalComputedVar = v`      (HirExpr::Def, no receiver)
    ///   - `Foo.staticComputed = v`     (HirExpr::Field, type-ref base, no receiver)
    ///   - `obj.computed = v`           (HirExpr::Field, value base, mut-borrow receiver)
    ///
    /// Protocol property requirements (a Field whose parent is a `Protocol`
    /// with `Settable` but no `Callable`) dispatch through the conformance
    /// witness using the `<name>.set` convention, mirroring the getter side.
    fn try_lower_setter_assign(&mut self, target_id: HirExprId, value_id: HirExprId) -> Option<Value> {
        let target = self.hir.exprs[target_id].clone();
        match target {
            HirExpr::Field { base, name, .. } => {
                let resolved = self
                    .typed
                    .and_then(|t| t.resolutions.get(&target_id))
                    .copied()?;

                // Protocol property requirement: no setter child to call directly;
                // emit a witness call with `<field>.set`.
                let is_field = self
                    .ctx
                    .world
                    .get::<kestrel_ast_builder::NodeKind>(resolved)
                    == Some(&kestrel_ast_builder::NodeKind::Field);
                let parent_is_protocol = is_field
                    && self.ctx.world.parent_of(resolved).is_some_and(|p| {
                        self.ctx.world.get::<kestrel_ast_builder::NodeKind>(p)
                            == Some(&kestrel_ast_builder::NodeKind::Protocol)
                    });
                let has_callable = self
                    .ctx
                    .world
                    .get::<kestrel_ast_builder::Callable>(resolved)
                    .is_some();
                let has_settable = self
                    .ctx
                    .world
                    .get::<kestrel_ast_builder::Settable>(resolved)
                    .is_some();
                if parent_is_protocol && !has_callable && has_settable {
                    let protocol = self.ctx.world.parent_of(resolved).unwrap();
                    self.ctx.register_name(protocol);
                    let is_static = self
                        .ctx
                        .world
                        .get::<kestrel_ast_builder::Static>(resolved)
                        .is_some();
                    let rhs_val = self.lower_expr(value_id);
                    let newval_arg = CallArg::copy(rhs_val);
                    let method_type_args = self.resolve_type_args(target_id);
                    if is_static {
                        let self_type = self.type_from_type_ref(base);
                        let callee = Callee::witness(
                            protocol,
                            format!("{name}.set"),
                            self_type,
                            method_type_args,
                        );
                        self.emit_call(callee, vec![newval_arg], MirTy::Unit);
                    } else {
                        let receiver_ty = self.resolve_expr_type(base);
                        let base_val = self.lower_expr(base);
                        let receiver_arg = CallArg::mutating(base_val);
                        let callee = Callee::witness(
                            protocol,
                            format!("{name}.set"),
                            receiver_ty,
                            method_type_args,
                        );
                        self.emit_call(callee, vec![receiver_arg, newval_arg], MirTy::Unit);
                    }
                    return Some(Value::Immediate(Immediate::unit()));
                }

                let setter = find_setter_child(self.ctx, resolved)?;
                self.ctx.register_name(setter);
                let is_static = self
                    .ctx
                    .world
                    .get::<kestrel_ast_builder::Static>(resolved)
                    .is_some();
                let rhs_val = self.lower_expr(value_id);
                let newval_arg = CallArg::copy(rhs_val);
                if is_static {
                    let self_type = self.type_from_type_ref(base);
                    let type_args = self.prepend_receiver_type_args(&self_type, vec![]);
                    let callee = Callee::method(setter, type_args, self_type);
                    self.emit_call(callee, vec![newval_arg], MirTy::Unit);
                } else {
                    let receiver_ty = self.resolve_expr_type(base);
                    let base_val = self.lower_expr(base);
                    let receiver_arg = CallArg::mutating(base_val);
                    let type_args = self.resolve_type_args(target_id);
                    let type_args = self.prepend_receiver_type_args(&receiver_ty, type_args);
                    let callee = Callee::method(setter, type_args, receiver_ty);
                    self.emit_call(callee, vec![receiver_arg, newval_arg], MirTy::Unit);
                }
                Some(Value::Immediate(Immediate::unit()))
            },
            HirExpr::Def(entity, _, _) => {
                let setter = find_setter_child(self.ctx, entity)?;
                self.ctx.register_name(setter);
                let rhs_val = self.lower_expr(value_id);
                let newval_arg = CallArg::copy(rhs_val);
                let callee = Callee::direct_generic(setter, Vec::new());
                self.emit_call(callee, vec![newval_arg], MirTy::Unit);
                Some(Value::Immediate(Immediate::unit()))
            },
            _ => None,
        }
    }

    /// Derive a MirTy from an expression that's used as a type reference (e.g.,
    /// the base of a static member access like `T.foo` or `MyStruct.bar`).
    /// For Def(TypeParameter) returns MirTy::TypeParam; for Def(Struct/Enum/TypeAlias)
    /// returns MirTy::Named with type args lowered from the HIR. Falls back to
    /// the inferred expression type for anything else.
    fn type_from_type_ref(&mut self, expr_id: HirExprId) -> MirTy {
        if let HirExpr::Def(entity, hir_type_args, _) = self.hir.exprs[expr_id].clone() {
            let kind = self.ctx.world.get::<kestrel_ast_builder::NodeKind>(entity).cloned();
            match kind {
                Some(kestrel_ast_builder::NodeKind::TypeParameter) => {
                    return MirTy::TypeParam(entity);
                },
                Some(kestrel_ast_builder::NodeKind::Struct)
                | Some(kestrel_ast_builder::NodeKind::Enum)
                | Some(kestrel_ast_builder::NodeKind::TypeAlias)
                | Some(kestrel_ast_builder::NodeKind::Protocol) => {
                    let type_args = hir_type_args.iter()
                        .map(|t| lower_type(self.ctx, t))
                        .collect();
                    return MirTy::Named { entity, type_args };
                },
                _ => {},
            }
        }
        self.resolve_expr_type(expr_id)
    }

    /// Map a HIR local ID to its MIR local ID.
    fn map_local(&self, hir_id: HirLocalId) -> LocalId {
        self.local_map
            .get(&hir_id)
            .copied()
            .unwrap_or(LocalId::new(0))
    }

    // === Statement lowering ===

    fn lower_stmt(&mut self, stmt_id: HirStmtId) {
        let stmt = &self.hir.stmts[stmt_id];
        match stmt {
            HirStmt::Let { local, value, .. } => {
                let mir_local = self.map_local(*local);
                if let Some(init_expr) = value {
                    let init_value = self.lower_expr(*init_expr);
                    self.emit_stmt(Statement::new(StatementKind::Assign {
                        dest: Place::local(mir_local),
                        rvalue: value_to_rvalue(init_value),
                    }));
                }
            },
            HirStmt::Expr { expr, .. } => {
                // Lower for side effects, discard result
                let _ = self.lower_expr(*expr);
            },
            HirStmt::Deinit { .. } => {
                // Skip — deinit pass handles this later
            },
        }
    }

    // === Expression lowering ===

    /// Lower an expression, returning its value (Place or Immediate).
    fn lower_expr(&mut self, expr_id: HirExprId) -> Value {
        let expr = self.hir.exprs[expr_id].clone();
        match &expr {
            HirExpr::Literal { value, .. } => self.lower_literal_expr(expr_id, value),
            HirExpr::Local(hir_local, _) => {
                Value::Place(Place::local(self.map_local(*hir_local)))
            },
            HirExpr::Tuple { elements, .. } => {
                let values: Vec<Value> = elements.iter().map(|&e| self.lower_expr(e)).collect();
                let ty = self.resolve_expr_type(expr_id);
                let dest = self.fresh_temp(ty);
                self.emit_stmt(Statement::new(StatementKind::Assign {
                    dest: Place::local(dest),
                    rvalue: Rvalue::Tuple(values),
                }));
                Value::Place(Place::local(dest))
            },
            HirExpr::Field { base, name, .. } => {
                // Check if this is a computed property (resolved entity has Callable)
                let resolved = self.typed.and_then(|t| t.resolutions.get(&expr_id)).copied();
                let is_callable = resolved.map_or(false, |e| {
                    self.ctx.world.get::<kestrel_ast_builder::Callable>(e).is_some()
                });
                // Abstract protocol property: a Field whose parent is a Protocol
                // (no body, no Callable). Dispatch via witness so monomorphization
                // resolves to the conforming type's computed property.
                let is_protocol_property = !is_callable
                    && resolved.map_or(false, |e| {
                        let is_field = matches!(
                            self.ctx.world.get::<kestrel_ast_builder::NodeKind>(e),
                            Some(kestrel_ast_builder::NodeKind::Field)
                        );
                        if !is_field {
                            return false;
                        }
                        self.ctx.world.parent_of(e).map_or(false, |p| {
                            matches!(
                                self.ctx.world.get::<kestrel_ast_builder::NodeKind>(p),
                                Some(kestrel_ast_builder::NodeKind::Protocol)
                            )
                        })
                    });
                // Static property: no receiver. The base is a type-ref (Def of
                // TypeParameter or concrete type), not a value expression.
                let is_static = resolved.map_or(false, |e| {
                    self.ctx.world.get::<kestrel_ast_builder::Static>(e).is_some()
                });

                if is_protocol_property && is_static {
                    // Static protocol property: dispatch via witness with no receiver.
                    // Self type comes from the base's type-ref.
                    let property_entity = resolved.unwrap();
                    let protocol = self.ctx.world.parent_of(property_entity).unwrap();
                    self.ctx.register_name(protocol);
                    let self_type = self.type_from_type_ref(*base);
                    let result_ty = self.resolve_expr_type(expr_id);
                    let method_type_args = self.resolve_type_args(expr_id);
                    let callee = Callee::witness(
                        protocol,
                        name.clone(),
                        self_type,
                        method_type_args,
                    );
                    self.emit_call(callee, vec![], result_ty)
                } else if is_protocol_property {
                    let property_entity = resolved.unwrap();
                    let protocol = self.ctx.world.parent_of(property_entity).unwrap();
                    self.ctx.register_name(protocol);
                    let receiver_ty = self.resolve_expr_type(*base);
                    let base_val = self.lower_expr(*base);
                    let result_ty = self.resolve_expr_type(expr_id);
                    let receiver_arg = CallArg::borrow(base_val);
                    let method_type_args = self.resolve_type_args(expr_id);
                    let callee = Callee::witness(
                        protocol,
                        name.clone(),
                        receiver_ty,
                        method_type_args,
                    );
                    self.emit_call(callee, vec![receiver_arg], result_ty)
                } else if is_callable && is_static {
                    // Static computed property on concrete type: direct getter
                    // call, no receiver. Base is a type-ref, not a value.
                    let getter_entity = resolved.unwrap();
                    self.ctx.register_name(getter_entity);
                    let self_type = self.type_from_type_ref(*base);
                    let result_ty = self.resolve_expr_type(expr_id);
                    let type_args = self.prepend_receiver_type_args(&self_type, vec![]);
                    let callee = Callee::method(getter_entity, type_args, self_type);
                    self.emit_call(callee, vec![], result_ty)
                } else if is_static {
                    // Static stored field on a concrete type (e.g. `Foo.staticVar`).
                    // No receiver, no call — just a global place.
                    let static_entity = resolved.unwrap();
                    self.ctx.register_name(static_entity);
                    Value::Place(Place::Global(static_entity))
                } else if is_callable {
                    // Computed property: emit a getter call
                    let getter_entity = resolved.unwrap();
                    self.ctx.register_name(getter_entity);
                    let receiver_ty = self.resolve_expr_type(*base);
                    let base_val = self.lower_expr(*base);
                    let result_ty = self.resolve_expr_type(expr_id);

                    // Pass receiver as Ref (borrowing getter)
                    let receiver_arg = CallArg::borrow(base_val);
                    let type_args = self.resolve_type_args(expr_id);
                    let type_args = self.prepend_receiver_type_args(&receiver_ty, type_args);

                    // Use method callee so self_type is set — monomorphization
                    // needs self_type to mangle the name correctly
                    let callee = Callee::method(getter_entity, type_args, receiver_ty);
                    self.emit_call(callee, vec![receiver_arg], result_ty)
                } else {
                    // Stored field: direct place access
                    let base_val = self.lower_expr(*base);
                    match base_val {
                        Value::Place(p) => Value::Place(p.field(name.clone())),
                        _ => {
                            // Need to materialize the value into a temp first
                            let ty = self.resolve_expr_type(*base);
                            let temp = self.fresh_temp(ty);
                            self.emit_stmt(Statement::new(StatementKind::Assign {
                                dest: Place::local(temp),
                                rvalue: value_to_rvalue(base_val),
                            }));
                            Value::Place(Place::local(temp).field(name.clone()))
                        },
                    }
                }
            },
            HirExpr::TupleIndex { base, index, .. } => {
                let base_val = self.lower_expr(*base);
                match base_val {
                    Value::Place(p) => Value::Place(p.index(*index as usize)),
                    _ => {
                        let ty = self.resolve_expr_type(*base);
                        let temp = self.fresh_temp(ty);
                        self.emit_stmt(Statement::new(StatementKind::Assign {
                            dest: Place::local(temp),
                            rvalue: value_to_rvalue(base_val),
                        }));
                        Value::Place(Place::local(temp).index(*index as usize))
                    },
                }
            },
            HirExpr::If {
                condition,
                then_body,
                else_body,
                ..
            } => self.lower_if(expr_id, *condition, then_body, else_body.as_ref()),
            HirExpr::Loop { body, label, .. } => self.lower_loop(body, label.as_deref()),
            HirExpr::Break { label, .. } => self.lower_break(label.as_deref()),
            HirExpr::Continue { label, .. } => self.lower_continue(label.as_deref()),
            HirExpr::Return { value, .. } => {
                let ret_val = value
                    .map(|v| self.lower_expr(v))
                    .unwrap_or(Value::Immediate(Immediate::unit()));
                self.set_terminator(Terminator::ret(ret_val));
                // After return, the block is terminated — return a dummy value
                Value::Immediate(Immediate::unit())
            },
            HirExpr::Assign { target, value, .. } => {
                // Computed-property assignments (`obj.computed = v`,
                // `Foo.staticComputed = v`, `globalComputedVar = v`) dispatch
                // through the Field's Setter child entity rather than emitting
                // a stored-Place write.
                if let Some(val) = self.try_lower_setter_assign(*target, *value) {
                    return val;
                }
                let rhs = self.lower_expr(*value);
                let lhs = self.lower_expr(*target);
                if let Value::Place(dest) = lhs {
                    self.emit_stmt(Statement::new(StatementKind::Assign {
                        dest,
                        rvalue: value_to_rvalue(rhs),
                    }));
                }
                Value::Immediate(Immediate::unit())
            },
            HirExpr::Block { body, .. } => self.lower_hir_block(body),
            HirExpr::Error { .. } => Value::Immediate(Immediate::error()),

            // === References ===
            HirExpr::Def(entity, _type_args, _) => {
                // Function/type/enum-case reference
                self.ctx.register_name(*entity);
                let node_kind = self.ctx.world.get::<kestrel_ast_builder::NodeKind>(*entity);
                match node_kind {
                    Some(kestrel_ast_builder::NodeKind::Function)
                    | Some(kestrel_ast_builder::NodeKind::Initializer) => {
                        // If inference resolved this position to a thick callable
                        // (e.g., `let f = some_fn` where f: (T)->U, or passed to a
                        // closure-typed parameter), coerce the bare function reference
                        // into a thick closure with no captures via ApplyPartial.
                        // Otherwise downstream code memcpys 16 bytes from the function's
                        // code address into a 16-byte FuncThick slot.
                        let inferred_ty = self.resolve_expr_type(expr_id);
                        if matches!(inferred_ty, MirTy::FuncThick { .. }) {
                            let dest = self.fresh_temp(inferred_ty.clone());
                            self.emit_stmt(Statement::new(StatementKind::Assign {
                                dest: Place::local(dest),
                                rvalue: Rvalue::ApplyPartial {
                                    func: *entity,
                                    captures: vec![],
                                },
                            }));
                            return Value::Place(Place::local(dest));
                        }
                        // Function reference — return as immediate
                        let type_args = self.resolve_type_args(expr_id);
                        Value::Immediate(Immediate::function_ref_generic(*entity, type_args))
                    },
                    Some(kestrel_ast_builder::NodeKind::EnumCase) => {
                        // Simple enum case (no payload) — construct as enum variant
                        let ty = self.resolve_expr_type(expr_id);
                        let case_name = self.ctx.world
                            .get::<kestrel_ast_builder::Name>(*entity)
                            .map(|n| n.0.clone())
                            .unwrap_or_default();
                        let dest = self.fresh_temp(ty.clone());
                        self.emit_stmt(Statement::new(StatementKind::Assign {
                            dest: Place::local(dest),
                            rvalue: Rvalue::EnumVariant {
                                enum_ty: ty,
                                variant: case_name,
                                payload: vec![],
                            },
                        }));
                        Value::Place(Place::local(dest))
                    },
                    Some(kestrel_ast_builder::NodeKind::Struct) => {
                        // Struct used as value — likely an init reference.
                        // Try to find the default init and use that.
                        if let Some(init) = self.resolve_init_function(*entity, &[]) {
                            Value::Immediate(Immediate::function_ref(init))
                        } else {
                            Value::Immediate(Immediate::function_ref(*entity))
                        }
                    },
                    Some(kestrel_ast_builder::NodeKind::Field) => {
                        // Computed property (has Callable) → call the getter.
                        // Otherwise it's a stored field referenced by bare name —
                        // that only happens for globals (module-level or `static`
                        // inside a type); instance fields always go through
                        // `HirExpr::Field { base, .. }`, not `HirExpr::Def`.
                        if self.ctx.world.get::<kestrel_ast_builder::Callable>(*entity).is_some() {
                            self.ctx.register_name(*entity);
                            let result_ty = self.resolve_expr_type(expr_id);
                            // Static getter: no receiver, no type args
                            let callee = Callee::direct_generic(*entity, Vec::new());
                            self.emit_call(callee, Vec::new(), result_ty)
                        } else {
                            self.ctx.register_name(*entity);
                            Value::Place(Place::Global(*entity))
                        }
                    },
                    Some(kestrel_ast_builder::NodeKind::TypeParameter)
                    | Some(kestrel_ast_builder::NodeKind::TypeAlias) => {
                        // Type entities used as values — usually metatype references
                        // that don't have runtime representation
                        Value::Immediate(Immediate::unit())
                    },
                    _ => {
                        // Unknown entity — return error to avoid bad FunctionRef
                        Value::Immediate(Immediate::error())
                    },
                }
            },
            HirExpr::OverloadSet { candidates, .. } => {
                // Use type inference resolution to pick the right overload
                if let Some(&resolved) = self.typed.and_then(|t| t.resolutions.get(&expr_id)) {
                    self.ctx.register_name(resolved);
                    let type_args = self.resolve_type_args(expr_id);
                    Value::Immediate(Immediate::function_ref_generic(resolved, type_args))
                } else if let Some(&first) = candidates.first() {
                    self.ctx.register_name(first);
                    Value::Immediate(Immediate::function_ref(first))
                } else {
                    Value::Immediate(Immediate::error())
                }
            },

            // === Calls ===
            HirExpr::Call { callee, args, .. } => {
                self.lower_call(expr_id, *callee, args)
            },
            HirExpr::MethodCall {
                receiver,
                method,
                type_args: hir_type_args,
                args,
                ..
            } => self.lower_method_call(expr_id, *receiver, method, hir_type_args.as_deref(), args),
            HirExpr::ProtocolCall {
                receiver,
                protocol,
                method,
                args,
                ..
            } => self.lower_protocol_call(expr_id, *receiver, *protocol, method, args),

            // === Match expression ===
            HirExpr::Match {
                scrutinee, arms, ..
            } => self.lower_match(expr_id, *scrutinee, arms),

            // === Closures ===
            HirExpr::Closure { params, body, .. } => {
                self.lower_closure(expr_id, params, body)
            },

            // === Implicit member (.None, .Some(x), etc.) ===
            HirExpr::ImplicitMember { name, args, .. } => {
                let result_ty = self.resolve_expr_type(expr_id);

                // Check if inference resolved this to a static method (e.g., fromResidual)
                // rather than an enum case. Static methods need a call, not enum construction.
                let resolved = self.typed.and_then(|t| t.resolutions.get(&expr_id)).copied();
                let is_enum_case = resolved.map_or(true, |e| {
                    self.ctx.world.get::<kestrel_ast_builder::NodeKind>(e)
                        == Some(&kestrel_ast_builder::NodeKind::EnumCase)
                });

                if is_enum_case {
                    // Lower args as enum payload (e.g., .Some(value))
                    let payload: Vec<Value> = args
                        .as_ref()
                        .map(|a| a.iter().map(|arg| self.lower_expr(arg.value)).collect())
                        .unwrap_or_default();

                    let dest = self.fresh_temp(result_ty.clone());
                    self.emit_stmt(Statement::new(StatementKind::Assign {
                        dest: Place::local(dest),
                        rvalue: Rvalue::EnumVariant {
                            enum_ty: result_ty,
                            variant: name.clone(),
                            payload,
                        },
                    }));
                    Value::Place(Place::local(dest))
                } else {
                    // Static method call (e.g., .fromResidual(residual: early))
                    let resolved_entity = resolved.unwrap();
                    let call_args: Vec<kestrel_mir::CallArg> = args
                        .as_ref()
                        .map(|a| a.iter().map(|arg| {
                            let val = self.lower_expr(arg.value);
                            kestrel_mir::CallArg::copy(val)
                        }).collect())
                        .unwrap_or_default();

                    // Protocol method → Witness dispatch
                    if let Some(protocol) = self.find_protocol_for_method(resolved_entity) {
                        self.ctx.register_name(protocol);
                        let method_type_args = self.resolve_type_args(expr_id);
                        let type_args = self.prepend_receiver_type_args(&result_ty, method_type_args);
                        let callee = Callee::witness(protocol, name, result_ty.clone(), type_args);
                        self.emit_call(callee, call_args, result_ty)
                    } else {
                        // Direct static call
                        self.ctx.register_name(resolved_entity);
                        let type_args = self.resolve_type_args(expr_id);
                        let type_args = self.prepend_receiver_type_args(&result_ty, type_args);
                        let callee = Callee::direct_generic(resolved_entity, type_args);
                        self.emit_call(callee, call_args, result_ty)
                    }
                }
            },

            // === Array literal ===
            HirExpr::Array { elements, .. } => {
                let result_ty = self.resolve_expr_type(expr_id);
                if let Some(value) = self.lower_array_literal_via_init(elements, &result_ty) {
                    return value;
                }
                let values: Vec<Value> = elements.iter().map(|&e| self.lower_expr(e)).collect();

                // Extract element type from Array[T] type args
                let element_ty = match &result_ty {
                    MirTy::Named { type_args, .. } if !type_args.is_empty() => {
                        type_args[0].clone()
                    },
                    _ => MirTy::Error,
                };

                let dest = self.fresh_temp(result_ty);
                self.emit_stmt(Statement::new(StatementKind::Assign {
                    dest: Place::local(dest),
                    rvalue: Rvalue::ArrayLiteral {
                        element_ty,
                        values,
                    },
                }));
                Value::Place(Place::local(dest))
            },

            // === Dict literal — lowered as ArrayLiteral of (K, V) tuples ===
            HirExpr::Dict { entries, .. } => {
                let result_ty = self.resolve_expr_type(expr_id);

                // Extract key/value types from Dictionary[K, V, H] type args
                let (key_ty, value_ty) = match &result_ty {
                    MirTy::Named { type_args, .. } if type_args.len() >= 2 => {
                        (type_args[0].clone(), type_args[1].clone())
                    },
                    _ => (MirTy::Error, MirTy::Error),
                };

                let pair_ty = MirTy::Tuple(vec![key_ty.clone(), value_ty.clone()]);

                // Lower each entry to a (K, V) tuple
                let values: Vec<Value> = entries
                    .iter()
                    .map(|entry| {
                        let key = self.lower_expr(entry.key);
                        let val = self.lower_expr(entry.value);
                        // Emit a Tuple rvalue for each pair
                        let pair_dest = self.fresh_temp(pair_ty.clone());
                        self.emit_stmt(Statement::new(StatementKind::Assign {
                            dest: Place::local(pair_dest),
                            rvalue: Rvalue::Tuple(vec![key, val]),
                        }));
                        Value::Place(Place::local(pair_dest))
                    })
                    .collect();

                let dest = self.fresh_temp(result_ty);
                self.emit_stmt(Statement::new(StatementKind::Assign {
                    dest: Place::local(dest),
                    rvalue: Rvalue::ArrayLiteral {
                        element_ty: pair_ty,
                        values,
                    },
                }));
                Value::Place(Place::local(dest))
            },
        }
    }

    /// Resolve inferred type arguments for a generic call/reference.
    fn resolve_type_args(&mut self, expr_id: HirExprId) -> Vec<MirTy> {
        if let Some(typed) = self.typed {
            if let Some(resolved_args) = typed.type_args.get(&expr_id) {
                let args: Vec<MirTy> = resolved_args
                    .iter()
                    .map(|ty| lower_resolved_ty(self.ctx, ty))
                    .collect();
                return args;
            }
        }
        Vec::new()
    }

    /// Extract explicit type args from a HIR expression (Def, OverloadSet, MethodCall).
    /// Used as fallback when inference returns Error for type args.
    fn extract_explicit_type_args(&mut self, expr_id: HirExprId) -> Option<Vec<MirTy>> {
        let hir = self.hir.exprs[expr_id].clone();
        match &hir {
            HirExpr::Def(_, args, _) if !args.is_empty() => {
                Some(args.iter().map(|ty| lower_type(self.ctx, ty)).collect())
            }
            HirExpr::OverloadSet { type_args, .. } if !type_args.is_empty() => {
                Some(type_args.iter().map(|ty| lower_type(self.ctx, ty)).collect())
            }
            HirExpr::MethodCall { type_args: Some(args), .. } if !args.is_empty() => {
                Some(args.iter().map(|ty| lower_type(self.ctx, ty)).collect())
            }
            _ => None,
        }
    }

    /// Prepend the receiver's struct type_args to method-level type_args.
    /// Method FunctionDefs have inherited type_params first (from struct/enum/extension),
    /// followed by method-own type_params. The type_args must match this order.
    fn prepend_receiver_type_args(&self, receiver_ty: &MirTy, method_args: Vec<MirTy>) -> Vec<MirTy> {
        if let MirTy::Named { type_args, .. } = receiver_ty {
            if !type_args.is_empty() {
                let mut full_args = type_args.clone();
                full_args.extend(method_args);
                return full_args;
            }
        }
        method_args
    }

    /// Check if an entity is lang.panic or lang.panic_unwind.
    fn is_panic_intrinsic(&self, entity: Entity) -> bool {
        use kestrel_ast_builder::{Intrinsic, Name};
        if self.ctx.world.get::<Intrinsic>(entity).is_none() {
            return false;
        }
        let name = self.ctx.world.get::<Name>(entity).map(|n| n.0.as_str());
        matches!(name, Some("panic" | "panic_unwind"))
    }

    /// Try to lower a call to a lang intrinsic as a MIR Op instead of a function call.
    /// Returns Some(value) if the callee is an intrinsic, None otherwise.
    fn try_lower_intrinsic_call(
        &mut self,
        expr_id: HirExprId,
        callee_expr: HirExprId,
        args: &[HirCallArg],
    ) -> Option<Value> {
        use kestrel_ast_builder::{Intrinsic, Name};
        use kestrel_mir::{Op, IntBits, Signedness, FloatBits};
        use kestrel_mir::FloatConstantKind;
        use kestrel_mir::FloatPredicateKind;
        use kestrel_mir::FloatMathKind;

        // Get the callee entity — check both resolution paths
        let entity = if let Some(&resolved) = self.typed.and_then(|t| t.resolutions.get(&expr_id)) {
            resolved
        } else if let Some(&resolved) = self.typed.and_then(|t| t.resolutions.get(&callee_expr)) {
            resolved
        } else if let HirExpr::Def(e, _, _) = &self.hir.exprs[callee_expr] {
            *e
        } else {
            return None;
        };

        // Must have Intrinsic marker
        if self.ctx.world.get::<Intrinsic>(entity).is_none() {
            return None;
        }

        let name = self.ctx.world.get::<Name>(entity)?.0.clone();
        let result_ty = self.resolve_expr_type(expr_id);

        // Helper: lower arg at index
        let lower_arg = |this: &mut Self, idx: usize| -> Value {
            if idx < args.len() {
                this.lower_expr(args[idx].value)
            } else {
                Value::Immediate(Immediate::unit())
            }
        };

        // Helper: emit Op2 and return result
        let emit_op2 = |this: &mut Self, op: Op| -> Value {
            let lhs = this.lower_expr(args[0].value);
            let rhs = this.lower_expr(args[1].value);
            let dest = this.fresh_temp(result_ty.clone());
            this.emit_stmt(Statement::new(StatementKind::Assign {
                dest: Place::local(dest),
                rvalue: Rvalue::Op2 { op, lhs, rhs },
            }));
            Value::Place(Place::local(dest))
        };

        // Helper: emit Op1 and return result
        let emit_op1 = |this: &mut Self, op: Op| -> Value {
            let arg = this.lower_expr(args[0].value);
            let dest = this.fresh_temp(result_ty.clone());
            this.emit_stmt(Statement::new(StatementKind::Assign {
                dest: Place::local(dest),
                rvalue: Rvalue::Op1 { op, arg },
            }));
            Value::Place(Place::local(dest))
        };

        // Helper: emit Op3 (ternary, e.g. fma) and return result
        let emit_op3 = |this: &mut Self, op: Op| -> Value {
            let a = this.lower_expr(args[0].value);
            let b = this.lower_expr(args[1].value);
            let c = this.lower_expr(args[2].value);
            let dest = this.fresh_temp(result_ty.clone());
            this.emit_stmt(Statement::new(StatementKind::Assign {
                dest: Place::local(dest),
                rvalue: Rvalue::Op3 { op, a, b, c },
            }));
            Value::Place(Place::local(dest))
        };

        // Match intrinsic name to Op
        let val = match name.as_str() {
            // Boolean (i1) operations
            "i1_eq" => emit_op2(self, Op::BoolEq),
            "i1_and" => emit_op2(self, Op::BoolAnd),
            "i1_or" => emit_op2(self, Op::BoolOr),
            "i1_not" => emit_op1(self, Op::BoolNot),

            // Integer arithmetic — signed
            "i8_add" => emit_op2(self, Op::Add(IntBits::I8, Signedness::Signed)),
            "i8_sub" => emit_op2(self, Op::Sub(IntBits::I8, Signedness::Signed)),
            "i8_mul" => emit_op2(self, Op::Mul(IntBits::I8, Signedness::Signed)),
            "i8_signed_div" => emit_op2(self, Op::Div(IntBits::I8, Signedness::Signed)),
            "i8_signed_rem" => emit_op2(self, Op::Rem(IntBits::I8, Signedness::Signed)),
            "i16_add" => emit_op2(self, Op::Add(IntBits::I16, Signedness::Signed)),
            "i16_sub" => emit_op2(self, Op::Sub(IntBits::I16, Signedness::Signed)),
            "i16_mul" => emit_op2(self, Op::Mul(IntBits::I16, Signedness::Signed)),
            "i16_signed_div" => emit_op2(self, Op::Div(IntBits::I16, Signedness::Signed)),
            "i16_signed_rem" => emit_op2(self, Op::Rem(IntBits::I16, Signedness::Signed)),
            "i32_add" => emit_op2(self, Op::Add(IntBits::I32, Signedness::Signed)),
            "i32_sub" => emit_op2(self, Op::Sub(IntBits::I32, Signedness::Signed)),
            "i32_mul" => emit_op2(self, Op::Mul(IntBits::I32, Signedness::Signed)),
            "i32_signed_div" => emit_op2(self, Op::Div(IntBits::I32, Signedness::Signed)),
            "i32_signed_rem" => emit_op2(self, Op::Rem(IntBits::I32, Signedness::Signed)),
            "i64_add" => emit_op2(self, Op::Add(IntBits::I64, Signedness::Signed)),
            "i64_sub" => emit_op2(self, Op::Sub(IntBits::I64, Signedness::Signed)),
            "i64_mul" => emit_op2(self, Op::Mul(IntBits::I64, Signedness::Signed)),
            "i64_signed_div" => emit_op2(self, Op::Div(IntBits::I64, Signedness::Signed)),
            "i64_signed_rem" => emit_op2(self, Op::Rem(IntBits::I64, Signedness::Signed)),

            // Integer arithmetic — unsigned
            "i8_unsigned_div" => emit_op2(self, Op::Div(IntBits::I8, Signedness::Unsigned)),
            "i8_unsigned_rem" => emit_op2(self, Op::Rem(IntBits::I8, Signedness::Unsigned)),
            "i16_unsigned_div" => emit_op2(self, Op::Div(IntBits::I16, Signedness::Unsigned)),
            "i16_unsigned_rem" => emit_op2(self, Op::Rem(IntBits::I16, Signedness::Unsigned)),
            "i32_unsigned_div" => emit_op2(self, Op::Div(IntBits::I32, Signedness::Unsigned)),
            "i32_unsigned_rem" => emit_op2(self, Op::Rem(IntBits::I32, Signedness::Unsigned)),
            "i64_unsigned_div" => emit_op2(self, Op::Div(IntBits::I64, Signedness::Unsigned)),
            "i64_unsigned_rem" => emit_op2(self, Op::Rem(IntBits::I64, Signedness::Unsigned)),

            // Integer unary
            "i8_neg" => emit_op1(self, Op::Neg(IntBits::I8)),
            "i16_neg" => emit_op1(self, Op::Neg(IntBits::I16)),
            "i32_neg" => emit_op1(self, Op::Neg(IntBits::I32)),
            "i64_neg" => emit_op1(self, Op::Neg(IntBits::I64)),
            "i8_not" => emit_op1(self, Op::Not(IntBits::I8)),
            "i16_not" => emit_op1(self, Op::Not(IntBits::I16)),
            "i32_not" => emit_op1(self, Op::Not(IntBits::I32)),
            "i64_not" => emit_op1(self, Op::Not(IntBits::I64)),
            "i8_popcount" => emit_op1(self, Op::Popcount(IntBits::I8)),
            "i16_popcount" => emit_op1(self, Op::Popcount(IntBits::I16)),
            "i32_popcount" => emit_op1(self, Op::Popcount(IntBits::I32)),
            "i64_popcount" => emit_op1(self, Op::Popcount(IntBits::I64)),
            "i8_clz" => emit_op1(self, Op::Clz(IntBits::I8)),
            "i16_clz" => emit_op1(self, Op::Clz(IntBits::I16)),
            "i32_clz" => emit_op1(self, Op::Clz(IntBits::I32)),
            "i64_clz" => emit_op1(self, Op::Clz(IntBits::I64)),
            "i8_ctz" => emit_op1(self, Op::Ctz(IntBits::I8)),
            "i16_ctz" => emit_op1(self, Op::Ctz(IntBits::I16)),
            "i32_ctz" => emit_op1(self, Op::Ctz(IntBits::I32)),
            "i64_ctz" => emit_op1(self, Op::Ctz(IntBits::I64)),
            "i16_bswap" => emit_op1(self, Op::Bswap(IntBits::I16)),
            "i32_bswap" => emit_op1(self, Op::Bswap(IntBits::I32)),
            "i64_bswap" => emit_op1(self, Op::Bswap(IntBits::I64)),

            // Integer comparison — signed
            "i8_eq" => emit_op2(self, Op::Eq(IntBits::I8)),
            "i16_eq" => emit_op2(self, Op::Eq(IntBits::I16)),
            "i32_eq" => emit_op2(self, Op::Eq(IntBits::I32)),
            "i64_eq" => emit_op2(self, Op::Eq(IntBits::I64)),
            "i8_ne" => emit_op2(self, Op::Ne(IntBits::I8)),
            "i16_ne" => emit_op2(self, Op::Ne(IntBits::I16)),
            "i32_ne" => emit_op2(self, Op::Ne(IntBits::I32)),
            "i64_ne" => emit_op2(self, Op::Ne(IntBits::I64)),
            "i8_signed_lt" => emit_op2(self, Op::Lt(IntBits::I8, Signedness::Signed)),
            "i16_signed_lt" => emit_op2(self, Op::Lt(IntBits::I16, Signedness::Signed)),
            "i32_signed_lt" => emit_op2(self, Op::Lt(IntBits::I32, Signedness::Signed)),
            "i64_signed_lt" => emit_op2(self, Op::Lt(IntBits::I64, Signedness::Signed)),
            "i8_signed_le" => emit_op2(self, Op::Le(IntBits::I8, Signedness::Signed)),
            "i16_signed_le" => emit_op2(self, Op::Le(IntBits::I16, Signedness::Signed)),
            "i32_signed_le" => emit_op2(self, Op::Le(IntBits::I32, Signedness::Signed)),
            "i64_signed_le" => emit_op2(self, Op::Le(IntBits::I64, Signedness::Signed)),
            "i8_signed_gt" => emit_op2(self, Op::Gt(IntBits::I8, Signedness::Signed)),
            "i16_signed_gt" => emit_op2(self, Op::Gt(IntBits::I16, Signedness::Signed)),
            "i32_signed_gt" => emit_op2(self, Op::Gt(IntBits::I32, Signedness::Signed)),
            "i64_signed_gt" => emit_op2(self, Op::Gt(IntBits::I64, Signedness::Signed)),
            "i8_signed_ge" => emit_op2(self, Op::Ge(IntBits::I8, Signedness::Signed)),
            "i16_signed_ge" => emit_op2(self, Op::Ge(IntBits::I16, Signedness::Signed)),
            "i32_signed_ge" => emit_op2(self, Op::Ge(IntBits::I32, Signedness::Signed)),
            "i64_signed_ge" => emit_op2(self, Op::Ge(IntBits::I64, Signedness::Signed)),

            // Integer comparison — unsigned
            "i8_unsigned_lt" => emit_op2(self, Op::Lt(IntBits::I8, Signedness::Unsigned)),
            "i16_unsigned_lt" => emit_op2(self, Op::Lt(IntBits::I16, Signedness::Unsigned)),
            "i32_unsigned_lt" => emit_op2(self, Op::Lt(IntBits::I32, Signedness::Unsigned)),
            "i64_unsigned_lt" => emit_op2(self, Op::Lt(IntBits::I64, Signedness::Unsigned)),
            "i8_unsigned_le" => emit_op2(self, Op::Le(IntBits::I8, Signedness::Unsigned)),
            "i16_unsigned_le" => emit_op2(self, Op::Le(IntBits::I16, Signedness::Unsigned)),
            "i32_unsigned_le" => emit_op2(self, Op::Le(IntBits::I32, Signedness::Unsigned)),
            "i64_unsigned_le" => emit_op2(self, Op::Le(IntBits::I64, Signedness::Unsigned)),
            "i8_unsigned_gt" => emit_op2(self, Op::Gt(IntBits::I8, Signedness::Unsigned)),
            "i16_unsigned_gt" => emit_op2(self, Op::Gt(IntBits::I16, Signedness::Unsigned)),
            "i32_unsigned_gt" => emit_op2(self, Op::Gt(IntBits::I32, Signedness::Unsigned)),
            "i64_unsigned_gt" => emit_op2(self, Op::Gt(IntBits::I64, Signedness::Unsigned)),
            "i8_unsigned_ge" => emit_op2(self, Op::Ge(IntBits::I8, Signedness::Unsigned)),
            "i16_unsigned_ge" => emit_op2(self, Op::Ge(IntBits::I16, Signedness::Unsigned)),
            "i32_unsigned_ge" => emit_op2(self, Op::Ge(IntBits::I32, Signedness::Unsigned)),
            "i64_unsigned_ge" => emit_op2(self, Op::Ge(IntBits::I64, Signedness::Unsigned)),

            // Bitwise operations
            "i8_and" => emit_op2(self, Op::And(IntBits::I8)),
            "i16_and" => emit_op2(self, Op::And(IntBits::I16)),
            "i32_and" => emit_op2(self, Op::And(IntBits::I32)),
            "i64_and" => emit_op2(self, Op::And(IntBits::I64)),
            "i8_or" => emit_op2(self, Op::Or(IntBits::I8)),
            "i16_or" => emit_op2(self, Op::Or(IntBits::I16)),
            "i32_or" => emit_op2(self, Op::Or(IntBits::I32)),
            "i64_or" => emit_op2(self, Op::Or(IntBits::I64)),
            "i8_xor" => emit_op2(self, Op::Xor(IntBits::I8)),
            "i16_xor" => emit_op2(self, Op::Xor(IntBits::I16)),
            "i32_xor" => emit_op2(self, Op::Xor(IntBits::I32)),
            "i64_xor" => emit_op2(self, Op::Xor(IntBits::I64)),
            "i8_shl" => emit_op2(self, Op::Shl(IntBits::I8)),
            "i16_shl" => emit_op2(self, Op::Shl(IntBits::I16)),
            "i32_shl" => emit_op2(self, Op::Shl(IntBits::I32)),
            "i64_shl" => emit_op2(self, Op::Shl(IntBits::I64)),
            "i8_signed_shr" => emit_op2(self, Op::Shr(IntBits::I8, Signedness::Signed)),
            "i16_signed_shr" => emit_op2(self, Op::Shr(IntBits::I16, Signedness::Signed)),
            "i32_signed_shr" => emit_op2(self, Op::Shr(IntBits::I32, Signedness::Signed)),
            "i64_signed_shr" => emit_op2(self, Op::Shr(IntBits::I64, Signedness::Signed)),
            "i8_unsigned_shr" => emit_op2(self, Op::Shr(IntBits::I8, Signedness::Unsigned)),
            "i16_unsigned_shr" => emit_op2(self, Op::Shr(IntBits::I16, Signedness::Unsigned)),
            "i32_unsigned_shr" => emit_op2(self, Op::Shr(IntBits::I32, Signedness::Unsigned)),
            "i64_unsigned_shr" => emit_op2(self, Op::Shr(IntBits::I64, Signedness::Unsigned)),

            // Integer casts
            "cast_i8_i16" => emit_op1(self, Op::IntWiden(IntBits::I8, IntBits::I16)),
            "cast_i8_i32" => emit_op1(self, Op::IntWiden(IntBits::I8, IntBits::I32)),
            "cast_i8_i64" => emit_op1(self, Op::IntWiden(IntBits::I8, IntBits::I64)),
            "cast_i16_i32" => emit_op1(self, Op::IntWiden(IntBits::I16, IntBits::I32)),
            "cast_i16_i64" => emit_op1(self, Op::IntWiden(IntBits::I16, IntBits::I64)),
            "cast_i32_i64" => emit_op1(self, Op::IntWiden(IntBits::I32, IntBits::I64)),
            "cast_i64_i32" => emit_op1(self, Op::IntTruncate(IntBits::I64, IntBits::I32)),
            "cast_i64_i16" => emit_op1(self, Op::IntTruncate(IntBits::I64, IntBits::I16)),
            "cast_i64_i8" => emit_op1(self, Op::IntTruncate(IntBits::I64, IntBits::I8)),
            "cast_i32_i16" => emit_op1(self, Op::IntTruncate(IntBits::I32, IntBits::I16)),
            "cast_i32_i8" => emit_op1(self, Op::IntTruncate(IntBits::I32, IntBits::I8)),
            "cast_i16_i8" => emit_op1(self, Op::IntTruncate(IntBits::I16, IntBits::I8)),
            // Unsigned → signed widenings (zero-extend)
            "cast_u8_i16" => emit_op1(self, Op::IntUnsignedWiden(IntBits::I8, IntBits::I16)),
            "cast_u8_i32" => emit_op1(self, Op::IntUnsignedWiden(IntBits::I8, IntBits::I32)),
            "cast_u8_i64" => emit_op1(self, Op::IntUnsignedWiden(IntBits::I8, IntBits::I64)),
            "cast_u16_i32" => emit_op1(self, Op::IntUnsignedWiden(IntBits::I16, IntBits::I32)),
            "cast_u16_i64" => emit_op1(self, Op::IntUnsignedWiden(IntBits::I16, IntBits::I64)),
            "cast_u16_i8" => emit_op1(self, Op::IntTruncate(IntBits::I16, IntBits::I8)),
            "cast_u32_i64" => emit_op1(self, Op::IntUnsignedWiden(IntBits::I32, IntBits::I64)),
            "cast_u32_i16" => emit_op1(self, Op::IntTruncate(IntBits::I32, IntBits::I16)),
            "cast_u32_i8" => emit_op1(self, Op::IntTruncate(IntBits::I32, IntBits::I8)),
            "cast_u64_i32" => emit_op1(self, Op::IntTruncate(IntBits::I64, IntBits::I32)),
            "cast_u64_i16" => emit_op1(self, Op::IntTruncate(IntBits::I64, IntBits::I16)),
            "cast_u64_i8" => emit_op1(self, Op::IntTruncate(IntBits::I64, IntBits::I8)),

            // Float arithmetic
            "f32_add" => emit_op2(self, Op::FAdd(FloatBits::F32)),
            "f32_sub" => emit_op2(self, Op::FSub(FloatBits::F32)),
            "f32_mul" => emit_op2(self, Op::FMul(FloatBits::F32)),
            "f32_div" => emit_op2(self, Op::FDiv(FloatBits::F32)),
            "f32_neg" => emit_op1(self, Op::FNeg(FloatBits::F32)),
            "f64_add" => emit_op2(self, Op::FAdd(FloatBits::F64)),
            "f64_sub" => emit_op2(self, Op::FSub(FloatBits::F64)),
            "f64_mul" => emit_op2(self, Op::FMul(FloatBits::F64)),
            "f64_div" => emit_op2(self, Op::FDiv(FloatBits::F64)),
            "f64_neg" => emit_op1(self, Op::FNeg(FloatBits::F64)),

            // Float comparison
            "f32_eq" => emit_op2(self, Op::FEq(FloatBits::F32)),
            "f32_ne" => emit_op2(self, Op::FNe(FloatBits::F32)),
            "f32_lt" => emit_op2(self, Op::FLt(FloatBits::F32)),
            "f32_le" => emit_op2(self, Op::FLe(FloatBits::F32)),
            "f32_gt" => emit_op2(self, Op::FGt(FloatBits::F32)),
            "f32_ge" => emit_op2(self, Op::FGe(FloatBits::F32)),
            "f64_eq" => emit_op2(self, Op::FEq(FloatBits::F64)),
            "f64_ne" => emit_op2(self, Op::FNe(FloatBits::F64)),
            "f64_lt" => emit_op2(self, Op::FLt(FloatBits::F64)),
            "f64_le" => emit_op2(self, Op::FLe(FloatBits::F64)),
            "f64_gt" => emit_op2(self, Op::FGt(FloatBits::F64)),
            "f64_ge" => emit_op2(self, Op::FGe(FloatBits::F64)),

            // Float casts
            "cast_i64_f64" => emit_op1(self, Op::IntToFloat(IntBits::I64, FloatBits::F64)),
            "cast_i64_f32" => emit_op1(self, Op::IntToFloat(IntBits::I64, FloatBits::F32)),
            "cast_i32_f64" => emit_op1(self, Op::IntToFloat(IntBits::I32, FloatBits::F64)),
            "cast_i32_f32" => emit_op1(self, Op::IntToFloat(IntBits::I32, FloatBits::F32)),
            "cast_f64_i64" => emit_op1(self, Op::FloatToInt(FloatBits::F64, IntBits::I64)),
            "cast_f32_i64" => emit_op1(self, Op::FloatToInt(FloatBits::F32, IntBits::I64)),
            "cast_f64_i32" => emit_op1(self, Op::FloatToInt(FloatBits::F64, IntBits::I32)),
            "cast_f32_i32" => emit_op1(self, Op::FloatToInt(FloatBits::F32, IntBits::I32)),
            "cast_f32_f64" => emit_op1(self, Op::FloatWiden(FloatBits::F32, FloatBits::F64)),
            "cast_f64_f32" => emit_op1(self, Op::FloatTruncate(FloatBits::F64, FloatBits::F32)),

            // Float intrinsics
            "f32_floor" => emit_op1(self, Op::FloatMath(FloatBits::F32, FloatMathKind::Floor)),
            "f32_ceil" => emit_op1(self, Op::FloatMath(FloatBits::F32, FloatMathKind::Ceil)),
            "f32_round" => emit_op1(self, Op::FloatMath(FloatBits::F32, FloatMathKind::Round)),
            "f32_trunc" => emit_op1(self, Op::FloatMath(FloatBits::F32, FloatMathKind::Trunc)),
            "f32_sqrt" => emit_op1(self, Op::FloatMath(FloatBits::F32, FloatMathKind::Sqrt)),
            "f64_floor" => emit_op1(self, Op::FloatMath(FloatBits::F64, FloatMathKind::Floor)),
            "f64_ceil" => emit_op1(self, Op::FloatMath(FloatBits::F64, FloatMathKind::Ceil)),
            "f64_round" => emit_op1(self, Op::FloatMath(FloatBits::F64, FloatMathKind::Round)),
            "f64_trunc" => emit_op1(self, Op::FloatMath(FloatBits::F64, FloatMathKind::Trunc)),
            "f64_sqrt" => emit_op1(self, Op::FloatMath(FloatBits::F64, FloatMathKind::Sqrt)),
            "f32_is_nan" => emit_op1(self, Op::FloatPred(FloatBits::F32, FloatPredicateKind::IsNan)),
            "f32_is_infinite" => emit_op1(self, Op::FloatPred(FloatBits::F32, FloatPredicateKind::IsInfinite)),
            "f64_is_nan" => emit_op1(self, Op::FloatPred(FloatBits::F64, FloatPredicateKind::IsNan)),
            "f64_is_infinite" => emit_op1(self, Op::FloatPred(FloatBits::F64, FloatPredicateKind::IsInfinite)),
            "f32_fma" => emit_op3(self, Op::FloatFma(FloatBits::F32)),
            "f64_fma" => emit_op3(self, Op::FloatFma(FloatBits::F64)),
            "f32_copysign" => emit_op2(self, Op::FloatCopysign(FloatBits::F32)),
            "f64_copysign" => emit_op2(self, Op::FloatCopysign(FloatBits::F64)),
            "f32_infinity" => {
                let dest = self.fresh_temp(result_ty);
                self.emit_stmt(Statement::new(StatementKind::Assign {
                    dest: Place::local(dest),
                    rvalue: Rvalue::Op1 { op: Op::FloatConst(FloatBits::F32, FloatConstantKind::Infinity), arg: Value::Immediate(Immediate::unit()) },
                }));
                Value::Place(Place::local(dest))
            }
            "f64_infinity" => {
                let dest = self.fresh_temp(result_ty);
                self.emit_stmt(Statement::new(StatementKind::Assign {
                    dest: Place::local(dest),
                    rvalue: Rvalue::Op1 { op: Op::FloatConst(FloatBits::F64, FloatConstantKind::Infinity), arg: Value::Immediate(Immediate::unit()) },
                }));
                Value::Place(Place::local(dest))
            }
            "f32_nan" => {
                let dest = self.fresh_temp(result_ty);
                self.emit_stmt(Statement::new(StatementKind::Assign {
                    dest: Place::local(dest),
                    rvalue: Rvalue::Op1 { op: Op::FloatConst(FloatBits::F32, FloatConstantKind::Nan), arg: Value::Immediate(Immediate::unit()) },
                }));
                Value::Place(Place::local(dest))
            }
            "f64_nan" => {
                let dest = self.fresh_temp(result_ty);
                self.emit_stmt(Statement::new(StatementKind::Assign {
                    dest: Place::local(dest),
                    rvalue: Rvalue::Op1 { op: Op::FloatConst(FloatBits::F64, FloatConstantKind::Nan), arg: Value::Immediate(Immediate::unit()) },
                }));
                Value::Place(Place::local(dest))
            }

            // Pointer operations
            "ptr_null" => {
                let pointee = self.resolve_expr_type(expr_id);
                let inner = match &pointee { MirTy::Pointer(inner) => *inner.clone(), _ => MirTy::I8 };
                let dest = self.fresh_temp(result_ty);
                self.emit_stmt(Statement::new(StatementKind::Assign {
                    dest: Place::local(dest),
                    rvalue: Rvalue::Op1 { op: Op::PtrNull(inner), arg: Value::Immediate(Immediate::unit()) },
                }));
                Value::Place(Place::local(dest))
            }
            "ptr_offset" => emit_op2(self, Op::PtrOffset),
            "ptr_to_address" => emit_op1(self, Op::PtrToAddress),
            "ptr_is_null" => emit_op1(self, Op::PtrIsNull),
            "ptr_read" => {
                let pointee = self.resolve_expr_type(expr_id);
                emit_op1(self, Op::PtrRead(pointee))
            }
            "ptr_write" => {
                // Carry the value type so codegen can copy aggregates
                let value_ty = self.resolve_expr_type(args[1].value);
                emit_op2(self, Op::PtrWrite(value_ty))
            }
            "cast_ptr" => {
                // cast_ptr[T](ptr) → pointer cast to Pointer[T]
                let target_ty = self.resolve_expr_type(expr_id);
                emit_op1(self, Op::PtrCast(target_ty))
            }
            "ptr_to" => {
                // ptr_to[T](value) → take address of value, returns Pointer[T]
                // This is like RefToPtr — takes a reference and returns a raw pointer
                emit_op1(self, Op::RefToPtr)
            }
            "ptr_from_address" => {
                // ptr_from_address[T](address) → create Pointer[T] from integer address
                let target_ty = self.resolve_expr_type(expr_id);
                emit_op1(self, Op::PtrFromAddress(target_ty))
            }
            "stack_alloc" => {
                // stack_alloc[T](count) → allocate count*sizeof(T) bytes on stack
                let target_ty = self.resolve_expr_type(expr_id);
                let inner = match &target_ty { MirTy::Pointer(inner) => *inner.clone(), _ => MirTy::I8 };
                emit_op1(self, Op::StackAlloc(inner))
            }

            // String operations
            "str_ptr" => emit_op1(self, Op::StrPtr),
            "str_len" => emit_op1(self, Op::StrLen),

            // Memory
            "sizeof" | "size_of" => {
                // sizeof[T]() — the type arg T is what we measure, not the return type
                let type_args = self.resolve_type_args(callee_expr);
                let ty = type_args.into_iter().next().unwrap_or(self.resolve_expr_type(expr_id));
                let dest = self.fresh_temp(result_ty);
                self.emit_stmt(Statement::new(StatementKind::Assign {
                    dest: Place::local(dest),
                    rvalue: Rvalue::Op1 { op: Op::SizeOf(ty), arg: Value::Immediate(Immediate::unit()) },
                }));
                Value::Place(Place::local(dest))
            }
            "alignof" | "align_of" => {
                // alignof[T]() — same as sizeof, extract the type arg
                let type_args = self.resolve_type_args(callee_expr);
                let ty = type_args.into_iter().next().unwrap_or(self.resolve_expr_type(expr_id));
                let dest = self.fresh_temp(result_ty);
                self.emit_stmt(Statement::new(StatementKind::Assign {
                    dest: Place::local(dest),
                    rvalue: Rvalue::Op1 { op: Op::AlignOf(ty), arg: Value::Immediate(Immediate::unit()) },
                }));
                Value::Place(Place::local(dest))
            }

            // Atomic
            "atomic_add" => emit_op2(self, Op::AtomicAdd),
            "atomic_sub" => emit_op2(self, Op::AtomicSub),

            // Not a recognized intrinsic — fall through to regular call
            _ => return None,
        };

        Some(val)
    }

    /// Resolve the type of a field on a struct type.
    /// Resolve the type of a field on a struct type, substituting type params.
    fn resolve_field_type(&self, struct_ty: &MirTy, field_name: &str) -> MirTy {
        if let MirTy::Named { entity, type_args } = struct_ty {
            for s in &self.ctx.module.structs {
                if s.entity == *entity {
                    for field in &s.fields {
                        if field.name == field_name {
                            if s.type_params.is_empty() || type_args.is_empty() {
                                return field.ty.clone();
                            }
                            // Build subst and apply manually
                            let subst: std::collections::HashMap<Entity, MirTy> = s.type_params
                                .iter()
                                .zip(type_args.iter())
                                .map(|(tp, arg)| (tp.entity, arg.clone()))
                                .collect();
                            return self.substitute_mir_type(&field.ty, &subst);
                        }
                    }
                }
            }
        }
        MirTy::Unit
    }

    /// Simple recursive type substitution (replaces TypeParam entities in the subst map).
    fn substitute_mir_type(&self, ty: &MirTy, subst: &std::collections::HashMap<Entity, MirTy>) -> MirTy {
        match ty {
            MirTy::TypeParam(e) => subst.get(e).cloned().unwrap_or_else(|| ty.clone()),
            MirTy::Named { entity, type_args } => {
                if let Some(replacement) = subst.get(entity) {
                    return replacement.clone();
                }
                MirTy::Named {
                    entity: *entity,
                    type_args: type_args.iter().map(|a| self.substitute_mir_type(a, subst)).collect(),
                }
            }
            MirTy::Pointer(inner) => MirTy::Pointer(Box::new(self.substitute_mir_type(inner, subst))),
            MirTy::Ref(inner) => MirTy::Ref(Box::new(self.substitute_mir_type(inner, subst))),
            MirTy::Tuple(elems) => MirTy::Tuple(elems.iter().map(|e| self.substitute_mir_type(e, subst)).collect()),
            _ => ty.clone(),
        }
    }

    /// Find the subscript getter entity for a struct/enum type.
    /// Searches through children and extensions for a Subscript with a Callable.
    fn find_subscript_getter(&self, type_entity: Entity) -> Option<Entity> {
        use kestrel_ast_builder::NodeKind;
        // Search direct children of the type
        for &child in self.ctx.world.children_of(type_entity) {
            if self.ctx.world.get::<NodeKind>(child) == Some(&NodeKind::Subscript) {
                // The subscript entity itself has the Callable component
                if self.ctx.world.get::<kestrel_ast_builder::Callable>(child).is_some() {
                    return Some(child);
                }
            }
        }
        // Also check extensions
        for &child in self.ctx.world.children_of(type_entity) {
            if self.ctx.world.get::<NodeKind>(child) == Some(&NodeKind::Extension) {
                for &ext_child in self.ctx.world.children_of(child) {
                    if self.ctx.world.get::<NodeKind>(ext_child) == Some(&NodeKind::Subscript) {
                        if self.ctx.world.get::<kestrel_ast_builder::Callable>(ext_child).is_some() {
                            return Some(ext_child);
                        }
                    }
                }
            }
        }
        None
    }

    /// Get the method name for an entity, handling init/subscript/deinit which lack Name.
    fn method_name_of(&self, entity: Entity) -> String {
        use kestrel_ast_builder::NodeKind;
        self.ctx.world.get::<kestrel_ast_builder::Name>(entity)
            .map(|n| n.0.clone())
            .unwrap_or_else(|| {
                match self.ctx.world.get::<NodeKind>(entity) {
                    Some(NodeKind::Initializer) => "init".to_string(),
                    Some(NodeKind::Subscript) => "subscript".to_string(),
                    Some(NodeKind::Deinit) => "deinit".to_string(),
                    _ => String::new(),
                }
            })
    }

    /// Check if a MirTy is a protocol type (Named whose entity is a protocol).
    fn is_protocol_type(&self, ty: &MirTy) -> bool {
        if let MirTy::Named { entity, type_args } = ty {
            if type_args.is_empty() {
                return self.ctx.world.get::<kestrel_ast_builder::NodeKind>(*entity)
                    == Some(&kestrel_ast_builder::NodeKind::Protocol);
            }
        }
        false
    }

    /// If a method entity belongs to a protocol (abstract or extension), return the
    /// protocol entity. Both abstract protocol methods and protocol extension methods
    /// need Witness dispatch so the witness table can route to the correct implementation.
    fn find_protocol_for_method(&self, method: Entity) -> Option<Entity> {
        use kestrel_ast_builder::NodeKind;
        let parent = self.ctx.world.parent_of(method)?;
        let parent_kind = self.ctx.world.get::<NodeKind>(parent)?;
        match parent_kind {
            // Abstract protocol method (no body) — always needs Witness
            NodeKind::Protocol => Some(parent),
            // Protocol extension method (has default body) — also needs Witness
            // so the witness table can route to overrides or the default impl
            NodeKind::Extension => {
                use kestrel_name_res::extensions::ExtensionTargetEntity;
                let target = self.ctx.query.query(ExtensionTargetEntity {
                    extension: parent,
                    root: self.ctx.root,
                })?;
                let target_kind = self.ctx.world.get::<NodeKind>(target)?;
                match target_kind {
                    NodeKind::Protocol => Some(target),
                    _ => None, // Struct/Enum extension — Direct dispatch is fine
                }
            }
            _ => None,
        }
    }

    /// Check if an entity is a struct (via ECS NodeKind, not just MIR module).
    fn is_struct_entity(&self, entity: Entity) -> bool {
        self.ctx.world.get::<kestrel_ast_builder::NodeKind>(entity)
            == Some(&kestrel_ast_builder::NodeKind::Struct)
    }

    /// For static methods on generic structs (e.g., Pointer[Int32].nullPointer()),
    /// the struct's type args aren't on the method entity — they're on the parent.
    /// Look up the MIR FunctionDef, find its parent struct, and extract the parent's
    /// concrete type args from inference or the result type.
    fn infer_parent_type_args(
        &mut self,
        func_entity: Entity,
        expr_id: HirExprId,
        callee_expr: HirExprId,
    ) -> Vec<MirTy> {
        // Find the MIR function def
        let func_def = self.ctx.module.functions.iter().find(|f| f.entity == func_entity);
        let Some(func_def) = func_def else { return Vec::new() };

        // Must have inherited type params (from parent struct)
        if func_def.type_params.is_empty() {
            return Vec::new();
        }

        // Get the parent entity from the function kind
        let parent_entity = match &func_def.kind {
            FunctionKind::StaticMethod { parent } |
            FunctionKind::Method { parent, .. } |
            FunctionKind::Initializer { parent } => Some(*parent),
            _ => None,
        };
        let Some(parent) = parent_entity else { return Vec::new() };

        // Check if parent is a generic struct/enum
        let parent_type_params = self.ctx.world.get::<kestrel_ast_builder::TypeParams>(parent);
        let Some(parent_tps) = parent_type_params else { return Vec::new() };
        if parent_tps.0.is_empty() {
            return Vec::new();
        }
        let parent_tp_count = parent_tps.0.len();

        // Strategy 1: Check inference type_args for callee_expr and expr_id
        if let Some(typed) = self.typed {
            for &eid in &[callee_expr, expr_id] {
                if let Some(resolved_args) = typed.type_args.get(&eid) {
                    if resolved_args.len() >= parent_tp_count {
                        return resolved_args.iter()
                            .map(|ty| lower_resolved_ty(self.ctx, ty))
                            .collect();
                    }
                }
            }
        }

        // Strategy 2: Extract from the result type if it's a Named type
        // containing the parent (e.g., nullPointer() -> Pointer[Int32])
        let result_ty = self.resolve_expr_type(expr_id);
        if let MirTy::Named { entity, type_args } = &result_ty {
            if *entity == parent && type_args.len() == parent_tp_count {
                return type_args.clone();
            }
        }

        // Strategy 3: Check explicit HIR type args on the Def expression
        // For paths like Pointer[Int32].nullPointer, the Def might carry [Int32]
        if let HirExpr::Def(_, hir_args, _) = &self.hir.exprs[callee_expr] {
            if hir_args.len() == parent_tp_count {
                return hir_args.iter()
                    .map(|hir_ty| crate::ty::lower_type(self.ctx, hir_ty))
                    .collect();
            }
        }

        Vec::new()
    }

    /// If entity is a struct, resolve its init function. Otherwise return entity as-is.
    fn resolve_callee_entity(&mut self, entity: Entity, args: &[HirCallArg]) -> Entity {
        if self.is_struct_entity(entity) {
            self.resolve_init_function(entity, args).unwrap_or(entity)
        } else {
            entity
        }
    }

    /// Resolve the init function for a struct entity by finding its Initializer children.
    /// Falls back to the first init if multiple match or returns None.
    fn resolve_init_function(&mut self, struct_entity: Entity, args: &[HirCallArg]) -> Option<Entity> {
        use kestrel_ast_builder::{Callable, NodeKind};

        let arg_count = args.len();
        let arg_labels: Vec<Option<&str>> = args.iter().map(|a| a.label.as_deref()).collect();

        // Search for initializer children of the struct
        let children = self.ctx.world.children_of(struct_entity).to_vec();
        let mut best: Option<Entity> = None;

        for &child in &children {
            let Some(kind) = self.ctx.world.get::<NodeKind>(child) else { continue };
            if *kind != NodeKind::Initializer { continue }

            // Keep first init as fallback regardless of param matching
            if best.is_none() {
                best = Some(child);
            }

            let Some(callable) = self.ctx.world.get::<Callable>(child) else { continue };

            // Match by param count and labels
            if callable.params.len() != arg_count { continue }
            let labels_ok = callable.params.iter().zip(arg_labels.iter()).all(|(p, arg_label)| {
                match (p.label.as_deref(), arg_label) {
                    (Some(pl), Some(al)) => pl == *al,
                    (None, None) => true,
                    _ => false,
                }
            });
            if labels_ok {
                best = Some(child);
                break;
            }
        }

        // Also search extensions for init
        if best.is_none() {
            let extensions = self.ctx.query.query(kestrel_name_res::ExtensionsFor {
                target: struct_entity,
                root: self.ctx.root,
            });
            for ext in &extensions {
                for &child in self.ctx.world.children_of(*ext) {
                    let Some(kind) = self.ctx.world.get::<NodeKind>(child) else { continue };
                    if *kind != NodeKind::Initializer { continue }
                    best = Some(child);
                    break;
                }
                if best.is_some() { break }
            }
        }

        best
    }

    /// Lower call arguments from HIR to MIR.
    /// Trivially copyable types (primitives, refs, pointers, thin func ptrs)
    /// are passed by copy. Everything else is passed by borrow.
    fn lower_call_args(&mut self, args: &[HirCallArg]) -> Vec<CallArg> {
        args.iter()
            .map(|arg| {
                let value = self.lower_expr(arg.value);
                let arg_ty = self.resolve_expr_type(arg.value);
                if arg_ty.is_trivially_copyable() {
                    CallArg::copy(value)
                } else {
                    CallArg::borrow(value)
                }
            })
            .collect()
    }

    /// After lowering args, override their passing modes to match the callee's
    /// `mutating`/`consuming` parameter declarations. Indexes 1:1 against the
    /// callee's `params` (which include `self` at index 0 for instance methods).
    fn apply_callee_param_modes(&self, call_args: &mut [CallArg], callee_entity: kestrel_hecs::Entity) {
        let Some(callee) = self
            .ctx
            .module
            .functions
            .iter()
            .find(|f| f.entity == callee_entity)
        else {
            return;
        };
        for (arg, param) in call_args.iter_mut().zip(callee.params.iter()) {
            match param.mode {
                kestrel_mir::ParamMode::InOut => {
                    arg.mode = kestrel_mir::PassingMode::MutRef;
                }
                kestrel_mir::ParamMode::Consuming => {
                    arg.mode = kestrel_mir::PassingMode::Move;
                }
                kestrel_mir::ParamMode::In => {}
            }
        }
    }

    /// Lower a direct call: `callee(args...)`
    fn lower_call(
        &mut self,
        expr_id: HirExprId,
        callee_expr: HirExprId,
        args: &[HirCallArg],
    ) -> Value {
        // Intercept lang.panic / lang.panic_unwind — emit as Panic terminator, not a call
        if let HirExpr::Def(entity, _, _) = &self.hir.exprs[callee_expr] {
            if self.is_panic_intrinsic(*entity) {
                let msg = "panic".to_string();
                self.set_terminator(Terminator::panic(msg));
                return Value::Immediate(Immediate::unit());
            }
        }

        // Intercept lang intrinsics — emit as MIR Ops, not function calls
        if let Some(val) = self.try_lower_intrinsic_call(expr_id, callee_expr, args) {
            return val;
        }

        // Intercept enum case construction: `Foo.Bar(args)` must emit Rvalue::EnumVariant,
        // not a function call. Enum cases are not real functions — only the parent enum's
        // EnumDef and per-case payload StructDefs exist in MIR, so a Direct callee with
        // the case entity would fail symbol lookup at codegen.
        if let HirExpr::Def(entity, _, _) = &self.hir.exprs[callee_expr] {
            let entity = *entity;
            if matches!(
                self.ctx.world.get::<kestrel_ast_builder::NodeKind>(entity),
                Some(kestrel_ast_builder::NodeKind::EnumCase)
            ) {
                // Inference often doesn't tag the Call expr's type for case
                // construction — fall back to deriving from the parent enum.
                let inferred = self.resolve_expr_type(expr_id);
                let result_ty = if matches!(inferred, MirTy::Error) {
                    if let Some(parent) = self.ctx.world.parent_of(entity) {
                        self.ctx.register_name(parent);
                        // Generic enum type args (B3) come from explicit_type_args
                        // on the callee Def — try to resolve them.
                        let type_args = self.resolve_type_args(callee_expr);
                        MirTy::Named { entity: parent, type_args }
                    } else {
                        inferred
                    }
                } else {
                    inferred
                };
                let case_name = self.ctx.world
                    .get::<kestrel_ast_builder::Name>(entity)
                    .map(|n| n.0.clone())
                    .unwrap_or_default();
                let payload: Vec<Value> = args.iter()
                    .map(|arg| self.lower_expr(arg.value))
                    .collect();
                let dest = self.fresh_temp(result_ty.clone());
                self.emit_stmt(Statement::new(StatementKind::Assign {
                    dest: Place::local(dest),
                    rvalue: Rvalue::EnumVariant {
                        enum_ty: result_ty,
                        variant: case_name,
                        payload,
                    },
                }));
                return Value::Place(Place::local(dest));
            }
        }

        let call_args = self.lower_call_args(args);
        let result_ty = self.resolve_expr_type(expr_id);

        // Check if inference resolved the call expression itself (e.g., init calls
        // where Int64(intLiteral: 0) resolves to the specific init function entity,
        // or subscript calls where arr(index) resolves to the subscript function)
        if let Some(&resolved) = self.typed.and_then(|t| t.resolutions.get(&expr_id)) {
            let func_entity = self.resolve_callee_entity(resolved, args);
            self.ctx.register_name(func_entity);

            // Expand default arguments for missing params
            let explicit_count = args.len();
            let call_args = self.expand_default_args(call_args, func_entity, explicit_count);

            let mut type_args = self.resolve_type_args(expr_id);
            if type_args.is_empty() {
                type_args = self.resolve_type_args(callee_expr);
            }
            // Use explicit type args from the path (e.g., Array[Int64](...))
            // Also fall back when inference returned Error (unresolved types)
            let has_error = type_args.iter().any(|a| matches!(a, MirTy::Error));
            if has_error || type_args.is_empty() {
                if let Some(fallback) = self.extract_explicit_type_args(callee_expr) {
                    type_args = fallback;
                } else if has_error {
                    let callee_hir = &self.hir.exprs[callee_expr];
                    let func_name = &self.ctx.module.functions[self.func_id.index()].name;
                    eprintln!("[DIAG] No fallback for Error in {} — callee variant: {}", func_name, match callee_hir {
                        HirExpr::Def(..) => "Def",
                        HirExpr::OverloadSet { .. } => "OverloadSet",
                        HirExpr::MethodCall { .. } => "MethodCall",
                        HirExpr::Call { .. } => "Call",
                        HirExpr::Local(..) => "Local",
                        HirExpr::Field { .. } => "Field",
                        _ => "Other",
                    });
                }
            }

            // For static methods on generic structs, type_args may be empty because
            // the struct's type args aren't on the method entity. Extract from parent.
            if type_args.is_empty() {
                type_args = self.infer_parent_type_args(func_entity, expr_id, callee_expr);
            }

            // Protocol method → Witness dispatch
            if let Some(protocol) = self.find_protocol_for_method(func_entity) {
                let method_name = self.method_name_of(func_entity);
                let self_type = if method_name == "init" {
                    result_ty.clone()
                } else {
                    self.resolve_expr_type(callee_expr)
                };
                self.ctx.register_name(protocol);
                let callee = Callee::witness(protocol, &method_name, self_type, type_args);
                return self.emit_call_maybe_init(callee, call_args, result_ty);
            }

            // If the resolved function has a receiver (subscript/computed property call),
            // the callee expression is the receiver — add it as the first arg
            let mut call_args = call_args;
            let has_receiver = self.ctx.world.get::<kestrel_ast_builder::Callable>(func_entity)
                .map_or(false, |c| c.receiver.is_some());
            // Init functions handle their own self-allocation via emit_call_maybe_init
            let is_init = self.is_init_function(func_entity).is_some();
            if has_receiver && !is_init {
                let receiver_ty = self.resolve_expr_type(callee_expr);
                let receiver_val = self.lower_expr(callee_expr);
                let receiver_arg = if receiver_ty.is_trivially_copyable() {
                    CallArg::copy(receiver_val)
                } else {
                    CallArg::borrow(receiver_val)
                };
                let type_args = self.prepend_receiver_type_args(&receiver_ty, type_args);
                let callee = Callee::method(func_entity, type_args, receiver_ty);
                call_args.insert(0, receiver_arg);
                return self.emit_call(callee, call_args, result_ty);
            }

            let callee = Callee::direct_generic(func_entity, type_args);
            return self.emit_call_maybe_init(callee, call_args, result_ty);
        }

        // Check what the callee is
        let callee_hir = self.hir.exprs[callee_expr].clone();
        match &callee_hir {
            // Direct function call: foo(args) or foo[T](args)
            HirExpr::Def(entity, explicit_hir_args, _) => {
                let func_entity = self.resolve_callee_entity(*entity, args);
                self.ctx.register_name(func_entity);
                let mut type_args = self.resolve_type_args(callee_expr);
                // Fall back to explicit HIR type args if inference didn't resolve them
                let has_error = type_args.iter().any(|a| matches!(a, MirTy::Error));
                if (type_args.is_empty() || has_error) && !explicit_hir_args.is_empty() {
                    type_args = explicit_hir_args
                        .iter()
                        .map(|hir_ty| lower_type(self.ctx, hir_ty))
                        .collect();
                }
                // Protocol method → Witness dispatch
                if let Some(protocol) = self.find_protocol_for_method(func_entity) {
                    self.ctx.register_name(protocol);
                    let method_name = self.method_name_of(func_entity);
                    let self_type = if method_name == "init" {
                        result_ty.clone()
                    } else {
                        self.resolve_expr_type(callee_expr)
                    };
                    let callee = Callee::witness(protocol, &method_name, self_type, type_args);
                    return self.emit_call_maybe_init(callee, call_args, result_ty);
                }
                let callee = Callee::direct_generic(func_entity, type_args);
                self.emit_call_maybe_init(callee, call_args, result_ty)
            },
            // Overloaded function call: resolved by inference
            HirExpr::OverloadSet { candidates, type_args: explicit_hir_args, .. } => {
                let resolved = self.typed
                    .and_then(|t| t.resolutions.get(&callee_expr))
                    .copied()
                    .or_else(|| candidates.first().copied());
                if let Some(entity) = resolved {
                    let func_entity = self.resolve_callee_entity(entity, args);
                    self.ctx.register_name(func_entity);
                    let mut type_args = self.resolve_type_args(callee_expr);
                    let has_error = type_args.iter().any(|a| matches!(a, MirTy::Error));
                    if (type_args.is_empty() || has_error) && !explicit_hir_args.is_empty() {
                        type_args = explicit_hir_args
                            .iter()
                            .map(|hir_ty| lower_type(self.ctx, hir_ty))
                            .collect();
                    }
                    // Protocol method → Witness dispatch
                    if let Some(protocol) = self.find_protocol_for_method(func_entity) {
                        self.ctx.register_name(protocol);
                        let method_name = self.method_name_of(func_entity);
                        let self_type = if method_name == "init" {
                            result_ty.clone()
                        } else {
                            self.resolve_expr_type(callee_expr)
                        };
                        let callee = Callee::witness(protocol, &method_name, self_type, type_args);
                        return self.emit_call_maybe_init(callee, call_args, result_ty);
                    }
                    let callee = Callee::direct_generic(func_entity, type_args);
                    self.emit_call_maybe_init(callee, call_args, result_ty)
                } else {
                    Value::Immediate(Immediate::error())
                }
            },
            // Indirect call through a variable/expression
            _ => {
                let callee_ty = self.resolve_expr_type(callee_expr);
                let callee_val = self.lower_expr(callee_expr);
                match callee_val {
                    Value::Place(p) => {
                        // Dispatch thin vs thick based on the callee's function type
                        let callee = match &callee_ty {
                            MirTy::FuncThin { .. } => Callee::Thin(p),
                            _ => Callee::Thick(p),
                        };
                        self.emit_call(callee, call_args, result_ty)
                    },
                    Value::Immediate(Immediate { kind: ImmediateKind::FunctionRef { func, type_args }, .. }) => {
                        let func_entity = self.resolve_callee_entity(func, args);
                        let callee = Callee::direct_generic(func_entity, type_args);
                        self.emit_call_maybe_init(callee, call_args, result_ty)
                    },
                    _ => Value::Immediate(Immediate::error()),
                }
            },
        }
    }

    /// Lower a method call: `receiver.method(args)`
    fn lower_method_call(
        &mut self,
        expr_id: HirExprId,
        receiver_expr: HirExprId,
        method_name: &str,
        hir_type_args: Option<&[kestrel_hir::ty::HirTy]>,
        args: &[HirCallArg],
    ) -> Value {
        let mut receiver_ty = self.resolve_expr_type(receiver_expr);
        let result_ty = self.resolve_expr_type(expr_id);

        // If the receiver type resolves to a protocol entity (happens inside protocol
        // extensions where self is abstract), replace with SelfType so monomorphization
        // can substitute the concrete type
        if let MirTy::Named { entity, type_args } = &receiver_ty {
            if type_args.is_empty() {
                if self.ctx.world.get::<kestrel_ast_builder::NodeKind>(*entity)
                    == Some(&kestrel_ast_builder::NodeKind::Protocol)
                {
                    receiver_ty = MirTy::SelfType;
                }
            }
        }

        // Check for function-typed field calls BEFORE lowering receiver into call_args,
        // since field calls use the receiver differently (to access the field, not as self)
        if let Some(&resolved_entity) = self.typed.and_then(|t| t.resolutions.get(&expr_id)) {
            if self.ctx.world.get::<kestrel_ast_builder::NodeKind>(resolved_entity)
                == Some(&kestrel_ast_builder::NodeKind::Field)
            {
                let field_name = self.ctx.world.get::<kestrel_ast_builder::Name>(resolved_entity)
                    .map(|n| n.0.clone())
                    .unwrap_or_default();
                let receiver_val = self.lower_expr(receiver_expr);
                // Build field place from receiver
                let field_place = match receiver_val {
                    Value::Place(p) => p.field(field_name),
                    _ => {
                        let temp = self.fresh_temp(receiver_ty.clone());
                        self.emit_stmt(Statement::new(StatementKind::Assign {
                            dest: Place::local(temp),
                            rvalue: value_to_rvalue(receiver_val),
                        }));
                        Place::local(temp).field(field_name)
                    }
                };
                // Don't include receiver as arg — it's used to access the field
                let field_args = self.lower_call_args(args);
                // Function-typed fields are thick callables (closures)
                let callee = Callee::Thick(field_place);
                return self.emit_call(callee, field_args, result_ty);
            }

            // Subscript on a field: `self.data(index)` where `data` is a field with
            // a subscriptable type (e.g., Array[UInt8]). The inference resolves to the
            // subscript entity on the field's type, not a method on the receiver.
            // Decompose into field access + subscript call on the field's type.
            if self.ctx.world.get::<kestrel_ast_builder::NodeKind>(resolved_entity)
                == Some(&kestrel_ast_builder::NodeKind::Subscript)
            {
                let subscript_parent = self.ctx.world.parent_of(resolved_entity);
                let receiver_entity = match &receiver_ty {
                    MirTy::Named { entity, .. } => Some(*entity),
                    _ => None,
                };
                // Only decompose if subscript belongs to a different type than the receiver
                if subscript_parent != receiver_entity && subscript_parent.is_some() {
                    let field_ty = self.resolve_field_type(&receiver_ty, method_name);
                    let receiver_val = self.lower_expr(receiver_expr);
                    let field_place = match receiver_val {
                        Value::Place(p) => p.field(method_name.to_string()),
                        _ => {
                            let temp = self.fresh_temp(receiver_ty.clone());
                            self.emit_stmt(Statement::new(StatementKind::Assign {
                                dest: Place::local(temp),
                                rvalue: value_to_rvalue(receiver_val),
                            }));
                            Place::local(temp).field(method_name.to_string())
                        }
                    };
                    // Call the subscript with the field as receiver
                    let receiver_arg = if field_ty.is_trivially_copyable() {
                        CallArg::copy(Value::Place(field_place))
                    } else {
                        CallArg::borrow(Value::Place(field_place))
                    };
                    let mut call_args = vec![receiver_arg];
                    call_args.extend(self.lower_call_args(args));
                    self.ctx.register_name(resolved_entity);
                    let mut method_type_args = self.resolve_type_args(expr_id);
                    if method_type_args.iter().any(|a| matches!(a, MirTy::Error)) {
                        if let Some(hir_args) = hir_type_args {
                            if !hir_args.is_empty() {
                                method_type_args = hir_args
                                    .iter()
                                    .map(|ty| lower_type(self.ctx, ty))
                                    .collect();
                            }
                        }
                    }
                    let type_args = self.prepend_receiver_type_args(&field_ty, method_type_args);
                    let callee = Callee::method(resolved_entity, type_args, field_ty);
                    return self.emit_call(callee, call_args, result_ty);
                }
            }
        }

        // If the resolved method is `static`, it takes no receiver — don't lower or
        // prepend one. The receiver expression is just a type ref (e.g. `T` for a
        // type-param-rooted call like `T.create()`) with no side effects to evaluate.
        let resolved_entity = self.typed.and_then(|t| t.resolutions.get(&expr_id)).copied();
        let is_static = resolved_entity.map_or(false, |e| {
            self.ctx.world.get::<kestrel_ast_builder::Static>(e).is_some()
        });

        let call_args = if is_static {
            self.lower_call_args(args)
        } else {
            let receiver_val = self.lower_expr(receiver_expr);
            let receiver_arg = if receiver_ty.is_trivially_copyable() {
                CallArg::copy(receiver_val)
            } else {
                CallArg::borrow(receiver_val)
            };
            let mut a = vec![receiver_arg];
            a.extend(self.lower_call_args(args));
            a
        };

        // Type inference tells us which function entity this resolves to
        if let Some(resolved_entity) = resolved_entity {
            // Expand default arguments for missing params
            let explicit_count = args.len();
            let call_args = self.expand_default_args(call_args, resolved_entity, explicit_count);

            // Check if the method is from a protocol (needs Witness dispatch).
            if let Some(protocol) = self.find_protocol_for_method(resolved_entity) {
                self.ctx.register_name(protocol);
                let method_type_args = self.resolve_type_args(expr_id);
                let callee = Callee::witness(protocol, method_name, receiver_ty.clone(), method_type_args);
                return self.emit_call(callee, call_args, result_ty);
            }

            self.ctx.register_name(resolved_entity);
            let mut method_type_args = self.resolve_type_args(expr_id);
            // Fall back to explicit HIR type args when inference returns Error
            if method_type_args.iter().any(|a| matches!(a, MirTy::Error)) {
                if let Some(hir_args) = hir_type_args {
                    if !hir_args.is_empty() {
                        method_type_args = hir_args.iter().map(|ty| lower_type(self.ctx, ty)).collect();
                    }
                }
            }
            // Prepend receiver's struct type_args — inherited type_params come first
            let type_args = self.prepend_receiver_type_args(&receiver_ty, method_type_args);

            let callee = Callee::method(resolved_entity, type_args, receiver_ty);
            self.emit_call(callee, call_args, result_ty)
        } else {
            // Unresolved method — emit error
            Value::Immediate(Immediate::error())
        }
    }

    /// Lower a protocol call: `receiver.protocol.method(args)`
    /// These come from desugared operators and protocol method calls.
    fn lower_protocol_call(
        &mut self,
        expr_id: HirExprId,
        receiver_expr: HirExprId,
        protocol: Entity,
        method_name: &str,
        args: &[HirCallArg],
    ) -> Value {
        let receiver_ty = self.resolve_expr_type(receiver_expr);
        let receiver_val = self.lower_expr(receiver_expr);
        let result_ty = self.resolve_expr_type(expr_id);

        // Build args: receiver first (copy if trivially copyable), then explicit args
        let receiver_arg = if receiver_ty.is_trivially_copyable() {
            CallArg::copy(receiver_val)
        } else {
            CallArg::borrow(receiver_val)
        };
        let mut call_args = vec![receiver_arg];
        call_args.extend(self.lower_call_args(args));

        // Always use witness dispatch for protocol calls. The witness resolver
        // handles both concrete and generic receivers. Using Direct calls for
        // protocol methods is wrong — inference resolves to the abstract protocol
        // method entity (which has no body), not the concrete implementation.

        // Witness call — resolved at monomorphization time
        self.ctx.register_name(protocol);
        let method_type_args = self.resolve_type_args(expr_id);
        let callee = Callee::witness(protocol, method_name, receiver_ty, method_type_args);
        self.emit_call(callee, call_args, result_ty)
    }

    /// Check if an entity is an Initializer in the MIR and return its parent struct entity.
    fn is_init_function(&self, entity: Entity) -> Option<Entity> {
        let func = self.ctx.module.functions.iter().find(|f| f.entity == entity);
        if let Some(f) = func {
            match f.kind {
                FunctionKind::Initializer { parent } => Some(parent),
                _ => None,
            }
        } else {
            // Entity not yet in functions list — check ECS directly
            use kestrel_ast_builder::NodeKind;
            if self.ctx.world.get::<NodeKind>(entity) == Some(&NodeKind::Initializer) {
                self.ctx.world.parent_of(entity)
            } else {
                None
            }
        }
    }

    /// Emit a call, handling init calls by allocating self and prepending it as first arg.
    /// For init calls: allocates a temp of the struct type, passes &mut temp as self,
    /// calls the init, and returns the temp as the result.
    fn emit_call_maybe_init(
        &mut self,
        callee: Callee,
        mut call_args: Vec<CallArg>,
        result_ty: MirTy,
    ) -> Value {
        // Check if this is an init function (Direct or Witness)
        let is_init = match &callee {
            Callee::Direct { func, .. } => self.is_init_function(*func).is_some(),
            Callee::Witness { method, .. } => method == "init",
            _ => false,
        };

        if is_init {
            // Init call: allocate self, prepend as first arg, call, return self
            let self_local = self.fresh_temp(result_ty.clone());
            let self_ref = CallArg::mutating(Value::Place(Place::local(self_local)));
            call_args.insert(0, self_ref);

            // Ensure Direct init callees have self_type and struct type_args set.
            // Init functions inherit type_params from their parent struct, so the
            // struct's type_args must be prepended for correct mangling/substitution.
            let callee = match callee {
                Callee::Direct { func, type_args, self_type: None } => {
                    // Prepend struct type args, then append init's own type args.
                    // e.g., Array[Int64].init[I](from:) needs [Int64, Range[Int64]]
                    let type_args = self.prepend_receiver_type_args(&result_ty, type_args);
                    Callee::Direct { func, type_args, self_type: Some(result_ty.clone()) }
                }
                other => other,
            };

            // Init returns Unit — no dest needed
            self.emit_stmt(Statement::new(StatementKind::Call {
                dest: None,
                callee,
                args: call_args,
            }));
            Value::Place(Place::local(self_local))
        } else if let Callee::Direct { func, .. } = &callee {
            // Check if the callee is a struct entity (memberwise init with no explicit init)
            if self.is_struct_entity(*func) {
                // Memberwise construct: use actual field names from the struct
                let struct_def = self.ctx.module.structs.iter().find(|s| s.entity == *func);
                let fields: Vec<(String, Value)> = call_args.into_iter()
                    .enumerate()
                    .map(|(i, arg)| {
                        let name = struct_def
                            .and_then(|s| s.fields.get(i))
                            .map(|f| f.name.clone())
                            .unwrap_or_else(|| format!("_{i}"));
                        (name, arg.value)
                    })
                    .collect();
                let dest = self.fresh_temp(result_ty.clone());
                self.emit_stmt(Statement::new(StatementKind::Assign {
                    dest: Place::local(dest),
                    rvalue: Rvalue::Construct { ty: result_ty, fields },
                }));
                Value::Place(Place::local(dest))
            } else {
                self.emit_call(callee, call_args, result_ty)
            }
        } else {
            self.emit_call(callee, call_args, result_ty)
        }
    }

    /// Expand missing call arguments with default parameter values.
    /// For each missing param that has a default_entity, creates a synthetic thunk
    /// function, lowers it, and calls it to produce the default value.
    fn expand_default_args(
        &mut self,
        mut call_args: Vec<CallArg>,
        callee_entity: Entity,
        explicit_arg_count: usize,
    ) -> Vec<CallArg> {
        let Some(callable) = self.ctx.world.get::<kestrel_ast_builder::Callable>(callee_entity) else {
            return call_args;
        };
        if explicit_arg_count >= callable.params.len() {
            return call_args;
        }

        // Collect default entities for missing params (avoid borrow of callable across mut self)
        let defaults: Vec<(Entity, usize)> = callable.params[explicit_arg_count..]
            .iter()
            .enumerate()
            .filter_map(|(i, p)| p.default_entity.map(|e| (e, explicit_arg_count + i)))
            .collect();

        for (default_entity, _param_idx) in defaults {
            // Get the param type from the default entity's TypeAnnotation
            let param_ty = crate::ty::resolve_type_annotation(self.ctx, default_entity);

            let default_val = self.lower_default_arg(default_entity, param_ty.clone());
            let arg = if param_ty.is_trivially_copyable() {
                CallArg::copy(default_val)
            } else {
                CallArg::borrow(default_val)
            };
            call_args.push(arg);
        }

        call_args
    }

    /// Lower a default parameter expression by creating a synthetic thunk function
    /// and calling it. The thunk evaluates the default expression and returns its value.
    fn lower_default_arg(&mut self, default_entity: Entity, param_ty: MirTy) -> Value {
        // Create a synthetic thunk function for this default expression
        let thunk_entity = self.ctx.next_synthetic_entity();
        let parent_name = &self.ctx.module.functions[self.func_id.index()].name;
        let thunk_name = format!("{parent_name}.default_arg.{}", self.ctx.synthetic_entity_counter);
        self.ctx.module.register_name(thunk_entity, &thunk_name);

        let mut def = FunctionDef::new(thunk_entity, &thunk_name, param_ty.clone());
        def.kind = FunctionKind::Free;
        // Inherit caller's type_params so generic defaults work
        def.type_params = self.ctx.module.functions[self.func_id.index()]
            .type_params
            .clone();

        let thunk_func_id = self.ctx.module.add_function(def);

        // Lower the default expression's body into the thunk function.
        // This reborrows self.ctx — safe because lower_function_body creates
        // its own BodyLowerCtx with the default's HirBody/TypedBody.
        lower_function_body(self.ctx, default_entity, thunk_func_id);

        // Emit a call to the thunk at the current call site
        let type_args: Vec<MirTy> = self.ctx.module.functions[self.func_id.index()]
            .type_params
            .iter()
            .map(|tp| MirTy::TypeParam(tp.entity))
            .collect();
        let callee = Callee::direct_generic(thunk_entity, type_args);
        self.emit_call(callee, vec![], param_ty)
    }

    /// Emit a call statement and return the result value.
    fn emit_call(
        &mut self,
        callee: Callee,
        mut args: Vec<CallArg>,
        result_ty: MirTy,
    ) -> Value {
        // Override arg passing modes from the callee's `mutating`/`consuming`
        // param declarations. Only applies to Direct calls — witness/indirect
        // dispatch can't know the param modes at MIR-emission time.
        if let Callee::Direct { func, .. } = &callee {
            self.apply_callee_param_modes(&mut args, *func);
        }
        if result_ty == MirTy::Unit || result_ty == MirTy::Never {
            self.emit_stmt(Statement::new(StatementKind::Call {
                dest: None,
                callee,
                args,
            }));
            Value::Immediate(Immediate::unit())
        } else {
            let dest = self.fresh_temp(result_ty);
            self.emit_stmt(Statement::new(StatementKind::Call {
                dest: Some(Place::local(dest)),
                callee,
                args,
            }));
            Value::Place(Place::local(dest))
        }
    }


    /// Lower a literal value to an immediate.
    /// Lower a literal expression. If the target type is a Named struct (e.g., Bool, Int64),
    /// emit an init call to the struct's literal protocol init (boolLiteral:, intLiteral:, etc.)
    /// so the primitive gets properly wrapped in the struct. If the target type is a MIR
    /// primitive (inside init bodies), return the bare immediate.
    fn lower_literal_expr(&mut self, expr_id: HirExprId, lit: &HirLiteral) -> Value {
        let result_ty = self.resolve_expr_type(expr_id);

        // If the type is a Named struct, wrap the primitive via init call.
        // e.g. `42` with result type Int64 → Int64(intLiteral: 42)
        if let MirTy::Named { entity, .. } = &result_ty {
            let label = match lit {
                HirLiteral::Bool(_) => "boolLiteral",
                HirLiteral::Integer(_) => "intLiteral",
                HirLiteral::Float(_) => "floatLiteral",
                HirLiteral::Char(_) => "charLiteral",
                HirLiteral::String(s) => {
                    // String literals need a 2-arg init: init(stringLiteral: ptr, length: i64)
                    if let Some(init_entity) = self.find_string_literal_init(*entity) {
                        let content = decode_string_literal(s);
                        let ptr_val = Value::Immediate(Immediate::string_ptr(content.to_string()));
                        let len_val = Value::Immediate(Immediate::i64(content.len() as i64));
                        self.ctx.register_name(init_entity);
                        let call_args = vec![
                            CallArg::copy(ptr_val),
                            CallArg::copy(len_val),
                        ];
                        let callee = Callee::method(init_entity, vec![], result_ty.clone());
                        return self.emit_call_maybe_init(callee, call_args, result_ty);
                    }
                    return self.lower_literal_primitive(lit, &result_ty);
                }
                HirLiteral::Null => {
                    return self.lower_literal_primitive(lit, &result_ty);
                }
            };

            if let Some(init_entity) = self.find_literal_init(*entity, label) {
                // Use the init's parameter type for the primitive width.
                // e.g. Float32.init(floatLiteral: lang.f64) → f64 literal
                let param_ty = self.resolve_init_param_type(init_entity)
                    .unwrap_or_else(|| result_ty.clone());
                let primitive = self.lower_literal_primitive(lit, &param_ty);
                self.ctx.register_name(init_entity);
                let call_args = vec![CallArg::copy(primitive)];
                // Set self_type to the target struct so monomorphization
                // mangles correctly (not the caller's self_type)
                let callee = Callee::method(init_entity, vec![], result_ty.clone());
                return self.emit_call_maybe_init(callee, call_args, result_ty);
            }
        }

        // Primitive type or no init found — type inference already resolved
        // the correct width (e.g. F32 for a literal in lang.f32 context)
        self.lower_literal_primitive(lit, &result_ty)
    }

    /// Lower a literal to its primitive MIR immediate, using `target_ty`
    /// (from type inference) to select the correct bit width.
    fn lower_literal_primitive(&self, lit: &HirLiteral, target_ty: &MirTy) -> Value {
        match lit {
            HirLiteral::Integer(v) => match target_ty {
                MirTy::I8 => Value::Immediate(Immediate::i8(*v as i8)),
                MirTy::I16 => Value::Immediate(Immediate::i16(*v as i16)),
                MirTy::I32 => Value::Immediate(Immediate::i32(*v as i32)),
                _ => Value::Immediate(Immediate::i64(*v)),
            },
            HirLiteral::Float(v) => match target_ty {
                MirTy::F32 => Value::Immediate(Immediate::f32(*v as f32)),
                _ => Value::Immediate(Immediate::f64(*v)),
            },
            HirLiteral::Bool(v) => Value::Immediate(Immediate::bool(*v)),
            HirLiteral::String(s) => Value::Immediate(Immediate::string(decode_string_literal(s))),
            HirLiteral::Char(c) => Value::Immediate(Immediate::i32(*c as i32)),
            HirLiteral::Null => Value::Immediate(Immediate::unit()),
        }
    }

    /// Lower an array literal via the target type's internal array-literal initializer
    /// when the contextual result type is not a raw `Array[T]`.
    fn lower_array_literal_via_init(
        &mut self,
        elements: &[HirExprId],
        result_ty: &MirTy,
    ) -> Option<Value> {
        let (init_entity, element_ty, type_args) = self.resolve_array_literal_init(result_ty)?;

        let ptr_ty = MirTy::Pointer(Box::new(element_ty.clone()));
        let ptr_local = self.fresh_temp(ptr_ty);
        let ptr_place = Place::local(ptr_local);
        let count_value = Value::Immediate(Immediate::i64(elements.len() as i64));

        self.emit_stmt(Statement::new(StatementKind::Assign {
            dest: ptr_place.clone(),
            rvalue: Rvalue::Op1 {
                op: Op::StackAlloc(element_ty.clone()),
                arg: count_value.clone(),
            },
        }));

        let size_local = self.fresh_temp(MirTy::I64);
        self.emit_stmt(Statement::new(StatementKind::Assign {
            dest: Place::local(size_local),
            rvalue: Rvalue::Op1 {
                op: Op::SizeOf(element_ty.clone()),
                arg: Value::Immediate(Immediate::unit()),
            },
        }));

        for (i, &element_expr) in elements.iter().enumerate() {
            let element_value = self.lower_expr_with_hint(element_expr, &element_ty);
            let element_ptr = if i == 0 {
                Value::Place(ptr_place.clone())
            } else {
                let offset_local = self.fresh_temp(MirTy::I64);
                self.emit_stmt(Statement::new(StatementKind::Assign {
                    dest: Place::local(offset_local),
                    rvalue: Rvalue::Op2 {
                        op: Op::Mul(IntBits::I64, Signedness::Signed),
                        lhs: Value::Immediate(Immediate::i64(i as i64)),
                        rhs: Value::Place(Place::local(size_local)),
                    },
                }));

                let offset_ptr_local =
                    self.fresh_temp(MirTy::Pointer(Box::new(element_ty.clone())));
                self.emit_stmt(Statement::new(StatementKind::Assign {
                    dest: Place::local(offset_ptr_local),
                    rvalue: Rvalue::Op2 {
                        op: Op::PtrOffset,
                        lhs: Value::Place(ptr_place.clone()),
                        rhs: Value::Place(Place::local(offset_local)),
                    },
                }));
                Value::Place(Place::local(offset_ptr_local))
            };

            let write_local = self.fresh_temp(MirTy::Unit);
            self.emit_stmt(Statement::new(StatementKind::Assign {
                dest: Place::local(write_local),
                rvalue: Rvalue::Op2 {
                    op: Op::PtrWrite(element_ty.clone()),
                    lhs: element_ptr,
                    rhs: element_value,
                },
            }));
        }

        self.ctx.register_name(init_entity);
        let callee = Callee::method(init_entity, type_args, result_ty.clone());
        let call_args = vec![CallArg::copy(Value::Place(ptr_place)), CallArg::copy(count_value)];
        Some(self.emit_call_maybe_init(callee, call_args, result_ty.clone()))
    }

    /// Lower an expression with a contextual type hint for cases the type checker
    /// leaves as `Error`, such as implicit enum members inside array literals.
    fn lower_expr_with_hint(&mut self, expr_id: HirExprId, expected_ty: &MirTy) -> Value {
        if !matches!(self.resolve_expr_type(expr_id), MirTy::Error) {
            return self.lower_expr(expr_id);
        }

        match &self.hir.exprs[expr_id] {
            HirExpr::ImplicitMember { name, args, .. } => {
                let payload: Vec<Value> = args
                    .as_ref()
                    .map(|a| a.iter().map(|arg| self.lower_expr(arg.value)).collect())
                    .unwrap_or_default();
                let dest = self.fresh_temp(expected_ty.clone());
                self.emit_stmt(Statement::new(StatementKind::Assign {
                    dest: Place::local(dest),
                    rvalue: Rvalue::EnumVariant {
                        enum_ty: expected_ty.clone(),
                        variant: name.clone(),
                        payload,
                    },
                }));
                Value::Place(Place::local(dest))
            }
            HirExpr::Def(entity, _, _) => {
                if self.ctx.world.get::<kestrel_ast_builder::NodeKind>(*entity)
                    == Some(&kestrel_ast_builder::NodeKind::EnumCase)
                {
                    let variant = self.ctx.world
                        .get::<kestrel_ast_builder::Name>(*entity)
                        .map(|n| n.0.clone())
                        .unwrap_or_default();
                    let dest = self.fresh_temp(expected_ty.clone());
                    self.emit_stmt(Statement::new(StatementKind::Assign {
                        dest: Place::local(dest),
                        rvalue: Rvalue::EnumVariant {
                            enum_ty: expected_ty.clone(),
                            variant,
                            payload: vec![],
                        },
                    }));
                    Value::Place(Place::local(dest))
                } else {
                    self.lower_expr(expr_id)
                }
            }
            _ => self.lower_expr(expr_id),
        }
    }

    /// Get the type of the first non-self parameter of an init function.
    fn resolve_init_param_type(&mut self, init_entity: Entity) -> Option<MirTy> {
        use crate::ty::resolve_callable_types;
        let types = resolve_callable_types(self.ctx, init_entity);
        // First param type (init literals have exactly one param)
        types.into_iter().next().flatten()
    }

    /// Find a literal protocol init (e.g., init(boolLiteral:)) on a struct entity.
    fn find_literal_init(&self, struct_entity: Entity, label: &str) -> Option<Entity> {
        use kestrel_ast_builder::{Callable, NodeKind};

        for &child in self.ctx.world.children_of(struct_entity) {
            let Some(kind) = self.ctx.world.get::<NodeKind>(child) else { continue };
            if *kind != NodeKind::Initializer { continue }
            let Some(callable) = self.ctx.world.get::<Callable>(child) else { continue };
            // Match init with exactly 1 param whose label matches
            if callable.params.len() == 1 {
                if callable.params[0].label.as_deref() == Some(label) {
                    return Some(child);
                }
            }
        }
        None
    }

    /// Find the string literal init: init(stringLiteral: ptr, length: i64).
    /// This is a 2-param init unlike other literal inits.
    fn find_string_literal_init(&self, struct_entity: Entity) -> Option<Entity> {
        use kestrel_ast_builder::{Callable, NodeKind};

        // Search direct children first
        for &child in self.ctx.world.children_of(struct_entity) {
            let Some(kind) = self.ctx.world.get::<NodeKind>(child) else { continue };
            if *kind != NodeKind::Initializer { continue }
            let Some(callable) = self.ctx.world.get::<Callable>(child) else { continue };
            // Second param has no external label (single-name param: "length: lang.i64")
            if callable.params.len() == 2
                && callable.params[0].label.as_deref() == Some("stringLiteral")
            {
                return Some(child);
            }
        }
        None
    }

    /// Resolve the internal array-literal initializer and its concrete element type.
    fn resolve_array_literal_init(&self, result_ty: &MirTy) -> Option<(Entity, MirTy, Vec<MirTy>)> {
        let MirTy::Named { entity, .. } = result_ty else {
            return None;
        };

        let init_func = self.ctx.module.functions.iter().find(|f| {
            matches!(f.kind, FunctionKind::Initializer { parent } if parent == *entity)
                && f.params.len() == 3
                && matches!(f.params[0].ty, MirTy::RefMut(_))
                && matches!(f.params[1].ty, MirTy::Pointer(_))
                && matches!(f.params[2].ty, MirTy::I64)
        })?;
        let type_args = self.prepend_receiver_type_args(result_ty, vec![]);
        let subst: HashMap<Entity, MirTy> = init_func
            .type_params
            .iter()
            .zip(type_args.iter())
            .map(|(tp, ty)| (tp.entity, ty.clone()))
            .collect();
        let ptr_ty = self.substitute_mir_type(&init_func.params.get(1)?.ty, &subst);
        let MirTy::Pointer(element_ty) = ptr_ty else {
            return None;
        };

        Some((init_func.entity, *element_ty, type_args))
    }

    /// Lower an if expression.
    /// Creates: current_block → branch → then_block / else_block → merge_block.
    /// Both branches assign their result to a shared temp before jumping to merge.
    fn lower_if(
        &mut self,
        expr_id: HirExprId,
        condition: HirExprId,
        then_body: &HirBlock,
        else_body: Option<&HirBlock>,
    ) -> Value {
        let cond_val = self.lower_expr(condition);
        let result_ty = self.resolve_expr_type(expr_id);

        let then_block = self.new_block();
        let else_block = self.new_block();
        let merge_block = self.new_block();

        // Result temp — both branches assign to this before jumping to merge
        let result_local = self.fresh_temp(result_ty);

        // Branch on condition
        self.set_terminator(Terminator::branch(cond_val, then_block, else_block));

        // Lower then branch
        self.switch_to_block(then_block);
        let then_val = self.lower_hir_block(then_body);
        if !self.is_terminated() {
            self.emit_stmt(Statement::new(StatementKind::Assign {
                dest: Place::local(result_local),
                rvalue: value_to_rvalue(then_val),
            }));
            self.set_terminator(Terminator::jump(merge_block));
        }

        // Lower else branch
        self.switch_to_block(else_block);
        if let Some(else_body) = else_body {
            let else_val = self.lower_hir_block(else_body);
            if !self.is_terminated() {
                self.emit_stmt(Statement::new(StatementKind::Assign {
                    dest: Place::local(result_local),
                    rvalue: value_to_rvalue(else_val),
                }));
                self.set_terminator(Terminator::jump(merge_block));
            }
        } else {
            // No else branch — result is unit
            self.emit_stmt(Statement::new(StatementKind::Assign {
                dest: Place::local(result_local),
                rvalue: Rvalue::Const(Immediate::unit()),
            }));
            self.set_terminator(Terminator::jump(merge_block));
        }

        // Continue in merge block
        self.switch_to_block(merge_block);
        Value::Place(Place::local(result_local))
    }

    /// Lower a loop expression.
    /// Creates: header_block (loop body) → jump header; exit_block for break.
    fn lower_loop(&mut self, body: &HirBlock, label: Option<&str>) -> Value {
        let header_block = self.new_block();
        let exit_block = self.new_block();

        // Jump into the loop header
        if !self.is_terminated() {
            self.set_terminator(Terminator::jump(header_block));
        }

        // Push loop info for break/continue
        self.loop_stack.push(LoopInfo {
            header_block,
            exit_block,
            label: label.map(|s| s.to_string()),
        });

        // Lower loop body
        self.switch_to_block(header_block);
        let _ = self.lower_hir_block(body);

        // At end of body, loop back to header
        if !self.is_terminated() {
            self.set_terminator(Terminator::jump(header_block));
        }

        // Pop loop info
        self.loop_stack.pop();

        // Continue after the loop
        self.switch_to_block(exit_block);
        Value::Immediate(Immediate::unit())
    }

    /// Lower a break expression — jump to the loop's exit block.
    fn lower_break(&mut self, label: Option<&str>) -> Value {
        let exit_block = self.find_loop(label).map(|l| l.exit_block);
        if let Some(exit) = exit_block {
            self.set_terminator(Terminator::jump(exit));
        }
        Value::Immediate(Immediate::unit())
    }

    /// Lower a continue expression — jump to the loop's header block.
    fn lower_continue(&mut self, label: Option<&str>) -> Value {
        let header_block = self.find_loop(label).map(|l| l.header_block);
        if let Some(header) = header_block {
            self.set_terminator(Terminator::jump(header));
        }
        Value::Immediate(Immediate::unit())
    }

    /// Find a loop by label (or the innermost loop if no label).
    fn find_loop(&self, label: Option<&str>) -> Option<&LoopInfo> {
        match label {
            Some(label) => self.loop_stack.iter().rev().find(|l| {
                l.label.as_deref() == Some(label)
            }),
            None => self.loop_stack.last(),
        }
    }

    // === Closure lowering ===

    /// Lower a closure expression into a synthetic function + ApplyPartial.
    ///
    /// Strategy:
    /// 1. Identify captures (locals from parent scope referenced in closure body)
    /// 2. Create env struct for captures (if any)
    /// 3. Create a synthetic call function with env loads at the top
    /// 4. Register ClosureInfo for codegen
    /// 5. Emit ApplyPartial to create the thick callable value
    fn lower_closure(
        &mut self,
        expr_id: HirExprId,
        params: &[HirClosureParam],
        body: &HirBlock,
    ) -> Value {
        // Get the closure's function type from inference
        let closure_ty = self.resolve_expr_type(expr_id);

        // Identify captured locals: locals referenced in the body that aren't
        // closure params and come from the parent function
        let closure_param_locals: std::collections::HashSet<HirLocalId> =
            params.iter().map(|p| p.local).collect();
        let captured_locals = self.find_captures(body, &closure_param_locals);

        // Generate unique closure name using global counter to avoid collisions
        let closure_idx = self.ctx.closure_counter;
        self.ctx.closure_counter += 1;
        let parent_name = &self.ctx.module.functions[self.func_id.index()].name;
        let closure_name = format!("{}.closure.{}", parent_name, closure_idx);

        // Determine param and return types from the closure's function type
        let (param_tys, ret_ty) = match &closure_ty {
            MirTy::FuncThick { params, ret } => (params.clone(), *ret.clone()),
            _ => {
                // Fallback: infer from params
                let p: Vec<MirTy> = params
                    .iter()
                    .map(|p| self.resolve_local_type(p.local))
                    .collect();
                (p, MirTy::Unit)
            },
        };

        // Create env struct for captures (if any)
        let env_struct_id = if !captured_locals.is_empty() {
            let env_struct_name = format!("{}.env", closure_name);
            let env_struct_entity = self.ctx.next_synthetic_entity();
            self.ctx
                .module
                .register_name(env_struct_entity, &env_struct_name);

            let mut env_def = StructDef::new(env_struct_entity, &env_struct_name);
            // Inherit parent's type_params so struct_layout can substitute TypeParam field types
            env_def.type_params = self.ctx.module.functions[self.func_id.index()]
                .type_params
                .iter()
                .map(|tp| kestrel_mir::TypeParamDef::new(tp.entity, &tp.name))
                .collect();
            for &captured in &captured_locals {
                let cap_ty = self.resolve_local_type(captured);
                let cap_name = self.hir.locals[captured].name.clone();
                env_def.add_field(FieldDef::new(&cap_name, cap_ty));
            }
            Some(self.ctx.module.add_struct(env_def))
        } else {
            None
        };

        // Create the synthetic call function
        let closure_entity = self.ctx.next_synthetic_entity();
        self.ctx.module.register_name(closure_entity, &closure_name);

        let mut func_def = FunctionDef::new(closure_entity, &closure_name, ret_ty.clone());
        // Inherit parent's type_params so monomorphization propagates concrete type_args
        func_def.type_params = self.ctx.module.functions[self.func_id.index()]
            .type_params
            .clone();
        func_def.kind = if let Some(env_id) = env_struct_id {
            FunctionKind::ClosureCall { env_struct: env_id }
        } else {
            FunctionKind::Closure
        };

        // === Build closure body by swapping BodyLowerCtx state ===

        // Create the closure's MIR body
        let mut closure_body = MirBody::new();

        // Add env parameter — typed to the actual env struct pointer if capturing
        let env_ty = if let Some(env_id) = env_struct_id {
            let env_entity = self.ctx.module.structs[env_id.index()].entity;
            // Build type_args matching the parent's type_params so substitute_type
            // propagates concrete types through the env struct pointer
            let env_type_args: Vec<MirTy> = self.ctx.module.functions[self.func_id.index()]
                .type_params
                .iter()
                .map(|tp| MirTy::TypeParam(tp.entity))
                .collect();
            MirTy::Pointer(Box::new(MirTy::Named {
                entity: env_entity,
                type_args: env_type_args,
            }))
        } else {
            MirTy::Pointer(Box::new(MirTy::Unit))
        };
        let env_local = closure_body.add_local(LocalDef::new("env", env_ty.clone()));
        let env_param = ParamDef::new("env", env_local, env_ty);
        func_def.params.push(env_param);
        closure_body.param_count += 1;

        // Add closure params and build local map for closure context
        let mut closure_local_map = HashMap::new();
        for (i, cp) in params.iter().enumerate() {
            let ty = param_tys.get(i).cloned().unwrap_or(MirTy::Error);
            let local = closure_body.add_local(LocalDef::new(
                &self.hir.locals[cp.local].name,
                ty.clone(),
            ));
            let param = ParamDef::new(&self.hir.locals[cp.local].name, local, ty);
            func_def.params.push(param);
            closure_body.param_count += 1;
            closure_local_map.insert(cp.local, local);
        }

        // Create locals for captures (will be loaded from env struct in entry block)
        let mut capture_local_ids = Vec::new();
        for &captured in &captured_locals {
            let cap_ty = self.resolve_local_type(captured);
            let cap_name = self.hir.locals[captured].name.clone();
            let closure_local = closure_body.add_local(LocalDef::new(&cap_name, cap_ty));
            closure_local_map.insert(captured, closure_local);
            capture_local_ids.push(closure_local);
        }

        // Create entry block
        let entry_block = closure_body.add_block(BasicBlock::new());
        closure_body.entry = entry_block;

        // Save parent state
        let saved_body = std::mem::replace(&mut self.body, closure_body);
        let saved_block = self.current_block.take();
        let saved_local_map = std::mem::replace(&mut self.local_map, closure_local_map);
        let saved_loop_stack = std::mem::take(&mut self.loop_stack);
        let saved_func_id = self.func_id;
        let saved_temp_counter = self.temp_counter;

        // Switch to closure context
        self.current_block = Some(entry_block);

        // Emit loads from env struct for captured locals
        if env_struct_id.is_some() {
            for (i, &captured) in captured_locals.iter().enumerate() {
                let closure_local = capture_local_ids[i];
                let cap_name = self.hir.locals[captured].name.clone();
                // Deref the env pointer and access the field by name
                let field_place = Place::local(env_local).deref().field(&cap_name);
                self.emit_stmt(Statement::new(StatementKind::Assign {
                    dest: Place::local(closure_local),
                    rvalue: Rvalue::Copy(field_place),
                }));
            }
        }

        // Lower the closure body
        let body_val = self.lower_hir_block(body);

        // Add return terminator if not already terminated
        if !self.is_terminated() {
            self.set_terminator(Terminator::ret(body_val));
        }

        // Extract completed closure body and restore parent state
        let completed_closure_body = std::mem::replace(&mut self.body, saved_body);
        self.current_block = saved_block;
        self.local_map = saved_local_map;
        self.loop_stack = saved_loop_stack;
        self.func_id = saved_func_id;
        self.temp_counter = saved_temp_counter;

        // Attach body to closure function and add to module
        func_def.body = Some(completed_closure_body);
        let closure_func_id = self.ctx.module.add_function(func_def);

        // Register ClosureInfo for codegen
        if let Some(env_id) = env_struct_id {
            let captures: Vec<CaptureInfo> = captured_locals
                .iter()
                .map(|&hir_local| {
                    let cap_ty = self.resolve_local_type(hir_local);
                    let cap_name = self.hir.locals[hir_local].name.clone();
                    CaptureInfo::new(cap_name, cap_ty, CaptureMode::ByRef)
                })
                .collect();

            self.ctx.module.add_closure(ClosureInfo {
                env_struct: env_id,
                call_function: closure_func_id,
                captures,
            });
        }

        // Collect capture values from the parent scope
        let capture_values: Vec<Value> = captured_locals
            .iter()
            .map(|&hir_local| {
                let mir_local = self.map_local(hir_local);
                Value::Place(Place::local(mir_local))
            })
            .collect();

        // Emit ApplyPartial to create the thick callable
        let result_ty = closure_ty;
        let dest = self.fresh_temp(result_ty);
        self.emit_stmt(Statement::new(StatementKind::Assign {
            dest: Place::local(dest),
            rvalue: Rvalue::ApplyPartial {
                func: closure_entity,
                captures: capture_values,
            },
        }));

        Value::Place(Place::local(dest))
    }

    /// Find locals from the parent scope that are referenced in a closure body.
    fn find_captures(
        &self,
        body: &HirBlock,
        closure_params: &std::collections::HashSet<HirLocalId>,
    ) -> Vec<HirLocalId> {
        let mut captures = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Walk all expressions in the body looking for Local references
        self.collect_captures_block(body, closure_params, &mut captures, &mut seen);
        captures
    }

    /// Recursively collect local references in a block.
    fn collect_captures_block(
        &self,
        block: &HirBlock,
        closure_params: &std::collections::HashSet<HirLocalId>,
        captures: &mut Vec<HirLocalId>,
        seen: &mut std::collections::HashSet<HirLocalId>,
    ) {
        for &stmt_id in &block.stmts {
            self.collect_captures_stmt(stmt_id, closure_params, captures, seen);
        }
        if let Some(tail) = block.tail_expr {
            self.collect_captures_expr(tail, closure_params, captures, seen);
        }
    }

    fn collect_captures_stmt(
        &self,
        stmt_id: HirStmtId,
        closure_params: &std::collections::HashSet<HirLocalId>,
        captures: &mut Vec<HirLocalId>,
        seen: &mut std::collections::HashSet<HirLocalId>,
    ) {
        let stmt = &self.hir.stmts[stmt_id];
        match stmt {
            HirStmt::Let { value, .. } => {
                if let Some(expr) = value {
                    self.collect_captures_expr(*expr, closure_params, captures, seen);
                }
            },
            HirStmt::Expr { expr, .. } => {
                self.collect_captures_expr(*expr, closure_params, captures, seen);
            },
            HirStmt::Deinit { .. } => {},
        }
    }

    fn collect_captures_expr(
        &self,
        expr_id: HirExprId,
        closure_params: &std::collections::HashSet<HirLocalId>,
        captures: &mut Vec<HirLocalId>,
        seen: &mut std::collections::HashSet<HirLocalId>,
    ) {
        let expr = &self.hir.exprs[expr_id];
        match expr {
            HirExpr::Local(local_id, _) => {
                // It's a capture if it's not a closure param and we've mapped it
                // (meaning it's from the parent scope)
                if !closure_params.contains(local_id)
                    && self.local_map.contains_key(local_id)
                    && seen.insert(*local_id)
                {
                    captures.push(*local_id);
                }
            },
            // Recurse into sub-expressions
            HirExpr::Call { callee, args, .. } => {
                self.collect_captures_expr(*callee, closure_params, captures, seen);
                for arg in args {
                    self.collect_captures_expr(arg.value, closure_params, captures, seen);
                }
            },
            HirExpr::MethodCall { receiver, args, .. } => {
                self.collect_captures_expr(*receiver, closure_params, captures, seen);
                for arg in args {
                    self.collect_captures_expr(arg.value, closure_params, captures, seen);
                }
            },
            HirExpr::ProtocolCall { receiver, args, .. } => {
                self.collect_captures_expr(*receiver, closure_params, captures, seen);
                for arg in args {
                    self.collect_captures_expr(arg.value, closure_params, captures, seen);
                }
            },
            HirExpr::If { condition, then_body, else_body, .. } => {
                self.collect_captures_expr(*condition, closure_params, captures, seen);
                self.collect_captures_block(then_body, closure_params, captures, seen);
                if let Some(else_b) = else_body {
                    self.collect_captures_block(else_b, closure_params, captures, seen);
                }
            },
            HirExpr::Loop { body, .. } => {
                self.collect_captures_block(body, closure_params, captures, seen);
            },
            HirExpr::Block { body, .. } => {
                self.collect_captures_block(body, closure_params, captures, seen);
            },
            HirExpr::Match { scrutinee, arms, .. } => {
                self.collect_captures_expr(*scrutinee, closure_params, captures, seen);
                for arm in arms {
                    self.collect_captures_expr(arm.body, closure_params, captures, seen);
                    if let Some(guard) = arm.guard {
                        self.collect_captures_expr(guard, closure_params, captures, seen);
                    }
                }
            },
            HirExpr::Tuple { elements, .. } | HirExpr::Array { elements, .. } => {
                for &elem in elements {
                    self.collect_captures_expr(elem, closure_params, captures, seen);
                }
            },
            HirExpr::Field { base, .. } | HirExpr::TupleIndex { base, .. } => {
                self.collect_captures_expr(*base, closure_params, captures, seen);
            },
            HirExpr::Assign { target, value, .. } => {
                self.collect_captures_expr(*target, closure_params, captures, seen);
                self.collect_captures_expr(*value, closure_params, captures, seen);
            },
            HirExpr::Return { value, .. } => {
                if let Some(v) = value {
                    self.collect_captures_expr(*v, closure_params, captures, seen);
                }
            },
            HirExpr::Closure { body, .. } => {
                self.collect_captures_block(body, closure_params, captures, seen);
            },
            HirExpr::Dict { entries, .. } => {
                for entry in entries {
                    self.collect_captures_expr(entry.key, closure_params, captures, seen);
                    self.collect_captures_expr(entry.value, closure_params, captures, seen);
                }
            },
            // No sub-expressions to recurse into
            HirExpr::Literal { .. }
            | HirExpr::Def(..)
            | HirExpr::OverloadSet { .. }
            | HirExpr::ImplicitMember { .. }
            | HirExpr::Break { .. }
            | HirExpr::Continue { .. }
            | HirExpr::Error { .. } => {},
        }
    }

    // === Match lowering ===

    /// Lower a match expression using the pattern-matching decision tree compiler.
    fn lower_match(
        &mut self,
        expr_id: HirExprId,
        scrutinee_expr: HirExprId,
        arms: &[HirMatchArm],
    ) -> Value {
        let result_ty = self.resolve_expr_type(expr_id);
        let scrutinee_ty = self.resolve_expr_resolved_ty(scrutinee_expr);

        // Lower scrutinee to a place (materialize immediates into a temp)
        let scrutinee_val = self.lower_expr(scrutinee_expr);
        let scrutinee_place = match scrutinee_val {
            Value::Place(p) => p,
            Value::Immediate(imm) => {
                let s_ty = self.resolve_expr_type(scrutinee_expr);
                let temp = self.fresh_temp(s_ty);
                self.emit_stmt(Statement::new(StatementKind::Assign {
                    dest: Place::local(temp),
                    rvalue: Rvalue::Const(imm),
                }));
                Place::local(temp)
            },
        };

        // Create result temp (where arm bodies will store their values)
        let result_local = self.fresh_temp(result_ty);
        let join_block = self.new_block();

        // Compile patterns into a decision tree
        let tree = kestrel_pattern_matching::compile_decision_tree(
            self.hir,
            &self.ctx.query,
            self.ctx.root,
            &scrutinee_ty,
            arms,
        );

        // Emit the decision tree as MIR control flow
        self.emit_decision_tree(&tree, &scrutinee_place, arms, result_local, join_block);

        // Continue from the join block
        self.switch_to_block(join_block);
        Value::Place(Place::local(result_local))
    }

    /// Recursively emit a decision tree as MIR basic blocks.
    fn emit_decision_tree(
        &mut self,
        tree: &DecisionTree,
        scrutinee: &Place,
        arms: &[HirMatchArm],
        result_local: LocalId,
        join_block: BlockId,
    ) {
        match tree {
            DecisionTree::Switch {
                path,
                ty: _,
                cases,
                default,
            } => {
                // Build the place to test by applying the access path
                let test_place = apply_access_path(scrutinee.clone(), path);

                if cases.len() == 1 && default.is_none() {
                    // Single case — no need for a switch, just emit directly
                    // (Bind any downcast and recurse)
                    let (_, subtree) = &cases[0];
                    self.emit_decision_tree(subtree, scrutinee, arms, result_local, join_block);
                    return;
                }

                // Check if this is a simple boolean branch
                if cases.len() == 2
                    && matches!(&cases[0].0, Constructor::True)
                    && matches!(&cases[1].0, Constructor::False)
                {
                    let true_block = self.new_block();
                    let false_block = self.new_block();
                    self.set_terminator(Terminator::branch(
                        Value::Place(test_place),
                        true_block,
                        false_block,
                    ));

                    self.switch_to_block(true_block);
                    self.emit_decision_tree(
                        &cases[0].1,
                        scrutinee,
                        arms,
                        result_local,
                        join_block,
                    );

                    self.switch_to_block(false_block);
                    self.emit_decision_tree(
                        &cases[1].1,
                        scrutinee,
                        arms,
                        result_local,
                        join_block,
                    );
                    return;
                }

                // General switch: create a block for each case + optional default
                let mut case_blocks: Vec<(kestrel_mir::SwitchCase, BlockId)> =
                    Vec::with_capacity(cases.len());
                for (ctor, _) in cases.iter() {
                    let case = constructor_to_switch_case(ctor, self.ctx);
                    let block = self.new_block();
                    case_blocks.push((case, block));
                }

                let default_block = if default.is_some() {
                    Some(self.new_block())
                } else {
                    None
                };

                // Build switch cases — include default as the last case if present
                let mut switch_cases = case_blocks.clone();
                if let Some(def_block) = default_block {
                    switch_cases.push((kestrel_mir::SwitchCase::Wildcard, def_block));
                }

                self.set_terminator(Terminator::switch(test_place, switch_cases));

                // Emit each case's subtree
                for ((_, subtree), (_, block_id)) in cases.iter().zip(case_blocks.iter()) {
                    self.switch_to_block(*block_id);
                    self.emit_decision_tree(subtree, scrutinee, arms, result_local, join_block);
                }

                // Emit default
                if let (Some(def_tree), Some(def_block)) = (default, default_block) {
                    self.switch_to_block(def_block);
                    self.emit_decision_tree(def_tree, scrutinee, arms, result_local, join_block);
                }
            },

            DecisionTree::Success {
                arm_index,
                bindings,
            } => {
                // Bind pattern variables from the scrutinee
                self.emit_bindings(bindings, scrutinee);

                // Lower the arm body
                if let Some(arm) = arms.get(*arm_index) {
                    let body_val = self.lower_expr(arm.body);
                    if !self.is_terminated() {
                        // Store result and jump to join
                        self.emit_stmt(Statement::new(StatementKind::Assign {
                            dest: Place::local(result_local),
                            rvalue: value_to_rvalue(body_val),
                        }));
                        self.set_terminator(Terminator::jump(join_block));
                    }
                }
            },

            DecisionTree::Guard {
                arm_index,
                bindings,
                success,
                failure,
            } => {
                // Bind variables first (needed for guard evaluation)
                self.emit_bindings(bindings, scrutinee);

                // Lower the guard condition
                if let Some(arm) = arms.get(*arm_index) {
                    if let Some(guard_expr) = arm.guard {
                        let guard_val = self.lower_expr(guard_expr);
                        let success_block = self.new_block();
                        let failure_block = self.new_block();
                        self.set_terminator(Terminator::branch(
                            guard_val,
                            success_block,
                            failure_block,
                        ));

                        self.switch_to_block(success_block);
                        self.emit_decision_tree(
                            success,
                            scrutinee,
                            arms,
                            result_local,
                            join_block,
                        );

                        self.switch_to_block(failure_block);
                        self.emit_decision_tree(
                            failure,
                            scrutinee,
                            arms,
                            result_local,
                            join_block,
                        );
                    } else {
                        // No guard — treat as success
                        self.emit_decision_tree(
                            success,
                            scrutinee,
                            arms,
                            result_local,
                            join_block,
                        );
                    }
                }
            },

            DecisionTree::Failure => {
                // Unreachable — exhaustiveness should prevent this
                self.set_terminator(Terminator::panic("match failure: non-exhaustive patterns"));
            },
        }
    }

    /// Emit binding assignments: extract values from scrutinee via access paths.
    fn emit_bindings(&mut self, bindings: &[Binding], scrutinee: &Place) {
        for binding in bindings {
            let mir_local = self.map_local(binding.local_id);
            let source = apply_access_path(scrutinee.clone(), &binding.path);
            self.emit_stmt(Statement::new(StatementKind::Assign {
                dest: Place::local(mir_local),
                rvalue: Rvalue::Copy(source),
            }));
        }
    }

    /// Get the ResolvedTy for an expression (needed for pattern matching).
    fn resolve_expr_resolved_ty(&self, expr_id: HirExprId) -> kestrel_type_infer::result::ResolvedTy {
        if let Some(typed) = self.typed {
            if let Some(resolved) = typed.expr_types.get(&expr_id) {
                return resolved.clone();
            }
        }
        kestrel_type_infer::result::ResolvedTy::Error
    }

    /// Lower a HirBlock (stmts + optional tail expr).
    fn lower_hir_block(&mut self, block: &HirBlock) -> Value {
        for &stmt_id in &block.stmts {
            self.lower_stmt(stmt_id);
            if self.is_terminated() {
                return Value::Immediate(Immediate::unit());
            }
        }

        if let Some(tail) = block.tail_expr {
            self.lower_expr(tail)
        } else {
            Value::Immediate(Immediate::unit())
        }
    }
}

/// Convert a Value to an Rvalue for assignment.
fn value_to_rvalue(value: Value) -> Rvalue {
    match value {
        Value::Place(p) => Rvalue::Copy(p),
        Value::Immediate(i) => Rvalue::Const(i),
    }
}

/// Decode a HIR string literal using lib1-compatible escape handling.
///
/// HIR currently stores both regular and raw strings in `HirLiteral::String`,
/// preserving the original quotes. Normal quoted strings are unescaped; raw
/// triple-quoted strings have only their surrounding quotes stripped.
fn decode_string_literal(raw: &str) -> String {
    let quote_count = raw.chars().take_while(|&c| c == '"').count();
    if quote_count >= 3
        && raw.len() >= quote_count * 2
        && raw.ends_with(&"\"".repeat(quote_count))
    {
        return raw[quote_count..raw.len() - quote_count].to_string();
    }

    if raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"') {
        return unescape_string_literal(&raw[1..raw.len() - 1]);
    }

    unescape_string_literal(raw)
}

/// Decode string escapes with the same semantics as lib1's binder helper.
fn unescape_string_literal(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.char_indices().peekable();

    while let Some((_, c)) = chars.next() {
        if c != '\\' {
            result.push(c);
            continue;
        }

        match chars.next() {
            None => result.push('\\'),
            Some((_, next_char)) => match next_char {
                'n' => result.push('\n'),
                'r' => result.push('\r'),
                't' => result.push('\t'),
                '\\' => result.push('\\'),
                '"' => result.push('"'),
                '\'' => result.push('\''),
                '0' => result.push('\0'),
                '\n' => {
                    while let Some(&(_, ch)) = chars.peek() {
                        if ch == ' ' || ch == '\t' {
                            chars.next();
                        } else {
                            break;
                        }
                    }
                }
                '\r' => {
                    if let Some(&(_, '\n')) = chars.peek() {
                        chars.next();
                    }
                    while let Some(&(_, ch)) = chars.peek() {
                        if ch == ' ' || ch == '\t' {
                            chars.next();
                        } else {
                            break;
                        }
                    }
                }
                'x' => {
                    let mut hex_str = String::new();
                    for _ in 0..2 {
                        if let Some(&(_, ch)) = chars.peek() {
                            if ch.is_ascii_hexdigit() {
                                hex_str.push(ch);
                                chars.next();
                            } else {
                                break;
                            }
                        }
                    }
                    if hex_str.len() != 2 {
                        result.push_str(&format!("\\x{}", hex_str));
                    } else {
                        let value = u8::from_str_radix(&hex_str, 16).unwrap();
                        if value > 0x7F {
                            result.push_str(&format!("\\x{:02X}", value));
                        } else {
                            result.push(value as char);
                        }
                    }
                }
                'u' => {
                    if chars.peek().map(|&(_, c)| c) != Some('{') {
                        result.push_str("\\u");
                        continue;
                    }
                    chars.next();

                    let mut hex_str = String::new();
                    let mut found_close = false;
                    while let Some(&(_, ch)) = chars.peek() {
                        if ch == '}' {
                            chars.next();
                            found_close = true;
                            break;
                        } else if ch.is_ascii_hexdigit() && hex_str.len() < 6 {
                            hex_str.push(ch);
                            chars.next();
                        } else if ch.is_ascii_hexdigit() {
                            hex_str.push(ch);
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    let escape_seq = format!("\\u{{{}}}", hex_str);
                    if !found_close || hex_str.is_empty() || hex_str.len() > 6 {
                        result.push_str(&escape_seq);
                    } else {
                        match u32::from_str_radix(&hex_str, 16) {
                            Ok(code_point) if code_point <= 0x10FFFF => {
                                if let Some(ch) = char::from_u32(code_point) {
                                    result.push(ch);
                                } else {
                                    result.push_str(&escape_seq);
                                }
                            }
                            _ => result.push_str(&escape_seq),
                        }
                    }
                }
                other => {
                    result.push('\\');
                    result.push(other);
                }
            },
        }
    }

    result
}

/// Apply an access path to a place to reach a sub-value.
/// e.g., scrutinee + [Downcast("Some"), Index(0)] → scrutinee.Some.0
///
/// `IndexFromEnd` / `RestSlice` produce values (not places) and must be
/// materialized by `emit_bindings` via the `ArrayMatchable` protocol — they
/// should never appear in switch-test paths. If they're encountered here,
/// emit_bindings didn't route them correctly; fall back to returning the
/// scrutinee place unchanged so MIR still compiles (runtime correctness is
/// handled by emit_array_pattern_binding below).
fn apply_access_path(mut place: Place, path: &[PathElement]) -> Place {
    for elem in path {
        place = match elem {
            PathElement::Field(name) => place.field(name.clone()),
            PathElement::Index(i) => place.index(*i),
            PathElement::Downcast(variant) => place.downcast(variant.clone()),
            // Array-matchable paths: pass through (emit_bindings handles them
            // separately via matchGet / matchSlice protocol calls).
            PathElement::IndexFromEnd(_) | PathElement::RestSlice { .. } => place,
        };
    }
    place
}

/// Map a decision-tree `Constructor` to a `SwitchCase` for MIR.
///
/// The single-constructor cases (`Tuple`, `Struct`, `Unit`) are never
/// emitted as a real multi-case switch — they get flattened to unconditional
/// jumps upstream — but they're still handled here for completeness.
fn constructor_to_switch_case(ctor: &Constructor, ctx: &mut LowerCtx) -> kestrel_mir::SwitchCase {
    use kestrel_mir::SwitchCase;
    match ctor {
        Constructor::True => SwitchCase::Bool(true),
        Constructor::False => SwitchCase::Bool(false),
        Constructor::Variant { entity, .. } => {
            ctx.register_name(*entity);
            SwitchCase::Variant(ctx.module.resolve_name(*entity).to_string())
        }
        Constructor::Struct { entity, .. } => {
            ctx.register_name(*entity);
            SwitchCase::Variant(ctx.module.resolve_name(*entity).to_string())
        }
        Constructor::IntLiteral(v) => SwitchCase::IntLiteral(*v),
        Constructor::IntRange { start, end } => SwitchCase::IntRange {
            start: *start,
            end: *end,
        },
        Constructor::CharLiteral(c) => SwitchCase::CharLiteral(*c as u32),
        Constructor::CharRange { start, end } => SwitchCase::CharRange {
            start: start.map(|c| c as u32),
            end: end.map(|c| c as u32),
        },
        Constructor::StringLiteral(s) => SwitchCase::StringLiteral(s.clone()),
        Constructor::Wildcard | Constructor::Tuple { .. } | Constructor::Unit => {
            SwitchCase::Wildcard
        }
        Constructor::Array { .. } | Constructor::NonExhaustive | Constructor::Missing => {
            SwitchCase::Wildcard
        }
    }
}

/// Find the `NodeKind::Setter` child of a Field or Subscript entity.
/// Setters are spawned by the AST builder as children — one per declaration
/// with a `SetterClause` — so at most one match per parent.
fn find_setter_child(ctx: &LowerCtx, parent: Entity) -> Option<Entity> {
    ctx.world
        .children_of(parent)
        .iter()
        .copied()
        .find(|&e| {
            matches!(
                ctx.world.get::<kestrel_ast_builder::NodeKind>(e),
                Some(kestrel_ast_builder::NodeKind::Setter)
            )
        })
}

#[cfg(test)]
mod tests {
    use super::decode_string_literal;

    #[test]
    fn decode_string_literal_unescapes_like_lib1() {
        assert_eq!(
            decode_string_literal("\"\\x1b[31mhello\\n\\u{41}\""),
            "\x1b[31mhello\nA"
        );
    }

    #[test]
    fn decode_string_literal_preserves_raw_strings() {
        assert_eq!(
            decode_string_literal("\"\"\"\\x1b[31mhello\\n\"\"\""),
            "\\x1b[31mhello\\n"
        );
    }
}

