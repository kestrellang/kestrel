//! Function compilation.

use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::monomorphize::Substitution;
use crate::types::translate_type_with_subst;

use kestrel_execution_graph::{
    Block, FunctionDef, Id, Local, LocalDef, MirTy, Place, PlaceKind, Rvalue, StatementKind, Ty,
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

fn is_aggregate_type(ctx: &CodegenContext<'_>, ty_id: Id<Ty>) -> bool {
    matches!(
        ctx.mir.ty(ty_id),
        MirTy::Tuple(_) | MirTy::Named { .. } | MirTy::Str | MirTy::FuncThick { .. }
    )
}

/// Compile a function body.
pub fn compile_function_body(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
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
        compile_blocks(ctx, func_def, subst, &mut builder, is_main)?;
    }

    builder.finalize();
    Ok(())
}

/// Compile all blocks in a function.
fn compile_blocks(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    builder: &mut FunctionBuilder<'_>,
    is_main: bool,
) -> Result<(), CodegenError> {
    let mir_entry_block = func_def.entry_block.unwrap();
    let concrete_ret = subst
        .apply_ty_readonly(ctx.mir, func_def.ret)
        .unwrap_or(func_def.ret);
    let ret_mir_ty = ctx.mir.ty(concrete_ret);
    let needs_sret =
        !is_main && !matches!(ret_mir_ty, MirTy::Unit) && is_aggregate_type(ctx, concrete_ret);

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
        let cl_type = translate_type_with_subst(ctx.mir, local_def.ty, ctx.target, subst);
        builder.declare_var(var, cl_type);
        local_map.insert(local_id, var);
    }

    // Initialize parameters - switch to entry block and copy params to locals
    let entry_block = block_map[&mir_entry_block];
    builder.switch_to_block(entry_block);

    let params = builder.block_params(entry_block).to_vec();
    let mut param_offset = 0;
    let mut sret_ptr = None;
    if needs_sret {
        sret_ptr = params.get(0).copied();
        param_offset = 1;
    }

    for (i, &param_id) in func_def.params.iter().enumerate() {
        // Use the param's direct local field instead of searching by name
        let param = &ctx.mir.params[param_id];
        let local_id = param.local;
        if let Some(&var) = local_map.get(&local_id) {
            builder.def_var(var, params[i + param_offset]);
        }
    }

    // Collect parameter local IDs for filtering
    let param_local_ids: HashSet<Id<Local>> = func_def
        .params
        .iter()
        .map(|&p| ctx.mir.params[p].local)
        .collect();

    // Allocate stack slots for ALL aggregate-typed non-parameter locals.
    // This ensures they have valid addresses when Ref/RefMut is taken.
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    for &local_id in &func_def.locals {
        // Skip parameters - they're already initialized from function args
        if param_local_ids.contains(&local_id) {
            continue;
        }

        let local_def = ctx.mir.local(local_id);
        let concrete_ty = subst
            .apply_ty_readonly(ctx.mir, local_def.ty)
            .unwrap_or(local_def.ty);
        if is_aggregate_type(ctx, concrete_ty) {
            // Allocate a stack slot for this local
            let layout = ctx.layouts.layout_of(concrete_ty);
            let size = if layout.size == 0 { 1 } else { layout.size };
            let align = if layout.align == 0 { 1 } else { layout.align };
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                size as u32,
                align as u8,
            ));
            let addr = builder.ins().stack_addr(ptr_type, slot, 0);

            // Initialize the Variable to point to the stack slot
            let var = local_map[&local_id];
            builder.def_var(var, addr);
        }
    }

    // Compile each block (but don't seal yet - we need all predecessors first)
    for (i, &block_id) in func_def.blocks.iter().enumerate() {
        let cl_block = block_map[&block_id];
        if block_id != mir_entry_block {
            builder.switch_to_block(cl_block);
        }

        let next_block_id = func_def.blocks.get(i + 1).copied();

        crate::block::compile_block(
            ctx,
            func_def,
            subst,
            block_id,
            next_block_id,
            builder,
            &block_map,
            &local_map,
            is_main,
            sret_ptr,
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
