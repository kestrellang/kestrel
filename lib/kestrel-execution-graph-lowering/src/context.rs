//! Lowering context - holds all state during the lowering pass.

use std::collections::HashMap;

use kestrel_execution_graph::{
    BasicBlock, Block, Callee, CallArg, Function, Id, Immediate, Local, MirContext, Place,
    QualifiedName, Rvalue, StatementKind, Terminator, TerminatorKind, Ty, TypeParam, Value,
};
use kestrel_reporting::{Diagnostic, IntoDiagnostic};
use kestrel_semantic_model::SemanticModel;
use kestrel_semantic_tree::behavior::copy_semantics::CopySemanticsBehavior;
use kestrel_semantic_tree::behavior::deinit::DeinitBehavior;
use kestrel_semantic_tree::expr::LoopId;
use kestrel_semantic_tree::symbol::local::LocalId;
use kestrel_semantic_tree::ty::TyKind;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::error::LoweringError;
use crate::LoweringResult;

/// Information about a loop for break/continue resolution.
#[derive(Debug, Clone)]
pub struct LoopInfo {
    /// The loop identifier from the semantic tree.
    pub loop_id: LoopId,
    /// The header block (where condition is checked, for while loops).
    /// For infinite loops, this is the body entry.
    pub header_block: Id<Block>,
    /// The exit block (where to jump on break).
    pub exit_block: Id<Block>,
    /// Index into scope_stack where this loop's scope begins.
    /// Used to emit deinits for break/continue.
    pub scope_depth: usize,
}

// =============================================================================
// Deinit/Drop Tracking
// =============================================================================

/// Tracks a local variable's deinit status during lowering.
///
/// This is used to determine whether a variable needs to have its destructor
/// called at scope exit, and whether that call should be conditional.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeinitStatus {
    /// Value is definitely valid and needs deinit at scope exit.
    Valid,
    /// Value was definitely moved, no deinit needed.
    Moved,
    /// Value was conditionally moved in one branch but not another.
    /// The flag is a Bool local that is true if the value needs deinit.
    MaybeMoved { flag: Id<Local> },
}

/// Information about a lexical scope for deinit insertion.
///
/// Each lexical scope (function body, if branch, loop body, block expression, etc.)
/// tracks which locals were declared in it and their deinit status.
#[derive(Debug, Clone)]
pub struct ScopeInfo {
    /// Locals declared in this scope, in declaration order.
    /// At scope exit, these are deinited in reverse order.
    pub locals: Vec<Id<Local>>,
    /// Current deinit status of each local in scope.
    /// Only locals that need deinit (non-Copyable with deinit behavior) are tracked here.
    pub deinit_status: HashMap<Id<Local>, DeinitStatus>,
}

/// The central context for lowering semantic tree to MIR.
///
/// This struct holds all state needed during the lowering pass, including:
/// - Reference to the semantic model (source)
/// - The MIR context being built (destination)
/// - Current function state
/// - Local variable mappings
/// - Loop stack for break/continue
/// - Scope stack for deinit tracking
/// - Collected diagnostics
pub struct LoweringContext<'a> {
    /// The semantic model providing queries and context.
    pub model: &'a SemanticModel,

    /// The MIR context being built.
    pub mir: MirContext,

    /// The current function being lowered (if any).
    current_function: Option<Id<Function>>,

    /// Maps semantic LocalId to MIR local Id.
    /// Reset for each function.
    local_map: HashMap<LocalId, Id<Local>>,

    /// Maps semantic TypeParameterSymbol ID to MIR TypeParam ID.
    /// Set when entering a generic item (struct, enum, function).
    type_param_map: HashMap<SymbolId, Id<TypeParam>>,

    /// Stack of active loops for break/continue resolution.
    loop_stack: Vec<LoopInfo>,

    /// Stack of lexical scopes for deinit tracking.
    /// Each scope tracks locals declared in it and their deinit status.
    scope_stack: Vec<ScopeInfo>,

    /// Temporaries created during current statement evaluation.
    /// These are deinited at the end of each statement.
    statement_temps: Vec<Id<Local>>,

    /// The current block being built.
    current_block: Option<Id<Block>>,

    /// Diagnostics collected during lowering.
    diagnostics: Vec<Diagnostic<usize>>,

    /// Counter for generating unique temporary names.
    temp_counter: u32,

    /// Counter for generating unique closure indices within a function.
    /// Reset when entering a new function.
    closure_counter: u32,

    /// Counter for generating unique deinit flag names.
    deinit_flag_counter: u32,
}

