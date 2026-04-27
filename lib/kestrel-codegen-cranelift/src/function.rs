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
use kestrel_codegen::{LayoutCache, substitute_type_with_self};
use kestrel_hecs::Entity;
use kestrel_mir::{
    BlockId, FunctionDef, LocalId, MirBody, MirTy, PassingMode, Rvalue,
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

    // Recover conformance-introduced free TypeParams + correct assoc-type
    // entity bindings. When a function's body dispatches `Callee::Witness`
    // against a generic-bound `I: Proto[A, ..]` and the conformance is
    // `extend C: Proto[T_ext, ..]`, the conformance's free `T_ext` only
    // gets bound at the call site via the protocol args. For the OUTER
    // monomorph that needs to lay out `I.Output` (which resolves through
    // `C: Proto[T_ext]` to `Param(T_ext)`), `T_ext` must be in `subst`.
    //
    // Equally important: the existing first-match-by-name loop above can
    // route a protocol's `Yield`/`Output`/etc. assoc through the WRONG
    // conformance when more than one entity in `subst.values()` conforms
    // to the same protocol (e.g. `subst` has both `Int64` and `Range[Int64]`
    // for an `Array[Int64].subscript[I=Range[Int64]]` instance — both
    // conform to `ArrayIndex`, and the first picked wins). Walk the body's
    // witness calls and override the assoc-entity bindings with the
    // witness's `type_bindings` for the call site's actual `self_type`.
    if let Some(body) = &func.body {
        for block in &body.blocks {
            for stmt in &block.stmts {
                let kestrel_mir::StatementKind::Call { callee, .. } = &stmt.kind else {
                    continue;
                };
                let kestrel_mir::Callee::Witness {
                    protocol,
                    self_type: callee_self,
                    method_type_args,
                    ..
                } = callee
                else {
                    continue;
                };
                bind_witness_protocol_args(
                    module,
                    *protocol,
                    callee_self,
                    method_type_args,
                    self_type,
                    subst,
                    func,
                );
            }
        }
    }
}

/// For a single `Callee::Witness` site in a function body, substitute the
/// site's `self_type` and `method_type_args` using the current `subst`, find
/// the conformance witness for `(protocol, substituted self_type)`, and
/// pattern-bind the witness's wildcard `protocol_type_args` against the
/// substituted call-site protocol args. Newly recovered bindings get inserted
/// into `subst`. No-op if the site's self_type doesn't substitute to a
/// concrete type or no matching witness is found.
fn bind_witness_protocol_args(
    module: &kestrel_mir::MirModule,
    protocol: Entity,
    callee_self: &MirTy,
    method_type_args: &[MirTy],
    outer_self: Option<&MirTy>,
    subst: &mut HashMap<Entity, MirTy>,
    func: &FunctionDef,
) {
    use kestrel_codegen::substitute_type_with_self;

    let sub_self = substitute_type_with_self(callee_self, subst, outer_self, module);
    let sub_args: Vec<MirTy> = method_type_args
        .iter()
        .map(|t| substitute_type_with_self(t, subst, outer_self, module))
        .collect();

    let proto_param_count = module
        .protocols
        .iter()
        .find(|p| p.entity == protocol)
        .map(|p| p.type_params.len())
        .unwrap_or(0);
    if proto_param_count == 0 {
        return;
    }
    let call_proto_args = sub_args.get(..proto_param_count).unwrap_or(&[]);
    if call_proto_args.iter().any(ty_has_typeparam) {
        return; // Need fully-concrete call args to bind extension free params.
    }

    let mut chosen: Option<&kestrel_mir::WitnessDef> = None;
    for w in &module.witnesses {
        if w.protocol != protocol {
            continue;
        }
        let mut tmp = HashMap::new();
        if !witness_pattern_matches(&w.implementing_type, &sub_self, &mut tmp) {
            continue;
        }
        chosen = Some(w);
        break;
    }
    let Some(witness) = chosen else { return };

    for (witness_arg, call_arg) in witness
        .protocol_type_args
        .values()
        .zip(call_proto_args.iter())
    {
        bind_pattern_into_subst(witness_arg, call_arg, subst);
    }

    // Bind the protocol's associated-type aliases via the witness's
    // `type_bindings`. Without this, the existing first-match-by-name loop
    // above can pick the wrong conformance (e.g., it sees `Int64` and
    // `Range[Int64]` both in `subst.values()` and either one nominally
    // satisfies the assoc lookup, but only the one that matches the actual
    // call site is correct). Look up each assoc-name's TypeAlias entity on
    // the protocol and override `subst[entity]` with the witness's binding.
    let proto_def = module.protocols.iter().find(|p| p.entity == protocol);
    let Some(proto_def) = proto_def else { return };
    for assoc in &proto_def.associated_types {
        let Some(bound) = witness.type_bindings.get(&assoc.name) else {
            continue;
        };
        // Locate the protocol-level assoc-type entity by name. The protocol
        // owns the alias as a child; look it up by walking children and
        // matching `{Protocol}.{Name}` against `module.resolve_name`.
        // Locate the protocol-level assoc-type entity by walking the
        // function's candidate Named entities and matching the qualified
        // name (`Protocol.Assoc`). MirModule has no central name→entity
        // index; the candidate set is what's referenced by the function's
        // own signature/body, so the alias entity is in there if it's
        // mentioned anywhere in the surface this monomorph touches.
        let qualified = format!("{}.{}", proto_def.name, assoc.name);
        let mut candidates_local: Vec<Entity> = Vec::new();
        collect_named_entities_from_func(func, &mut candidates_local);
        let entity_for_assoc = candidates_local
            .into_iter()
            .find(|&cand| module.resolve_name(cand) == qualified);
        if let Some(e) = entity_for_assoc {
            // The witness's binding may reference the conformance's free
            // TypeParams (e.g. `Yield = Slice[T_ext]`). Those resolve via
            // the chained substitution + TypeParam recursion in
            // `substitute_type_inner` once the proto-args pattern bindings
            // above populate them.
            //
            // Only override when the existing binding is itself a bare
            // TypeParam — that's the broken case (the existing first-match
            // assoc loop returned a conformance whose bound was a free
            // `Param(T_ext)` that nothing in `subst` resolves). A concrete
            // existing binding means the assoc resolved correctly the first
            // time; don't disturb it.
            let should_insert = match subst.get(&e) {
                None => true,
                Some(MirTy::TypeParam(_)) => true,
                Some(_) => false,
            };
            if should_insert {
                subst.insert(e, bound.clone());
            }
        }
    }
}

