//! Function body compilation — sets up locals, stack slots, and dispatches
//! to block compilation.
//!
//! Introduces `FunctionState` to encapsulate per-function state, replacing
//! the 10+ parameter lists in lib1.

use crate::block;
use crate::common::{self, is_aggregate_type};
use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::types;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags, StackSlotData, StackSlotKind};
use cranelift_codegen::ir::Value as CrValue;
use cranelift_codegen::verifier::verify_function;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::Module;
use kestrel_codegen2::substitute_type;
use kestrel_hecs::Entity;
use kestrel_mir::{
    BlockId, FunctionDef, FunctionKind, LocalId, MirBody, MirTy, PassingMode, Place, Rvalue,
    StatementKind,
};
use std::collections::{HashMap, HashSet};

/// Per-function compilation state.
///
/// Bundles all the context needed during block/statement/rvalue compilation,
/// eliminating the 10+ parameter passing pattern from lib1.
pub struct FunctionState<'a> {
    pub body: &'a MirBody,
    pub func_def: &'a FunctionDef,
    /// Type param substitutions for this instantiation.
    pub subst: HashMap<Entity, MirTy>,
    /// Protocol extension self type (if applicable).
    pub self_type: Option<MirTy>,
    /// MIR BlockId → Cranelift Block mapping.
    pub block_map: HashMap<BlockId, ir::Block>,
    /// Local variable → Cranelift Variable mapping (indexed by LocalId).
    pub local_vars: Vec<Variable>,
    /// Locals that need stack slots (address-taken or aggregate).
    pub stack_locals: HashSet<LocalId>,
    /// Whether this is the main function (special return ABI).
    pub is_main: bool,
    /// Pointer for sret (struct return), if applicable.
    pub sret_ptr: Option<CrValue>,
}

/// Build a substitution map from function type params and concrete type args.
pub fn build_subst(func: &FunctionDef, type_args: &[MirTy]) -> HashMap<Entity, MirTy> {
    func.type_params
        .iter()
        .zip(type_args.iter())
        .map(|(tp, arg)| (tp.entity, arg.clone()))
        .collect()
}