impl<'a> LoweringContext<'a> {
    /// Create a new lowering context.
    pub fn new(model: &'a SemanticModel) -> Self {
        LoweringContext {
            model,
            mir: MirContext::new(),
            current_function: None,
            local_map: HashMap::new(),
            type_param_map: HashMap::new(),
            loop_stack: Vec::new(),
            scope_stack: Vec::new(),
            statement_temps: Vec::new(),
            current_block: None,
            diagnostics: Vec::new(),
            temp_counter: 0,
            closure_counter: 0,
            deinit_flag_counter: 0,
        }
    }

    /// Finish lowering and return the result.
    pub fn finish(self) -> LoweringResult {
        LoweringResult {
            mir: self.mir,
            diagnostics: self.diagnostics,
        }
    }

    // === Diagnostics ===

    /// Emit a lowering error as a diagnostic.
    pub fn emit_error(&mut self, error: LoweringError) {
        self.diagnostics.push(error.into_diagnostic());
    }

    /// Emit a raw diagnostic.
    pub fn emit_diagnostic(&mut self, diagnostic: Diagnostic<usize>) {
        self.diagnostics.push(diagnostic);
    }

    // === Function Management ===

    /// Enter a function for lowering.
    pub fn enter_function(&mut self, func_id: Id<Function>) {
        self.current_function = Some(func_id);
        self.local_map.clear();
        self.loop_stack.clear();
        self.scope_stack.clear();
        self.statement_temps.clear();
        self.current_block = None;
        self.temp_counter = 0;
        self.closure_counter = 0;
        self.deinit_flag_counter = 0;
    }

    /// Exit the current function.
    pub fn exit_function(&mut self) {
        self.current_function = None;
        self.local_map.clear();
        self.loop_stack.clear();
        self.scope_stack.clear();
        self.statement_temps.clear();
        self.current_block = None;
    }

    /// Get the current function ID.
    pub fn current_function(&self) -> Option<Id<Function>> {
        self.current_function
    }

    /// Get the current function ID, panicking if not in a function.
    pub fn current_function_unwrap(&self) -> Id<Function> {
        self.current_function
            .expect("expected to be inside a function")
    }

    // === Local Variable Mapping ===

    /// Map a semantic local ID to a MIR local ID.
    pub fn map_local(&mut self, semantic_id: LocalId, mir_id: Id<Local>) {
        self.local_map.insert(semantic_id, mir_id);
    }

    /// Get the MIR local ID for a semantic local ID.
    pub fn get_local(&self, semantic_id: LocalId) -> Option<Id<Local>> {
        self.local_map.get(&semantic_id).copied()
    }

    /// Get the MIR local ID, panicking if not found.
    pub fn get_local_unwrap(&self, semantic_id: LocalId) -> Id<Local> {
        self.local_map
            .get(&semantic_id)
            .copied()
            .unwrap_or_else(|| panic!("local {:?} not found in local_map", semantic_id))
    }

    // === Type Parameter Mapping ===

    /// Map a semantic type parameter symbol ID to a MIR type param ID.
    pub fn map_type_param(&mut self, semantic_id: SymbolId, mir_id: Id<TypeParam>) {
        self.type_param_map.insert(semantic_id, mir_id);
    }

    /// Get the MIR type param ID for a semantic type parameter symbol.
    pub fn get_type_param(&self, semantic_id: SymbolId) -> Option<Id<TypeParam>> {
        self.type_param_map.get(&semantic_id).copied()
    }

    /// Clear the type parameter mapping (when leaving a generic context).
    pub fn clear_type_params(&mut self) {
        self.type_param_map.clear();
    }

    // === Loop Stack ===

    /// Push a loop onto the stack.
    /// The loop's scope_depth is set to the current scope stack depth.
    pub fn push_loop(&mut self, loop_id: LoopId, header_block: Id<Block>, exit_block: Id<Block>) {
        self.loop_stack.push(LoopInfo {
            loop_id,
            header_block,
            exit_block,
            scope_depth: self.scope_stack.len(),
        });
    }

    /// Pop a loop from the stack.
    pub fn pop_loop(&mut self) -> Option<LoopInfo> {
        self.loop_stack.pop()
    }

    /// Find a loop by its ID.
    pub fn find_loop(&self, loop_id: LoopId) -> Option<&LoopInfo> {
        self.loop_stack.iter().rev().find(|l| l.loop_id == loop_id)
    }

