//! Function body compilation — sets up locals, stack slots, and dispatches
//! to block compilation.
//!
//! Introduces `FunctionState` to encapsulate per-function state, replacing
//! the 10+ parameter lists in lib1.

use crate::block;
use crate::common::{self, is_aggregate};
use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::types;
use cranelift_codegen::ir::Value as CrValue;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags, StackSlotData, StackSlotKind};
use cranelift_codegen::verifier::verify_function;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::Module;
use kestrel_codegen2::{LayoutCache, substitute_type_with_self};
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

/// Augment a substitution map with associated type resolutions.
///
/// Scans the function body for `Named { entity, type_args: [] }` types that are
/// protocol associated types (e.g., `Iterable.Iter`) and resolves them through
/// the witness table using the concrete types from the subst map.
pub fn resolve_assoc_type_substs(
    module: &kestrel_mir::MirModule,
    func: &FunctionDef,
    subst: &mut HashMap<Entity, MirTy>,
    self_type: Option<&MirTy>,
) {
    use crate::monomorphize::witness;

    // Collect all Named entities from the function body that might be associated types
    let mut candidate_entities: Vec<Entity> = Vec::new();
    collect_named_entities_from_func(func, &mut candidate_entities);

    let debug = std::env::var("DEBUG_ASSOC").is_ok()
        && (func.name.contains("min") || func.name.contains("closure"));
    if debug {
        eprintln!(
            "\n=== resolve_assoc_type_substs for {} ===\n  self_type={:?}\n  initial subst={:?}",
            func.name, self_type, subst
        );
    }

    for entity in candidate_entities {
        if subst.contains_key(&entity) {
            continue; // Already resolved
        }

        let name = module.resolve_name(entity);
        if name == "<unknown>" {
            continue;
        }

        if debug {
            eprintln!("  candidate entity {:?} name={}", entity, name);
        }

        // Check if this entity's name matches <protocol_name>.<assoc_type_name>
        // for any known protocol
        for proto_def in &module.protocols {
            for assoc in &proto_def.associated_types {
                let expected = format!("{}.{}", proto_def.name, assoc.name);
                if name == expected {
                    // Candidate concrete types to try, in priority order:
                    // concrete types already in `subst` (method-level type
                    // params like `I` in `init[I](from: I)` — the body's
                    // `I.Item` must resolve via `I`, not via the enclosing
                    // `Self`). Fall back to the caller-supplied `self_type`
                    // for protocol extension methods whose body has no type
                    // params of its own (e.g. `extend Iterator { collect }`),
                    // where the only concrete witness to query is `Self`.
                    let subst_candidates = subst.values();
                    let self_candidate = self_type.into_iter();
                    for concrete in subst_candidates.chain(self_candidate) {
                        if debug {
                            eprintln!(
                                "    trying resolve {}.{} with concrete={:?}",
                                proto_def.name, assoc.name, concrete
                            );
                        }
                        match witness::resolve_associated_type(
                            module,
                            proto_def.entity,
                            concrete,
                            &assoc.name,
                        ) {
                            Ok(resolved) => {
                                if debug {
                                    eprintln!("    ✓ resolved to {:?}", resolved);
                                }
                                subst.insert(entity, resolved);
                                break;
                            },
                            Err(e) => {
                                if debug {
                                    eprintln!("    ✗ failed: {:?}", e);
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}

/// Collect all entities from Named types in a function's signature and body.
fn collect_named_entities_from_func(func: &FunctionDef, out: &mut Vec<Entity>) {
    // Signature types
    collect_named_entities_from_ty(&func.ret, out);
    for param in &func.params {
        collect_named_entities_from_ty(&param.ty, out);
    }

    // Body local types and callee types
    if let Some(body) = &func.body {
        for local in &body.locals {
            collect_named_entities_from_ty(&local.ty, out);
        }
        for block in &body.blocks {
            for stmt in &block.stmts {
                match &stmt.kind {
                    StatementKind::Call { callee, .. } => {
                        collect_named_entities_from_callee(callee, out);
                    },
                    StatementKind::Assign { rvalue, .. } => {
                        collect_named_entities_from_rvalue(rvalue, out);
                    },
                    _ => {},
                }
            }
        }
    }
}

fn collect_named_entities_from_ty(ty: &MirTy, out: &mut Vec<Entity>) {
    match ty {
        MirTy::Named { entity, type_args } => {
            if type_args.is_empty() {
                out.push(*entity);
            }
            for arg in type_args {
                collect_named_entities_from_ty(arg, out);
            }
        },
        MirTy::Pointer(inner) | MirTy::Ref(inner) | MirTy::RefMut(inner) => {
            collect_named_entities_from_ty(inner, out);
        },
        MirTy::Tuple(elems) => {
            for e in elems {
                collect_named_entities_from_ty(e, out);
            }
        },
        MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
            for p in params {
                collect_named_entities_from_ty(p, out);
            }
            collect_named_entities_from_ty(ret, out);
        },
        MirTy::AssociatedProjection { base, .. } => {
            collect_named_entities_from_ty(base, out);
        },
        _ => {},
    }
}

fn collect_named_entities_from_callee(callee: &kestrel_mir::Callee, out: &mut Vec<Entity>) {
    match callee {
        kestrel_mir::Callee::Witness {
            self_type,
            method_type_args,
            ..
        } => {
            collect_named_entities_from_ty(self_type, out);
            for arg in method_type_args {
                collect_named_entities_from_ty(arg, out);
            }
        },
        kestrel_mir::Callee::Direct {
            self_type,
            type_args,
            ..
        } => {
            if let Some(st) = self_type {
                collect_named_entities_from_ty(st, out);
            }
            for arg in type_args {
                collect_named_entities_from_ty(arg, out);
            }
        },
        _ => {},
    }
}

fn collect_named_entities_from_rvalue(rvalue: &Rvalue, out: &mut Vec<Entity>) {
    match rvalue {
        Rvalue::Construct { ty, .. } => {
            collect_named_entities_from_ty(ty, out);
        },
        Rvalue::EnumVariant { enum_ty, .. } => {
            collect_named_entities_from_ty(enum_ty, out);
        },
        _ => {},
    }
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
    let ret_ty = substitute_type_with_self(&func_def.ret, subst, self_type, ctx.module);
    let is_main = ctx.is_main_function(func_def);
    let use_sret = !is_main && common::needs_sret(&ret_ty, &mut ctx.layouts);

    let mut cl_func = ir::Function::with_name_signature(ir::UserFuncName::user(0, 0), sig.clone());

    let mut func_builder_ctx = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut cl_func, &mut func_builder_ctx);

    // Collect address-taken locals
    let stack_locals = collect_address_taken_locals(body, subst, self_type, &mut ctx.layouts);

    // `mutating` (InOut) and aggregate `In` parameters receive a pointer to
    // caller storage. The local Cranelift Variable holds that pointer; reads/
    // writes deref through it. This matches lib1's Borrow→Ref convention and
    // ensures `lang.ptr_to(in_param)` returns the caller's address, not a
    // dangling pointer to a callee-local copy.
    let mut inout_param_locals: HashSet<LocalId> = HashSet::new();
    for p in &func_def.params {
        let pty = substitute_type_with_self(&p.ty, subst, self_type, ctx.module);
        let pass_as_ptr = matches!(p.mode, kestrel_mir::ParamMode::InOut)
            || (matches!(p.mode, kestrel_mir::ParamMode::In)
                && is_aggregate(&pty, &mut ctx.layouts));
        if pass_as_ptr {
            inout_param_locals.insert(p.local);
        }
    }

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
        let ty = substitute_type_with_self(&local.ty, subst, self_type, ctx.module);
        let local_id = LocalId::new(i);
        let cl_ty = if is_aggregate(&ty, &mut ctx.layouts)
            || stack_locals.contains(&local_id)
            || inout_param_locals.contains(&local_id)
        {
            ptr_ty // Aggregates, address-taken locals, and InOut params store pointers
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
        let ty = substitute_type_with_self(&param.ty, subst, self_type, ctx.module);

        if inout_param_locals.contains(&local_id) {
            // InOut or aggregate In param: cl_param IS the caller's pointer.
            // Bind directly — no copy. For InOut this preserves write-back; for
            // In aggregates this matches lib1's Borrow→Ref convention so that
            // `lang.ptr_to(value)` returns the caller's address.
            builder.def_var(local_vars[local_id.index()], cl_param);
        } else if is_aggregate(&ty, &mut ctx.layouts) || stack_locals.contains(&local_id) {
            // Aggregate or address-taken: allocate a stack slot, copy the value
            let layout = ctx.layouts.layout_of(&ty);
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                layout.size as u32,
                common::align_to_shift(layout.align),
            ));
            let addr = builder.ins().stack_addr(ptr_ty, slot, Offset32::new(0));

            if is_aggregate(&ty, &mut ctx.layouts) {
                // Large aggregate: parameter is a pointer; copy the data
                common::copy_aggregate(&mut builder, &mut ctx.layouts, &ty, addr, cl_param);
            } else {
                // Scalar that's address-taken: store the value in the slot
                builder
                    .ins()
                    .store(MemFlags::new(), cl_param, addr, Offset32::new(0));
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

        let ty = substitute_type_with_self(&local.ty, subst, self_type, ctx.module);
        if is_aggregate(&ty, &mut ctx.layouts) || stack_locals.contains(&local_id) {
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

    // Skip verification in release — Cranelift's verifier can be deeply recursive.
    // Verify only in debug builds.
    #[cfg(debug_assertions)]
    if let Err(errors) = verify_function(&cl_func, ctx.isa.as_ref()) {
        eprintln!("CRANELIFT VERIFY FAILED: {mangled_name}\n{errors}");
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
    stacker::maybe_grow(256 * 1024, 8 * 1024 * 1024, || {
        cl_ctx.compile(ctx.isa.as_ref(), &mut Default::default())
    })
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
    self_type: Option<&MirTy>,
    layouts: &mut LayoutCache,
) -> HashSet<LocalId> {
    let mut result = HashSet::new();

    for block in &body.blocks {
        for stmt in &block.stmts {
            match &stmt.kind {
                StatementKind::Assign { rvalue, .. } => match rvalue {
                    Rvalue::Ref(place) | Rvalue::RefMut(place) => {
                        if let Some(id) = place.root_local() {
                            let ty = substitute_type_with_self(
                                &body.locals[id.index()].ty,
                                subst,
                                self_type,
                                layouts.module(),
                            );
                            if !is_aggregate(&ty, layouts) {
                                result.insert(id);
                            }
                        }
                    },
                    _ => {},
                },
                StatementKind::Call { args, .. } => {
                    for arg in args {
                        if matches!(arg.mode, PassingMode::Ref | PassingMode::MutRef) {
                            if let kestrel_mir::Value::Place(place) = &arg.value {
                                if let Some(id) = place.root_local() {
                                    let ty = substitute_type_with_self(
                                        &body.locals[id.index()].ty,
                                        subst,
                                        self_type,
                                        layouts.module(),
                                    );
                                    if !is_aggregate(&ty, layouts) {
                                        result.insert(id);
                                    }
                                }
                            }
                        }
                    }
                },
                _ => {},
            }
        }
    }

    result
}