/// Compile a function body into Cranelift IR.
pub fn compile_function(
    ctx: &mut CodegenContext,
    func_def: &FunctionDef,
    func_id: cranelift_module::FuncId,
    sig: &ir::Signature,
    subst: &HashMap<Entity, MirTy>,
    self_type: Option<&MirTy>,
    mangled_name: &str,
) -> Result<(), CodegenError> {
    let body = func_def.body.as_ref().unwrap();
    let ptr_ty = common::ptr_type(ctx.target);

    // Compute these before creating the builder (avoids borrow conflicts)
    let ret_ty = substitute_type(&func_def.ret, subst);
    let is_main = ctx.is_main_function(func_def);
    let use_sret = !is_main && common::needs_sret(&ret_ty);

    let mut cl_func = ir::Function::with_name_signature(
        ir::UserFuncName::user(0, 0),
        sig.clone(),
    );

    let mut func_builder_ctx = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut cl_func, &mut func_builder_ctx);

    // Collect address-taken locals
    let stack_locals = collect_address_taken_locals(body, subst);

    // Create Cranelift blocks for all MIR blocks
    let mut block_map = HashMap::new();
    for (i, _) in body.blocks.iter().enumerate() {
        let cl_block = builder.create_block();
        block_map.insert(BlockId::new(i), cl_block);
    }

    // Set up entry block params
    let entry_cl = block_map[&body.entry];
    builder.append_block_params_for_function_params(entry_cl);
    builder.switch_to_block(entry_cl);

    // Declare Cranelift variables for all locals
    // In cranelift 0.129, declare_var(type) returns a Variable automatically
    let mut local_vars = Vec::with_capacity(body.locals.len());
    for (i, local) in body.locals.iter().enumerate() {
        let ty = substitute_type(&local.ty, subst);
        let cl_ty = if is_aggregate_type(&ty) || stack_locals.contains(&LocalId::new(i)) {
            ptr_ty // Aggregates and address-taken locals store pointers to stack slots
        } else {
            types::translate_type(&ty, ctx.target)
        };
        let var = builder.declare_var(cl_ty);
        local_vars.push(var);
    }

    // Initialize sret pointer
    let param_offset = if use_sret { 1 } else { 0 };
    let sret_ptr = if use_sret {
        Some(builder.block_params(entry_cl)[0])
    } else {
        None
    };

    // Initialize parameters from entry block params
    for (param_idx, param) in func_def.params.iter().enumerate() {
        let local_id = param.local;
        let cl_param = builder.block_params(entry_cl)[param_idx + param_offset];
        let ty = substitute_type(&param.ty, subst);

        if is_aggregate_type(&ty) || stack_locals.contains(&local_id) {
            // Aggregate or address-taken: allocate a stack slot, copy the value
            let layout = ctx.layouts.layout_of(&ty);
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                layout.size as u32,
                common::align_to_shift(layout.align),
            ));
            let addr = builder.ins().stack_addr(ptr_ty, slot, Offset32::new(0));

            if is_aggregate_type(&ty) {
                // Parameter is already a pointer; copy the data
                common::copy_aggregate(&mut builder, &mut ctx.layouts, &ty, addr, cl_param);
            } else {
                // Scalar that's address-taken: store the value in the slot
                builder.ins().store(MemFlags::new(), cl_param, addr, Offset32::new(0));
            }

            builder.def_var(local_vars[local_id.index()], addr);
        } else {
            builder.def_var(local_vars[local_id.index()], cl_param);
        }
    }

    // Initialize non-parameter locals that need stack slots
    for (i, local) in body.locals.iter().enumerate() {
        let local_id = LocalId::new(i);
        // Skip params (already initialized above)
        if i < body.param_count {
            continue;
        }

        let ty = substitute_type(&local.ty, subst);
        if is_aggregate_type(&ty) || stack_locals.contains(&local_id) {
            let layout = ctx.layouts.layout_of(&ty);
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                layout.size as u32,
                common::align_to_shift(layout.align),
            ));
            let addr = builder.ins().stack_addr(ptr_ty, slot, Offset32::new(0));
            common::zero_memory(&mut builder, addr, layout.size, ptr_ty);
            builder.def_var(local_vars[local_id.index()], addr);
        }
    }

    // Build function state
    let state = FunctionState {
        body,
        func_def,
        subst: subst.clone(),
        self_type: self_type.cloned(),
        block_map,
        local_vars,
        stack_locals,
        is_main,
        sret_ptr,
    };

    // Compile all blocks
    for (i, _mir_block) in body.blocks.iter().enumerate() {
        let block_id = BlockId::new(i);
        let cl_block = state.block_map[&block_id];

        // Switch to block (entry block already switched above)
        if i > 0 {
            builder.switch_to_block(cl_block);
        }

        block::compile_block(ctx, &state, &mut builder, block_id)?;
    }

    // Seal all blocks (SSA construction needs all predecessors known)
    builder.seal_all_blocks();
    builder.finalize();

    // Verify the function IR
    if let Err(errors) = verify_function(&cl_func, ctx.isa.as_ref()) {
        return Err(CodegenError::FunctionCompilation {
            name: mangled_name.to_string(),
            source: Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("verification failed: {errors}"),
            )),
        });
    }

    // Compile and define
    let mut cl_ctx = cranelift_codegen::Context::for_function(cl_func);
    cl_ctx
        .compile(ctx.isa.as_ref(), &mut Default::default())
        .map_err(|e| CodegenError::FunctionCompilation {
            name: mangled_name.to_string(),
            source: Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("{e:?}"),
            )),
        })?;

    ctx.cl_module
        .define_function(func_id, &mut cl_ctx)
        .map_err(|e| CodegenError::FunctionDefinition {
            name: mangled_name.to_string(),
            source: e,
        })?;

    Ok(())
}

/// Scan the function body to find locals whose addresses are taken.
///
/// Locals are address-taken if they appear in:
/// - `Rvalue::Ref(place)` or `Rvalue::RefMut(place)`
/// - Call arguments with `PassingMode::Ref` or `PassingMode::MutRef`
fn collect_address_taken_locals(
    body: &MirBody,
    subst: &HashMap<Entity, MirTy>,
) -> HashSet<LocalId> {
    let mut result = HashSet::new();

    for block in &body.blocks {
        for stmt in &block.stmts {
            match &stmt.kind {
                StatementKind::Assign { rvalue, .. } => match rvalue {
                    Rvalue::Ref(place) | Rvalue::RefMut(place) => {
                        if let Some(id) = place.root_local() {
                            let ty = substitute_type(&body.locals[id.index()].ty, subst);
                            if !is_aggregate_type(&ty) {
                                result.insert(id);
                            }
                        }
                    }
                    _ => {}
                },
                StatementKind::Call { args, .. } => {
                    for arg in args {
                        if matches!(arg.mode, PassingMode::Ref | PassingMode::MutRef) {
                            if let kestrel_mir::Value::Place(place) = &arg.value {
                                if let Some(id) = place.root_local() {
                                    let ty =
                                        substitute_type(&body.locals[id.index()].ty, subst);
                                    if !is_aggregate_type(&ty) {
                                        result.insert(id);
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    result
}