    /// Get the innermost loop.
    pub fn innermost_loop(&self) -> Option<&LoopInfo> {
        self.loop_stack.last()
    }

    // === Block Management ===

    /// Set the current block.
    pub fn set_current_block(&mut self, block: Id<Block>) {
        self.current_block = Some(block);
    }

    /// Get the current block.
    pub fn current_block(&self) -> Option<Id<Block>> {
        self.current_block
    }

    /// Get the current block, panicking if not set.
    pub fn current_block_unwrap(&self) -> Id<Block> {
        self.current_block.expect("expected a current block")
    }

    /// Check if the current block is terminated.
    pub fn is_block_terminated(&self) -> bool {
        if let Some(block_id) = self.current_block {
            self.mir.block(block_id).terminator.is_some()
        } else {
            true // No current block is considered "terminated"
        }
    }

    // === Temporary Names ===

    /// Generate a fresh temporary name.
    pub fn fresh_temp(&mut self, prefix: &str) -> String {
        let n = self.temp_counter;
        self.temp_counter += 1;
        format!("{}_{}", prefix, n)
    }

    // === Closure Support ===

    /// Get the next closure index and increment the counter.
    pub fn next_closure_index(&mut self) -> u32 {
        let idx = self.closure_counter;
        self.closure_counter += 1;
        idx
    }

    /// Save the current local map (for restoring after lowering a nested closure).
    pub fn save_local_map(&self) -> HashMap<LocalId, Id<Local>> {
        self.local_map.clone()
    }

    /// Restore a previously saved local map.
    pub fn restore_local_map(&mut self, map: HashMap<LocalId, Id<Local>>) {
        self.local_map = map;
    }

    /// Set the current function (used when switching to closure context).
    pub fn set_current_function(&mut self, func_id: Option<Id<Function>>) {
        self.current_function = func_id;
    }

    /// Get the current closure counter value (for saving).
    pub fn get_closure_counter(&self) -> u32 {
        self.closure_counter
    }

    /// Set the closure counter (for restoring).
    pub fn set_closure_counter(&mut self, counter: u32) {
        self.closure_counter = counter;
    }

    /// Get the current temp counter value (for saving).
    pub fn get_temp_counter(&self) -> u32 {
        self.temp_counter
    }

    /// Set the temp counter (for restoring).
    pub fn set_temp_counter(&mut self, counter: u32) {
        self.temp_counter = counter;
    }

    // === Statement Emission ===

    /// Add a statement to the current block.
    pub fn emit_statement(&mut self, kind: StatementKind) {
        let block_id = self.current_block_unwrap();
        let stmt = kestrel_execution_graph::StatementData {
            meta: kestrel_execution_graph::Metadata::new(),
            priors: Vec::new(),
            kind,
        };
        let stmt_id = self.mir.statements.alloc(stmt);
        self.mir.block_mut(block_id).statements.push(stmt_id);
    }

    /// Emit an assignment statement.
    pub fn emit_assign(&mut self, dest: Place, rvalue: Rvalue) {
        self.emit_statement(StatementKind::Assign { dest, rvalue });
    }

    /// Emit a copy assignment.
    pub fn emit_copy(&mut self, dest: Place, src: Place) {
        self.emit_assign(dest, Rvalue::Copy(src));
    }

    /// Emit an immediate assignment.
    pub fn emit_imm(&mut self, dest: Place, imm: Immediate) {
        self.emit_assign(dest, Rvalue::Use(imm));
    }

    /// Emit an assignment from a value (place or immediate).
    pub fn emit_assign_value(&mut self, dest: Place, value: Value) {
        match value {
            Value::Place(p) => self.emit_copy(dest, p),
            Value::Immediate(i) => self.emit_imm(dest, i),
        }
    }

    // === Terminator Emission ===

    /// Set the terminator for the current block.
    pub fn emit_terminator(&mut self, kind: TerminatorKind) {
        let block_id = self.current_block_unwrap();
        self.mir.block_mut(block_id).terminator = Some(Terminator {
            meta: kestrel_execution_graph::Metadata::new(),
            kind,
        });
    }

    /// Emit a return terminator.
    pub fn emit_return(&mut self, value: Value) {
        self.emit_terminator(TerminatorKind::Return(value));
    }