/// Pattern-match `pattern` against `concrete`, recording any TypeParam-
/// in-pattern bindings into the shared `subst` (no overwriting). Mirrors
/// `monomorphize::witness::match_pattern` but writes directly into the
/// codegen subst map. `concrete` is assumed to already be substituted.
fn bind_pattern_into_subst(pattern: &MirTy, concrete: &MirTy, subst: &mut HashMap<Entity, MirTy>) {
    match (pattern, concrete) {
        (MirTy::TypeParam(entity), _) => {
            subst.entry(*entity).or_insert_with(|| concrete.clone());
        },
        (
            MirTy::Named {
                entity: e1,
                type_args: a1,
            },
            MirTy::Named {
                entity: e2,
                type_args: a2,
            },
        ) if e1 == e2 && a1.len() == a2.len() => {
            for (p, c) in a1.iter().zip(a2.iter()) {
                bind_pattern_into_subst(p, c, subst);
            }
        },
        (MirTy::Ref(a), MirTy::Ref(b))
        | (MirTy::RefMut(a), MirTy::RefMut(b))
        | (MirTy::Pointer(a), MirTy::Pointer(b)) => bind_pattern_into_subst(a, b, subst),
        (MirTy::Tuple(a), MirTy::Tuple(b)) if a.len() == b.len() => {
            for (p, c) in a.iter().zip(b.iter()) {
                bind_pattern_into_subst(p, c, subst);
            }
        },
        _ => {},
    }
}

/// Local copy of structural pattern match used purely to test whether a
/// witness's `implementing_type` matches a given concrete type. Doesn't
/// commit bindings to the outer subst — uses a local map for the check.
fn witness_pattern_matches(
    pattern: &MirTy,
    concrete: &MirTy,
    bindings: &mut HashMap<Entity, MirTy>,
) -> bool {
    match (pattern, concrete) {
        (MirTy::TypeParam(entity), _) => match bindings.get(entity) {
            Some(existing) => existing == concrete,
            None => {
                bindings.insert(*entity, concrete.clone());
                true
            },
        },
        (
            MirTy::Named {
                entity: e1,
                type_args: a1,
            },
            MirTy::Named {
                entity: e2,
                type_args: a2,
            },
        ) => {
            e1 == e2
                && a1.len() == a2.len()
                && a1
                    .iter()
                    .zip(a2)
                    .all(|(p, c)| witness_pattern_matches(p, c, bindings))
        },
        (MirTy::Ref(a), MirTy::Ref(b))
        | (MirTy::RefMut(a), MirTy::RefMut(b))
        | (MirTy::Pointer(a), MirTy::Pointer(b)) => witness_pattern_matches(a, b, bindings),
        (MirTy::Tuple(a), MirTy::Tuple(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|(p, c)| witness_pattern_matches(p, c, bindings))
        },
        (a, b) => a == b,
    }
}

fn ty_has_typeparam(ty: &MirTy) -> bool {
    match ty {
        MirTy::TypeParam(_) => true,
        MirTy::Named { type_args, .. } => type_args.iter().any(ty_has_typeparam),
        MirTy::Ref(inner) | MirTy::RefMut(inner) | MirTy::Pointer(inner) => ty_has_typeparam(inner),
        MirTy::Tuple(elems) => elems.iter().any(ty_has_typeparam),
        MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
            params.iter().any(ty_has_typeparam) || ty_has_typeparam(ret)
        },
        MirTy::AssociatedProjection { base, .. } => ty_has_typeparam(base),
        _ => false,
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

    // Capture pre-compile CLIF for dump mode. Cranelift's `ir::Function` Display
    // emits the standard CLIF text format. Post-compile CLIF would also be
    // interesting but requires a second format pass; leave for later.
    if ctx.options.emit_clif {
        ctx.clif_outputs
            .push((mangled_name.to_string(), format!("{}", cl_func)));
    }

    // Compile and define
    let mut cl_ctx = cranelift_codegen::Context::for_function(cl_func);
    stacker::maybe_grow(256 * 1024, 8 * 1024 * 1024, || {
        cl_ctx.compile(ctx.isa.as_ref(), &mut Default::default())
    })
    .map_err(|e| CodegenError::FunctionCompilation {
        name: mangled_name.to_string(),
        source: Box::new(std::io::Error::other(
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
                StatementKind::Assign {
                    rvalue: Rvalue::Ref(place) | Rvalue::RefMut(place),
                    ..
                } => {
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
                StatementKind::Call { args, .. } => {
                    for arg in args {
                        if matches!(arg.mode, PassingMode::Ref | PassingMode::MutRef)
                            && let kestrel_mir::Value::Place(place) = &arg.value
                                && let Some(id) = place.root_local() {
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
                },
                _ => {},
            }
        }
    }

    result
}
