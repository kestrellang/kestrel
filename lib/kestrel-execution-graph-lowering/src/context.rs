//! Lowering context - holds all state during the lowering pass.

use std::collections::HashMap;

use kestrel_execution_graph::{
    BasicBlock, Block, Callee, Function, Id, Immediate, Local, MirContext, Place, QualifiedName,
    Rvalue, StatementKind, Terminator, TerminatorKind, Ty, TypeParam, Value,
};
use kestrel_reporting::{Diagnostic, IntoDiagnostic};
use kestrel_semantic_model::SemanticModel;
use kestrel_semantic_tree::expr::LoopId;
use kestrel_semantic_tree::symbol::local::LocalId;
use semantic_tree::symbol::SymbolId;

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
}

/// The central context for lowering semantic tree to MIR.
///
/// This struct holds all state needed during the lowering pass, including:
/// - Reference to the semantic model (source)
/// - The MIR context being built (destination)
/// - Current function state
/// - Local variable mappings
/// - Loop stack for break/continue
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

    /// The current block being built.
    current_block: Option<Id<Block>>,

    /// Diagnostics collected during lowering.
    diagnostics: Vec<Diagnostic<usize>>,

    /// Counter for generating unique temporary names.
    temp_counter: u32,
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
            current_block: None,
            diagnostics: Vec::new(),
            temp_counter: 0,
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
        self.current_block = None;
        self.temp_counter = 0;
    }

    /// Exit the current function.
    pub fn exit_function(&mut self) {
        self.current_function = None;
        self.local_map.clear();
        self.loop_stack.clear();
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
    pub fn push_loop(&mut self, info: LoopInfo) {
        self.loop_stack.push(info);
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
    pub fn emit_call(
        &mut self,
        dest: Place,
        callee: Callee,
        args: Vec<Value>,
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
    pub fn emit_call_unit(&mut self, callee: Callee, args: Vec<Value>) {
        self.emit_statement(StatementKind::Call { callee, args });
    }
}