    /// Emit a return unit terminator.
    pub fn emit_return_unit(&mut self) {
        self.emit_return(Value::Immediate(Immediate::unit()));
    }

    /// Emit an unconditional jump.
    pub fn emit_jump(&mut self, target: Id<Block>) {
        self.emit_terminator(TerminatorKind::Jump(target));
    }

    /// Emit a conditional branch.
    pub fn emit_branch(&mut self, condition: Value, then_block: Id<Block>, else_block: Id<Block>) {
        self.emit_terminator(TerminatorKind::Branch {
            condition,
            then_block,
            else_block,
        });
    }

    /// Emit a switch terminator.
    pub fn emit_switch(&mut self, discriminant: Place, cases: Vec<(String, Id<Block>)>) {
        self.emit_terminator(TerminatorKind::Switch { discriminant, cases });
    }

    /// Emit an unreachable terminator.
    pub fn emit_unreachable(&mut self) {
        self.emit_terminator(TerminatorKind::Unreachable);
    }

    // === Local Creation ===

    /// Create a new local in the current function.
    pub fn create_local(&mut self, name: impl Into<String>, ty: Id<Ty>) -> Id<Local> {
        let func_id = self.current_function_unwrap();
        let mut builder = self.mir.function_builder(func_id);
        builder.local(name, ty)
    }

    /// Create a temporary local.
    pub fn create_temp(&mut self, prefix: &str, ty: Id<Ty>) -> Id<Local> {
        let name = self.fresh_temp(prefix);
        self.create_local(name, ty)
    }

    // === Block Creation ===

    /// Create a new block in the current function.
    pub fn create_block(&mut self) -> Id<Block> {
        let func_id = self.current_function_unwrap();
        let block = BasicBlock::new();
        let block_id = self.mir.blocks.alloc(block);
        self.mir.function_mut(func_id).blocks.push(block_id);
        block_id
    }

    // === Call Emission ===

    /// Emit a call that assigns its result to a place.
    ///
    /// For now, all arguments default to `PassingMode::Ref` (borrow).
    /// This will be updated when parameter access modes are available during lowering.
    pub fn emit_call(&mut self, dest: Place, callee: Callee, args: Vec<Value>) {
        // Convert values to CallArgs with default Ref passing mode
        let call_args: Vec<CallArg> = args
            .into_iter()
            .map(|v| CallArg::borrow(v))
            .collect();
        self.emit_assign(dest, Rvalue::Call { callee, args: call_args });
    }

    /// Emit a call with explicit passing modes for each argument.
    pub fn emit_call_with_modes(
        &mut self,
        dest: Place,
        callee: Callee,
        args: Vec<CallArg>,
    ) {
        self.emit_assign(dest, Rvalue::Call { callee, args });
    }

    /// Emit a direct function call and assign result to a place.
    pub fn emit_direct_call(
        &mut self,
        dest: Place,
        func_name: Id<QualifiedName>,
        type_args: Vec<Id<Ty>>,
        args: Vec<Value>,
    ) {
        let callee = if type_args.is_empty() {
            Callee::direct(func_name)
        } else {
            Callee::direct_generic(func_name, type_args)
        };
        self.emit_call(dest, callee, args);
    }

    /// Emit a call to a unit-returning function (no result assignment needed).
    ///
    /// For now, all arguments default to `PassingMode::Ref` (borrow).
    pub fn emit_call_unit(&mut self, callee: Callee, args: Vec<Value>) {
        // Convert values to CallArgs with default Ref passing mode
        let call_args: Vec<CallArg> = args
            .into_iter()
            .map(|v| CallArg::borrow(v))
            .collect();
        self.emit_statement(StatementKind::Call { callee, args: call_args });
    }

    /// Emit a call to a unit-returning function with explicit passing modes.
    pub fn emit_call_unit_with_modes(&mut self, callee: Callee, args: Vec<CallArg>) {
        self.emit_statement(StatementKind::Call { callee, args });
    }

    // ==========================================================================
    // Scope Management for Deinit
    // ==========================================================================

    /// Enter a new lexical scope.
    ///
    /// Call this when entering a block expression, if branch, loop body, etc.
    pub fn enter_scope(&mut self) {
        self.scope_stack.push(ScopeInfo {
            locals: Vec::new(),
            deinit_status: HashMap::new(),
        });
    }

    /// Exit the current scope, emitting deinits for all tracked locals.
    ///
    /// Locals are deinited in reverse declaration order.
    pub fn exit_scope(&mut self) {
        if let Some(scope) = self.scope_stack.pop() {
            self.emit_scope_deinits(&scope);
        }
    }

