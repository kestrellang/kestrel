//! Function compilation.

use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::types::translate_type;

use kestrel_execution_graph::{
    Block, FunctionDef, Id, Local, LocalDef, Place, PlaceKind, Rvalue, StatementKind,
};

use cranelift_codegen::ir::types as cl_types;
use cranelift_codegen::ir::{
    Function as CraneliftFunction, InstBuilder, MemFlags, StackSlotData, StackSlotKind, Value,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};

use std::collections::{HashMap, HashSet};

/// Collect locals that have their address taken (used in Ref/RefMut).
/// These locals must be stack-allocated, not register-allocated.
fn collect_address_taken_locals(
    ctx: &CodegenContext<'_>,
    func_def: &FunctionDef,
) -> HashSet<Id<Local>> {
    let mut result = HashSet::new();

    for &block_id in &func_def.blocks {
        let block = ctx.mir.block(block_id);
        for &stmt_id in &block.statements {
            let stmt = ctx.mir.statement(stmt_id);
            if let StatementKind::Assign { rvalue, .. } = &stmt.kind {
                match rvalue {
                    Rvalue::Ref(place) | Rvalue::RefMut(place) => {
                        // Find the root local of this place
                        if let Some(local_id) = get_root_local(place) {
                            result.insert(local_id);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    result
}

/// Get the root local of a place expression.
fn get_root_local(place: &Place) -> Option<Id<Local>> {
    match &place.kind {
        PlaceKind::Local(local_id) => Some(*local_id),
        PlaceKind::Field { parent, .. } => get_root_local(parent),
        PlaceKind::Index { parent, .. } => get_root_local(parent),
        PlaceKind::Downcast { parent, .. } => get_root_local(parent),
        PlaceKind::Deref(inner) => get_root_local(inner),
    }
}

/// Compile a function body.
pub fn compile_function_body(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    cl_func: &mut CraneliftFunction,
    is_main: bool,
) -> Result<(), CodegenError> {
    let mut builder_ctx = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(cl_func, &mut builder_ctx);

    // For now, just return a constant if no blocks
    if func_def.entry_block.is_none() {
        // Create a simple entry block with just a return
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        // Return unit (void)
        let ret_ty = ctx.mir.ty(func_def.ret);
        if matches!(ret_ty, kestrel_execution_graph::MirTy::Unit) {
            if is_main {
                // main() must return 0 for success exit code
                let zero = builder.ins().iconst(cl_types::I64, 0);
                builder.ins().return_(&[zero]);
            } else {
                builder.ins().return_(&[]);
            }
        } else {
            // Return 0 as placeholder
            let zero = builder.ins().iconst(cl_types::I64, 0);
            builder.ins().return_(&[zero]);
        }
    } else {
        // Compile the actual function body
        compile_blocks(ctx, func_def, &mut builder, is_main)?;
    }

    builder.finalize();
    Ok(())
}

/// Compile all blocks in a function.
fn compile_blocks(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    builder: &mut FunctionBuilder<'_>,
    is_main: bool,
) -> Result<(), CodegenError> {
    let mir_entry_block = func_def.entry_block.unwrap();

    // Create Cranelift blocks for each MIR block
    // The entry block gets special handling - it needs function parameters
    let mut block_map: HashMap<Id<Block>, cranelift_codegen::ir::Block> = HashMap::new();

    for &block_id in &func_def.blocks {
        let cl_block = builder.create_block();
        if block_id == mir_entry_block {
            // Entry block gets the function parameters
            builder.append_block_params_for_function_params(cl_block);
        }
        block_map.insert(block_id, cl_block);
    }

    // Map locals to variables
    let mut local_map: HashMap<Id<Local>, Variable> = HashMap::new();
    for (i, &local_id) in func_def.locals.iter().enumerate() {
        let var = Variable::from_u32(i as u32);
        let local_def = ctx.mir.local(local_id);
        let cl_type = translate_type(ctx.mir, local_def.ty, ctx.target);
        builder.declare_var(var, cl_type);
        local_map.insert(local_id, var);
    }

    // Initialize parameters - switch to entry block and copy params to locals
    let entry_block = block_map[&mir_entry_block];
    builder.switch_to_block(entry_block);

    let params = builder.block_params(entry_block).to_vec();
    for (i, &param_id) in func_def.params.iter().enumerate() {
        // Find the local that corresponds to this param
        for &local_id in &func_def.locals {
            let local_def = ctx.mir.local(local_id);
            if local_def.name == ctx.mir.params[param_id].name {
                let var = local_map[&local_id];
                builder.def_var(var, params[i]);
                break;
            }
        }
    }

    // Compile each block (but don't seal yet - we need all predecessors first)
    for &block_id in &func_def.blocks {
        let cl_block = block_map[&block_id];
        if block_id != mir_entry_block {
            builder.switch_to_block(cl_block);
        }

        crate::block::compile_block(
            ctx, func_def, block_id, builder, &block_map, &local_map, is_main,
        )?;
    }

    // Seal all blocks after all code has been emitted
    // This is necessary because Cranelift's SSA construction needs to know all
    // predecessors before sealing a block.
    for &block_id in &func_def.blocks {
        let cl_block = block_map[&block_id];
        builder.seal_block(cl_block);
    }

    Ok(())
}
