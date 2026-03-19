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
                self.set_terminator(Terminator::ret(value));
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
            HirExpr::Literal { value, .. } => self.lower_literal(value),
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
                    _ => {
                        // Type reference or unknown — return as entity reference
                        Value::Immediate(Immediate::function_ref(*entity))
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
                args,
                ..
            } => self.lower_method_call(expr_id, *receiver, method, args),
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

                // Lower args if present (e.g., .Some(value))
                let payload: Vec<Value> = args
                    .as_ref()
                    .map(|a| a.iter().map(|arg| self.lower_expr(arg.value)).collect())
                    .unwrap_or_default();

                // Construct enum variant
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
            },

            // === Array literal ===
            HirExpr::Array { elements, .. } => {
                let result_ty = self.resolve_expr_type(expr_id);
                let values: Vec<Value> = elements.iter().map(|&e| self.lower_expr(e)).collect();
                // Emit as a construct with indexed fields (0, 1, 2, ...)
                let fields: Vec<(String, Value)> = values
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| (format!("{}", i), v))
                    .collect();
                let dest = self.fresh_temp(result_ty.clone());
                self.emit_stmt(Statement::new(StatementKind::Assign {
                    dest: Place::local(dest),
                    rvalue: Rvalue::Construct {
                        ty: result_ty,
                        fields,
                    },
                }));
                Value::Place(Place::local(dest))
            },

            // === Dict literal ===
            HirExpr::Dict { entries, .. } => {
                let result_ty = self.resolve_expr_type(expr_id);
                // Lower each key-value pair as indexed fields
                let mut fields = Vec::new();
                for (i, entry) in entries.iter().enumerate() {
                    let key = self.lower_expr(entry.key);
                    let val = self.lower_expr(entry.value);
                    fields.push((format!("{}.key", i), key));
                    fields.push((format!("{}.value", i), val));
                }
                let dest = self.fresh_temp(result_ty.clone());
                self.emit_stmt(Statement::new(StatementKind::Assign {
                    dest: Place::local(dest),
                    rvalue: Rvalue::Construct {
                        ty: result_ty,
                        fields,
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
                return resolved_args
                    .iter()
                    .map(|ty| lower_resolved_ty(self.ctx, ty))
                    .collect();
            }
        }
        Vec::new()
    }

    /// Lower call arguments from HIR to MIR.
    /// Default passing mode is Ref (borrow). Proper move/copy semantics
    /// will be handled by the deinit pass later.
    fn lower_call_args(&mut self, args: &[HirCallArg]) -> Vec<CallArg> {
        args.iter()
            .map(|arg| {
                let value = self.lower_expr(arg.value);
                CallArg::borrow(value)
            })
            .collect()
    }

    /// Lower a direct call: `callee(args...)`
    fn lower_call(
        &mut self,
        expr_id: HirExprId,
        callee_expr: HirExprId,
        args: &[HirCallArg],
    ) -> Value {
        let call_args = self.lower_call_args(args);
        let result_ty = self.resolve_expr_type(expr_id);

        // Check what the callee is
        let callee_hir = self.hir.exprs[callee_expr].clone();
        match &callee_hir {
            // Direct function call: foo(args)
            HirExpr::Def(entity, _, _) => {
                self.ctx.register_name(*entity);
                let type_args = self.resolve_type_args(callee_expr);
                let callee = Callee::direct_generic(*entity, type_args);
                self.emit_call(callee, call_args, result_ty)
            },
            // Overloaded function call: resolved by inference
            HirExpr::OverloadSet { candidates, .. } => {
                let resolved = self.typed
                    .and_then(|t| t.resolutions.get(&callee_expr))
                    .copied()
                    .or_else(|| candidates.first().copied());
                if let Some(entity) = resolved {
                    self.ctx.register_name(entity);
                    let type_args = self.resolve_type_args(callee_expr);
                    let callee = Callee::direct_generic(entity, type_args);
                    self.emit_call(callee, call_args, result_ty)
                } else {
                    Value::Immediate(Immediate::error())
                }
            },
            // Indirect call through a variable/expression
            _ => {
                let callee_val = self.lower_expr(callee_expr);
                match callee_val {
                    Value::Place(p) => {
                        // Could be thin or thick — default to thick for now
                        let callee = Callee::Thick(p);
                        self.emit_call(callee, call_args, result_ty)
                    },
                    Value::Immediate(Immediate { kind: ImmediateKind::FunctionRef { func, type_args }, .. }) => {
                        let callee = Callee::direct_generic(func, type_args);
                        self.emit_call(callee, call_args, result_ty)
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
        _method_name: &str,
        args: &[HirCallArg],
    ) -> Value {
        let receiver_val = self.lower_expr(receiver_expr);
        let result_ty = self.resolve_expr_type(expr_id);

        // Build args: receiver first, then explicit args
        let mut call_args = vec![CallArg::borrow(receiver_val)];
        call_args.extend(self.lower_call_args(args));

        // Type inference tells us which function entity this resolves to
        if let Some(&resolved_entity) = self.typed.and_then(|t| t.resolutions.get(&expr_id)) {
            self.ctx.register_name(resolved_entity);
            let type_args = self.resolve_type_args(expr_id);
            let receiver_ty = self.resolve_expr_type(receiver_expr);
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
        let receiver_val = self.lower_expr(receiver_expr);
        let result_ty = self.resolve_expr_type(expr_id);
        let receiver_ty = self.resolve_expr_type(receiver_expr);

        // Build args: receiver first, then explicit args
        let mut call_args = vec![CallArg::borrow(receiver_val)];
        call_args.extend(self.lower_call_args(args));

        // Check if inference resolved this to a concrete function
        if let Some(&resolved_entity) = self.typed.and_then(|t| t.resolutions.get(&expr_id)) {
            // Resolved to a concrete method — direct call
            self.ctx.register_name(resolved_entity);
            let type_args = self.resolve_type_args(expr_id);
            let callee = Callee::method(resolved_entity, type_args, receiver_ty);
            self.emit_call(callee, call_args, result_ty)
        } else {
            // Emit as witness call — resolved at monomorphization time
            self.ctx.register_name(protocol);
            let method_type_args = self.resolve_type_args(expr_id);
            let callee = Callee::witness(protocol, method_name, receiver_ty, method_type_args);
            self.emit_call(callee, call_args, result_ty)
        }
    }

    /// Emit a call statement and return the result value.
    fn emit_call(
        &mut self,
        callee: Callee,
        args: Vec<CallArg>,
        result_ty: MirTy,
    ) -> Value {
        if result_ty == MirTy::Unit || result_ty == MirTy::Never {
            // Void call — no destination
            self.emit_stmt(Statement::new(StatementKind::Call {
                dest: None,
                callee,
                args,
            }));
            Value::Immediate(Immediate::unit())
        } else {
            // Call with return value — create temp for result
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
    fn lower_literal(&self, lit: &HirLiteral) -> Value {
        match lit {
            HirLiteral::Integer(v) => Value::Immediate(Immediate::i64(*v)),
            HirLiteral::Float(v) => Value::Immediate(Immediate::f64(*v)),
            HirLiteral::Bool(v) => Value::Immediate(Immediate::bool(*v)),
            HirLiteral::String(s) => Value::Immediate(Immediate::string(s.clone())),
            HirLiteral::Char(c) => Value::Immediate(Immediate::i32(*c as i32)),
            HirLiteral::Null => Value::Immediate(Immediate::unit()),
        }
    }

    /// Lower an if expression.
    /// Creates: current_block → branch → then_block / else_block → merge_block
    fn lower_if(
        &mut self,
        _expr_id: HirExprId,
        condition: HirExprId,
        then_body: &HirBlock,
        else_body: Option<&HirBlock>,
    ) -> Value {
        let cond_val = self.lower_expr(condition);

        let then_block = self.new_block();
        let else_block = self.new_block();
        let merge_block = self.new_block();

        // Branch on condition
        self.set_terminator(Terminator::branch(cond_val, then_block, else_block));

        // Lower then branch
        self.switch_to_block(then_block);
        let then_val = self.lower_hir_block(then_body);
        if !self.is_terminated() {
            self.set_terminator(Terminator::jump(merge_block));
        }

        // Lower else branch
        self.switch_to_block(else_block);
        if let Some(else_body) = else_body {
            let _else_val = self.lower_hir_block(else_body);
        }
        if !self.is_terminated() {
            self.set_terminator(Terminator::jump(merge_block));
        }

        // Continue in merge block
        self.switch_to_block(merge_block);

        // TODO: if/else as an expression should merge the then/else values
        // into a phi-like construct. For now, return the then value.
        then_val
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
    /// 2. Create env struct if capturing
    /// 3. Create a synthetic call function
    /// 4. Emit ApplyPartial to create the thick callable value
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

        // Generate unique closure name
        let closure_idx = self.temp_counter;
        self.temp_counter += 1;
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

        // Create the synthetic call function
        let closure_entity = kestrel_hecs::Entity::from_raw(u32::MAX - closure_idx);
        self.ctx.module.register_name(closure_entity, &closure_name);

        let mut func_def = FunctionDef::new(closure_entity, &closure_name, ret_ty.clone());
        func_def.kind = if captured_locals.is_empty() {
            FunctionKind::Free
        } else {
            // Will be set properly when we have an env struct
            FunctionKind::Free
        };

        // === Build closure body by swapping BodyLowerCtx state ===

        // Create the closure's MIR body
        let mut closure_body = MirBody::new();

        // Add env parameter (first param, even for non-capturing — ABI consistency)
        let env_ty = MirTy::Pointer(Box::new(MirTy::Unit));
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
            // Map the HIR local to the closure's MIR local
            closure_local_map.insert(cp.local, local);
        }

        // Map captured locals: create locals in the closure body that mirror
        // the captured parent locals. For now, these are just copies — a full
        // implementation would load them from the env struct.
        for &captured in &captured_locals {
            let cap_ty = self.resolve_local_type(captured);
            let cap_name = self.hir.locals[captured].name.clone();
            let closure_local = closure_body.add_local(LocalDef::new(cap_name, cap_ty));
            closure_local_map.insert(captured, closure_local);
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
        // func_id will be set after we add the function

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

        let _ = closure_func_id;
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
                let case_blocks: Vec<(String, BlockId)> = cases
                    .iter()
                    .map(|(ctor, _)| {
                        let name = constructor_name(ctor);
                        let block = self.new_block();
                        (name, block)
                    })
                    .collect();

                let default_block = if default.is_some() {
                    Some(self.new_block())
                } else {
                    None
                };

                // Build switch cases — include default as the last case if present
                let mut switch_cases = case_blocks.clone();
                if let Some(def_block) = default_block {
                    switch_cases.push(("_".to_string(), def_block));
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

/// Apply an access path to a place to reach a sub-value.
/// e.g., scrutinee + [Downcast("Some"), Index(0)] → scrutinee.Some.0
fn apply_access_path(mut place: Place, path: &[PathElement]) -> Place {
    for elem in path {
        place = match elem {
            PathElement::Field(name) => place.field(name.clone()),
            PathElement::Index(i) => place.index(*i),
            PathElement::Downcast(variant) => place.downcast(variant.clone()),
        };
    }
    place
}

/// Get a display name for a constructor (used in switch case labels).
fn constructor_name(ctor: &Constructor) -> String {
    match ctor {
        Constructor::True => "true".to_string(),
        Constructor::False => "false".to_string(),
        Constructor::Variant { entity, .. } => format!("{:?}", entity),
        Constructor::Tuple { arity } => format!("tuple_{}", arity),
        Constructor::Struct { entity, .. } => format!("{:?}", entity),
        Constructor::IntLiteral(v) => format!("{}", v),
        Constructor::IntRange { start, end } => {
            let s = start.map(|v| v.to_string()).unwrap_or_default();
            let e = end.map(|v| v.to_string()).unwrap_or_default();
            format!("{}..{}", s, e)
        },
        Constructor::CharLiteral(c) => format!("'{}'", c),
        Constructor::CharRange { start, end } => {
            let s = start.map(|c| format!("'{}'", c)).unwrap_or_default();
            let e = end.map(|c| format!("'{}'", c)).unwrap_or_default();
            format!("{}..{}", s, e)
        },
        Constructor::StringLiteral(s) => format!("{:?}", s),
        Constructor::Unit => "()".to_string(),
        Constructor::Wildcard => "_".to_string(),
        Constructor::Array { prefix_len, .. } => format!("array_{}", prefix_len),
        Constructor::NonExhaustive => "non_exhaustive".to_string(),
        Constructor::Missing => "missing".to_string(),
    }
}