    /// Exit the current scope WITHOUT emitting deinits.
    ///
    /// Returns the scope info for later processing (e.g., branch merging).
    pub fn exit_scope_no_emit(&mut self) -> Option<ScopeInfo> {
        self.scope_stack.pop()
    }

    /// Get the current scope depth.
    pub fn scope_depth(&self) -> usize {
        self.scope_stack.len()
    }

    /// Emit deinit statements for a scope's locals in reverse declaration order.
    pub fn emit_scope_deinits(&mut self, scope: &ScopeInfo) {
        // Skip if block is already terminated
        if self.is_block_terminated() {
            return;
        }

        // Deinit in reverse declaration order
        for &local in scope.locals.iter().rev() {
            if let Some(status) = scope.deinit_status.get(&local) {
                match status {
                    DeinitStatus::Valid => {
                        self.emit_statement(StatementKind::Deinit {
                            place: Place::local(local),
                        });
                    }
                    DeinitStatus::MaybeMoved { flag } => {
                        self.emit_statement(StatementKind::DeinitIf {
                            place: Place::local(local),
                            flag: *flag,
                        });
                    }
                    DeinitStatus::Moved => {
                        // Already moved, no deinit needed
                    }
                }
            }
        }
    }

    /// Emit deinits for ALL scopes (for return/panic).
    ///
    /// Scopes are processed from innermost to outermost.
    pub fn emit_all_scope_deinits(&mut self) {
        // Collect scopes to avoid borrow issues
        let scopes: Vec<ScopeInfo> = self.scope_stack.iter().rev().cloned().collect();
        for scope in scopes {
            self.emit_scope_deinits(&scope);
        }
    }

    /// Emit deinits for scopes between current depth and target loop.
    ///
    /// Used for break/continue to clean up inner scopes before jumping.
    pub fn emit_deinits_to_loop(&mut self, loop_id: LoopId) {
        let target_depth = self
            .find_loop(loop_id)
            .map(|l| l.scope_depth)
            .unwrap_or(0);

        // Emit deinits for scopes from current down to (but not including) target
        let scopes: Vec<ScopeInfo> = self
            .scope_stack
            .iter()
            .skip(target_depth)
            .rev()
            .cloned()
            .collect();

        for scope in scopes {
            self.emit_scope_deinits(&scope);
        }
    }

    // ==========================================================================
    // Local Tracking for Deinit
    // ==========================================================================

    /// Register a local as declared in the current scope.
    ///
    /// If `needs_deinit` is true, the local will be tracked for deinit at scope exit.
    pub fn track_local(&mut self, local: Id<Local>, needs_deinit: bool) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.locals.push(local);
            if needs_deinit {
                scope.deinit_status.insert(local, DeinitStatus::Valid);
            }
        }
    }

    /// Mark a local as moved (no deinit needed).
    ///
    /// Searches all scopes from innermost to outermost.
    pub fn mark_moved(&mut self, local: Id<Local>) {
        for scope in self.scope_stack.iter_mut().rev() {
            if scope.deinit_status.contains_key(&local) {
                scope.deinit_status.insert(local, DeinitStatus::Moved);
                return;
            }
        }
    }

    /// Mark a local as maybe-moved (needs conditional deinit).
    ///
    /// Creates a deinit flag if one doesn't exist. Returns the flag local.
    pub fn mark_maybe_moved(&mut self, local: Id<Local>) -> Id<Local> {
        // First check if it already has a MaybeMoved status with a flag
        for scope in self.scope_stack.iter() {
            if let Some(DeinitStatus::MaybeMoved { flag }) = scope.deinit_status.get(&local) {
                return *flag;
            }
        }

        // Create a new flag
        let flag = self.create_deinit_flag();

        // Update status in the appropriate scope
        for scope in self.scope_stack.iter_mut().rev() {
            if scope.deinit_status.contains_key(&local) {
                scope
                    .deinit_status
                    .insert(local, DeinitStatus::MaybeMoved { flag });
                return flag;
            }
        }

        flag
    }

    /// Create a new deinit flag local (Bool type, initialized to true).
    fn create_deinit_flag(&mut self) -> Id<Local> {
        let name = format!("__deinit_flag_{}", self.deinit_flag_counter);
        self.deinit_flag_counter += 1;
        let ty_bool = self.mir.ty_bool();
        let flag = self.create_local(&name, ty_bool);

        // Initialize to true (needs deinit)
        self.emit_statement(StatementKind::SetDeinitFlag { flag, value: true });

        flag
    }

    /// Get the deinit status of a local, searching all scopes.
    pub fn get_deinit_status(&self, local: Id<Local>) -> Option<&DeinitStatus> {
        for scope in self.scope_stack.iter().rev() {
            if let Some(status) = scope.deinit_status.get(&local) {
                return Some(status);
            }
        }
        None
    }

    /// Update the deinit status of a local in the appropriate scope.
    pub fn update_deinit_status(&mut self, local: Id<Local>, status: DeinitStatus) {
        for scope in self.scope_stack.iter_mut().rev() {
            if scope.deinit_status.contains_key(&local) {
                scope.deinit_status.insert(local, status);
                return;
            }
        }
    }

    // ==========================================================================
    // Temporary Tracking
    // ==========================================================================

    /// Register a temporary for end-of-statement cleanup.
    ///
    /// Call this when creating a temp that holds a non-Copyable value.
    /// The temp is also added to the current scope's deinit tracking so that
    /// `mark_moved()` can properly update its status when it's consumed.
    pub fn track_statement_temp(&mut self, local: Id<Local>) {
        self.statement_temps.push(local);
        // Also track in current scope for move detection
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.deinit_status.insert(local, DeinitStatus::Valid);
        }
    }

    /// Emit deinits for statement temporaries and clear the list.
    ///
    /// Called at the end of each statement.
    pub fn emit_temp_deinits(&mut self) {
        if self.is_block_terminated() {
            self.statement_temps.clear();
            return;
        }

        // Deinit in reverse order
        for local in self.statement_temps.drain(..).rev().collect::<Vec<_>>() {
            // Check if temp was moved
            let status = self.get_deinit_status(local).cloned();
            match status {
                Some(DeinitStatus::Valid) | None => {
                    // If not tracked or still valid, deinit it
                    // (temps might not be in scope_stack if they don't need tracking)
                    self.emit_statement(StatementKind::Deinit {
                        place: Place::local(local),
                    });
                }
                Some(DeinitStatus::MaybeMoved { flag }) => {
                    self.emit_statement(StatementKind::DeinitIf {
                        place: Place::local(local),
                        flag,
                    });
                }
                Some(DeinitStatus::Moved) => {
                    // Already moved, no deinit needed
                }
            }
        }
    }

    // ==========================================================================
    // Type Queries for Deinit
    // ==========================================================================

    /// Check if a semantic type needs deinit at scope exit.
    ///
    /// A type needs deinit if:
    /// 1. It has a `DeinitBehavior` (custom destructor), AND
    /// 2. It is NOT Copyable (Copyable types are handled differently)
    pub fn type_needs_deinit(&self, ty: &kestrel_semantic_tree::ty::Ty) -> bool {
        match ty.kind() {
            TyKind::Struct { symbol, .. } => {
                let meta = symbol.metadata();

                // Check if it has deinit
                let has_deinit = meta.get_behavior::<DeinitBehavior>().is_some();
                if !has_deinit {
                    return false;
                }

                // Check if it's NOT copyable
                let is_copyable = meta
                    .get_behavior::<CopySemanticsBehavior>()
                    .map(|b: std::sync::Arc<CopySemanticsBehavior>| b.is_copyable())
                    .unwrap_or(true); // Default is copyable

                !is_copyable
            }
            // Enums with deinit - TODO: Phase 5.6
            TyKind::Enum { .. } => false,
            // Primitives, functions, references, etc. don't need deinit
            _ => false,
        }
    }

    // ==========================================================================
    // Branch Merging for Conditional Drops
    // ==========================================================================

    /// Merge deinit status from two branches (if/else, match arms).
    ///
    /// For each local that was tracked before the branch:
    /// - If moved in both branches → stays Moved
    /// - If valid in both branches → stays Valid
    /// - If moved in one but not other → becomes MaybeMoved
    ///
    /// Returns a list of (local, new_status) updates to apply to the parent scope.
    pub fn merge_branch_scopes(
        &mut self,
        then_scope: &ScopeInfo,
        else_scope: &ScopeInfo,
    ) -> Vec<(Id<Local>, DeinitStatus)> {
        // First, collect all the locals and their statuses without mutation
        let mut to_check: Vec<(Id<Local>, DeinitStatus, DeinitStatus, DeinitStatus)> = Vec::new();

        for parent_scope in &self.scope_stack {
            for (&local, parent_status) in &parent_scope.deinit_status {
                let then_status = then_scope
                    .deinit_status
                    .get(&local)
                    .cloned()
                    .unwrap_or_else(|| parent_status.clone());
                let else_status = else_scope
                    .deinit_status
                    .get(&local)
                    .cloned()
                    .unwrap_or_else(|| parent_status.clone());

                to_check.push((local, parent_status.clone(), then_status, else_status));
            }
        }

        // Now process, which may require mutation (creating flags)
        let mut updates = Vec::new();
        for (local, parent_status, then_status, else_status) in to_check {
            let new_status = self.merge_statuses(&then_status, &else_status, local);
            if new_status != parent_status {
                updates.push((local, new_status));
            }
        }

        updates
    }

    /// Merge two deinit statuses from different branches.
    fn merge_statuses(
        &mut self,
        a: &DeinitStatus,
        b: &DeinitStatus,
        _local: Id<Local>,
    ) -> DeinitStatus {
        match (a, b) {
            // Same status → keep it
            (DeinitStatus::Valid, DeinitStatus::Valid) => DeinitStatus::Valid,
            (DeinitStatus::Moved, DeinitStatus::Moved) => DeinitStatus::Moved,

            // If either is MaybeMoved, result is MaybeMoved (keep existing flag)
            (DeinitStatus::MaybeMoved { flag }, _) | (_, DeinitStatus::MaybeMoved { flag }) => {
                DeinitStatus::MaybeMoved { flag: *flag }
            }

            // One moved, one valid → MaybeMoved (create new flag)
            (DeinitStatus::Valid, DeinitStatus::Moved)
            | (DeinitStatus::Moved, DeinitStatus::Valid) => {
                // Create a flag for this local
                let flag = self.create_deinit_flag();
                DeinitStatus::MaybeMoved { flag }
            }
        }
    }

    /// Set a deinit flag to a specific value.
    ///
    /// Used when entering a branch where we know the move status.
    pub fn set_deinit_flag(&mut self, flag: Id<Local>, value: bool) {
        self.emit_statement(StatementKind::SetDeinitFlag { flag, value });
    }

    // ==========================================================================
    // Branch Merging Support for Conditional Deinits
    // ==========================================================================

    /// Snapshot the deinit status of all tracked locals in parent scopes.
    ///
    /// Call this before lowering a branch to capture the "before" state.
    /// Returns a map of local -> status for all tracked locals.
    pub fn snapshot_parent_deinit_statuses(&self) -> HashMap<Id<Local>, DeinitStatus> {
        let mut snapshot = HashMap::new();
        for scope in &self.scope_stack {
            for (&local, status) in &scope.deinit_status {
                snapshot.insert(local, status.clone());
            }
        }
        snapshot
    }

    /// Get the current deinit status of a local from the parent scopes.
    ///
    /// This returns the status without needing to exit the scope.
    pub fn get_current_deinit_status(&self, local: Id<Local>) -> Option<DeinitStatus> {
        for scope in self.scope_stack.iter().rev() {
            if let Some(status) = scope.deinit_status.get(&local) {
                return Some(status.clone());
            }
        }
        None
    }

    /// Apply status updates to parent scopes after branch merging.
    ///
    /// Takes a list of (local, new_status) pairs and updates the parent scopes.
    pub fn apply_merge_updates(&mut self, updates: Vec<(Id<Local>, DeinitStatus)>) {
        for (local, new_status) in updates {
            self.update_deinit_status(local, new_status);
        }
    }

    /// Restore deinit statuses from a snapshot.
    ///
    /// This is used to reset parent scope statuses after lowering one branch,
    /// before lowering the other branch. This ensures each branch sees the
    /// same "before" state.
    pub fn restore_deinit_statuses(&mut self, snapshot: &HashMap<Id<Local>, DeinitStatus>) {
        for (&local, status) in snapshot {
            self.update_deinit_status(local, status.clone());
        }
    }

    /// Create a deinit flag without initializing it.
    ///
    /// Returns the flag local. The caller is responsible for setting the initial value
    /// in the appropriate branches.
    pub fn create_deinit_flag_uninit(&mut self) -> Id<Local> {
        let name = format!("__deinit_flag_{}", self.deinit_flag_counter);
        self.deinit_flag_counter += 1;
        let ty_bool = self.mir.ty_bool();
        self.create_local(&name, ty_bool)
    }

    /// Merge deinit statuses from two branches for parent-scope locals.
    ///
    /// Given snapshots of status before entering branches and after each branch,
    /// this determines which locals need conditional deinit and creates flags as needed.
    ///
    /// Returns:
    /// - Updates to apply to parent scopes
    /// - Locals that need flag=false in then branch
    /// - Locals that need flag=true in then branch
    /// - Locals that need flag=false in else branch
    /// - Locals that need flag=true in else branch
    pub fn compute_branch_merge(
        &mut self,
        before: &HashMap<Id<Local>, DeinitStatus>,
        then_statuses: &HashMap<Id<Local>, DeinitStatus>,
        else_statuses: &HashMap<Id<Local>, DeinitStatus>,
    ) -> BranchMergeResult {
        let mut updates = Vec::new();
        let mut then_flag_false = Vec::new();
        let mut then_flag_true = Vec::new();
        let mut else_flag_false = Vec::new();
        let mut else_flag_true = Vec::new();

        for (&local, before_status) in before {
            let then_status = then_statuses.get(&local).unwrap_or(before_status);
            let else_status = else_statuses.get(&local).unwrap_or(before_status);

            // Check if there's divergence
            match (then_status, else_status) {
                (DeinitStatus::Valid, DeinitStatus::Valid) => {
                    // Both valid - no change needed
                }
                (DeinitStatus::Moved, DeinitStatus::Moved) => {
                    // Both moved - update parent to Moved
                    if *before_status != DeinitStatus::Moved {
                        updates.push((local, DeinitStatus::Moved));
                    }
                }
                (DeinitStatus::Valid, DeinitStatus::Moved) => {
                    // Moved in else, valid in then -> need conditional deinit
                    let flag = self.create_deinit_flag_uninit();
                    updates.push((local, DeinitStatus::MaybeMoved { flag }));
                    then_flag_true.push(flag); // then: still valid, needs deinit
                    else_flag_false.push(flag); // else: moved, no deinit
                }
                (DeinitStatus::Moved, DeinitStatus::Valid) => {
                    // Moved in then, valid in else -> need conditional deinit
                    let flag = self.create_deinit_flag_uninit();
                    updates.push((local, DeinitStatus::MaybeMoved { flag }));
                    then_flag_false.push(flag); // then: moved, no deinit
                    else_flag_true.push(flag); // else: still valid, needs deinit
                }
                // If either is already MaybeMoved, keep the flag
                (DeinitStatus::MaybeMoved { flag }, DeinitStatus::Valid) => {
                    then_flag_true.push(*flag); // might have been set in nested if
                    else_flag_true.push(*flag);
                }
                (DeinitStatus::Valid, DeinitStatus::MaybeMoved { flag }) => {
                    then_flag_true.push(*flag);
                    else_flag_true.push(*flag); // might have been set in nested if
                }
                (DeinitStatus::MaybeMoved { flag }, DeinitStatus::Moved) => {
                    else_flag_false.push(*flag);
                }
                (DeinitStatus::Moved, DeinitStatus::MaybeMoved { flag }) => {
                    then_flag_false.push(*flag);
                }
                (DeinitStatus::MaybeMoved { flag: f1 }, DeinitStatus::MaybeMoved { flag: f2 }) => {
                    // Both maybe moved - this is complex, use the first flag
                    // In practice, they should be the same flag if from the same source
                    if f1 != f2 {
                        // Different flags - this shouldn't happen in well-formed code
                        // but just in case, we keep f1
                    }
                    let _ = f2; // suppress warning
                }
            }
        }

        BranchMergeResult {
            updates,
            then_flag_false,
            then_flag_true,
            else_flag_false,
            else_flag_true,
        }
    }
}

/// Result of computing branch merge for deinit flags.
#[derive(Debug)]
pub struct BranchMergeResult {
    /// Updates to apply to parent scopes
    pub updates: Vec<(Id<Local>, DeinitStatus)>,
    /// Flags to set to false at end of then branch
    pub then_flag_false: Vec<Id<Local>>,
    /// Flags to set to true at end of then branch
    pub then_flag_true: Vec<Id<Local>>,
    /// Flags to set to false at end of else branch
    pub else_flag_false: Vec<Id<Local>>,
    /// Flags to set to true at end of else branch
    pub else_flag_true: Vec<Id<Local>>,
}
