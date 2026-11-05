//! Bridges between Kestrel's ABI and the C ABI (`kestrel_codegen::c_abi`);
//! the twin of `kestrel-codegen-llvm`'s `c_bridge`.
//!
//! Every MIR function, `@extern` ones included, is called with Kestrel's ABI.
//! An `@extern` is a local Kestrel-ABI *import thunk* that makes the C call;
//! an `@export` gets an exported C-ABI *export wrapper* that calls the
//! Kestrel body.
//!
//! Cranelift has no array arguments, so an AArch64 struct split into two
//! registers passes as two scalars. That only differs from C when the
//! registers run out mid-struct (after seven integer arguments).

use cranelift_codegen::ir::{self, AbiParam, ArgumentPurpose, InstBuilder, MemFlags, Value};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::{FuncId, Linkage, Module};
use kestrel_codegen::c_abi::{self, CArg, CPrim, CRet, CSig, Piece};
use kestrel_mir::mono::MonoFunction;

use crate::context::CodegenCtx;
use crate::error::CodegenError;
use crate::mem;

fn prim_type(prim: CPrim) -> ir::Type {
    match prim {
        CPrim::I8 => ir::types::I8,
        CPrim::I16 => ir::types::I16,
        CPrim::I32 => ir::types::I32,
        CPrim::I64 => ir::types::I64,
        CPrim::F32 => ir::types::F32,
        // Two floats in one SSE register: the same bits as one double.
        CPrim::F64 | CPrim::F32x2 => ir::types::F64,
    }
}

/// `uext` / `sext` for an 8- or 16-bit integer: C wants the caller to widen
/// it to 32 bits.
fn extended(param: AbiParam, unsigned: bool) -> AbiParam {
    if param.value_type.bits() >= 32 || !param.value_type.is_int() {
        return param;
    }
    if unsigned { param.uext() } else { param.sext() }
}

/// Classify `func`'s C signature and lower it to a Cranelift signature.
pub fn c_signature(ctx: &mut CodegenCtx<'_>, func: &MonoFunction) -> (CSig, ir::Signature) {
    let module = ctx.module;
    let arena = &module.ty_arena;
    let ptr = ctx.ptr_ty;
    let params: Vec<_> = func.params.iter().map(|p| (p.ty, p.convention)).collect();
    let csig = c_abi::classify(ctx.c_abi, &mut ctx.tc, arena, module, func.ret, &params);
    let ext = func.extern_info.as_ref();
    let unsigned = |i: usize| {
        ext.and_then(|e| e.unsigned_params.get(i).copied())
            .unwrap_or(false)
    };

    let mut sig = ir::Signature::new(ctx.isa.default_call_conv());
    if csig.ret == CRet::Sret {
        sig.params
            .push(AbiParam::special(ptr, ArgumentPurpose::StructReturn));
    }
    for (i, (carg, param)) in csig.params.iter().zip(&func.params).enumerate() {
        match carg {
            CArg::Ignore => {},
            CArg::Pointer | CArg::Indirect => sig.params.push(AbiParam::new(ptr)),
            CArg::Direct => {
                let scalar = ctx.tc.value_scalar(param.ty, arena, module);
                sig.params
                    .push(extended(AbiParam::new(scalar), unsigned(i)));
            },
            CArg::Split { pieces, .. } => {
                sig.params
                    .extend(pieces.iter().map(|p| AbiParam::new(prim_type(p.prim))));
            },
            CArg::Stack { size, .. } => sig.params.push(AbiParam::special(
                ptr,
                ArgumentPurpose::StructArgument(*size as u32),
            )),
        }
    }
    match &csig.ret {
        CRet::Void | CRet::Sret => {},
        CRet::Direct => {
            let scalar = ctx.tc.value_scalar(func.ret, arena, module);
            let unsigned_ret = ext.is_some_and(|e| e.unsigned_ret);
            sig.returns
                .push(extended(AbiParam::new(scalar), unsigned_ret));
        },
        CRet::Split { pieces } => {
            sig.returns
                .extend(pieces.iter().map(|p| AbiParam::new(prim_type(p.prim))));
        },
    }
    (csig, sig)
}

fn load_pieces(builder: &mut FunctionBuilder, src: Value, pieces: &[Piece]) -> Vec<Value> {
    pieces
        .iter()
        .map(|p| {
            builder
                .ins()
                .load(prim_type(p.prim), MemFlags::new(), src, p.offset as i32)
        })
        .collect()
}

fn store_pieces(builder: &mut FunctionBuilder, dest: Value, pieces: &[Piece], vals: &[Value]) {
    for (p, &v) in pieces.iter().zip(vals) {
        builder
            .ins()
            .store(MemFlags::new(), v, dest, p.offset as i32);
    }
}

/// Compile and define a bridge function built by `build`.
fn define(
    ctx: &mut CodegenCtx<'_>,
    func_id: FuncId,
    sig: ir::Signature,
    name: &str,
    build: impl FnOnce(&mut CodegenCtx<'_>, &mut FunctionBuilder, &[Value]),
) -> Result<(), CodegenError> {
    let mut cl_func =
        ir::Function::with_name_signature(ir::UserFuncName::user(0, func_id.as_u32()), sig);
    let mut fbc = std::mem::take(&mut ctx.func_builder_ctx);
    {
        let mut builder = FunctionBuilder::new(&mut cl_func, &mut fbc);
        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);
        builder.seal_block(entry);
        let params = builder.block_params(entry).to_vec();
        build(ctx, &mut builder, &params);
        builder.finalize();
    }
    ctx.func_builder_ctx = fbc;
    let mut comp_ctx = cranelift_codegen::Context::for_function(cl_func);
    comp_ctx
        .compile(ctx.isa.as_ref(), &mut Default::default())
        .map_err(|e| CodegenError::FunctionCompilation {
            name: name.to_string(),
            source: Box::new(std::io::Error::other(format!("{e:?}"))),
        })?;
    ctx.cl_module
        .define_function(func_id, &mut comp_ctx)
        .map_err(|e| CodegenError::FunctionDefinition {
            name: name.to_string(),
            source: Box::new(e),
        })
}

/// Define the Kestrel-ABI `thunk` as a call to the imported C function `c_fn`.
pub fn define_import_thunk(
    ctx: &mut CodegenCtx<'_>,
    func: &MonoFunction,
    thunk: FuncId,
    c_fn: FuncId,
    csig: &CSig,
) -> Result<(), CodegenError> {
    let module = ctx.module;
    let sig = ctx
        .cl_module
        .declarations()
        .get_function_decl(thunk)
        .signature
        .clone();
    define(ctx, thunk, sig, &func.name, |ctx, builder, kparams| {
        let ptr = ctx.ptr_ty;
        let ret_repr = ctx.tc.repr(func.ret, &module.ty_arena, module);
        let mut k = kparams.iter().copied();
        let ksret = ret_repr.is_aggregate().then(|| k.next().unwrap());

        let mut cargs = Vec::new();
        if csig.ret == CRet::Sret {
            cargs.push(ksret.unwrap());
        }
        for (carg, param) in csig.params.iter().zip(&func.params) {
            let repr = ctx.tc.repr(param.ty, &module.ty_arena, module);
            if repr.is_zst() && param.convention == kestrel_mir::ParamConvention::Consuming {
                continue;
            }
            let kval = k.next().unwrap();
            match carg {
                CArg::Ignore => {},
                CArg::Direct | CArg::Pointer | CArg::Stack { .. } => cargs.push(kval),
                CArg::Split { pieces, .. } => {
                    let tmp = mem::alloc_stack_slot(
                        builder,
                        CSig::split_slot_size(pieces).max(repr.size()),
                        16,
                        ptr,
                    );
                    mem::copy_aggregate(builder, repr.size(), tmp, kval);
                    cargs.extend(load_pieces(builder, tmp, pieces));
                },
                CArg::Indirect => {
                    let copy = mem::alloc_stack_slot(builder, repr.size(), 16, ptr);
                    mem::copy_aggregate(builder, repr.size(), copy, kval);
                    cargs.push(copy);
                },
            }
        }

        let fref = ctx.cl_module.declare_func_in_func(c_fn, builder.func);
        let call = builder.ins().call(fref, &cargs);
        let results = builder.inst_results(call).to_vec();
        match &csig.ret {
            CRet::Void | CRet::Sret => {
                builder.ins().return_(&[]);
            },
            CRet::Direct => {
                builder.ins().return_(&results);
            },
            CRet::Split { pieces } => {
                let tmp = mem::alloc_stack_slot(builder, CSig::split_slot_size(pieces), 16, ptr);
                store_pieces(builder, tmp, pieces, &results);
                mem::copy_aggregate(builder, ret_repr.size(), ksret.unwrap(), tmp);
                builder.ins().return_(&[]);
            },
        }
    })
}

/// Declare and define the exported C-ABI wrapper of `func`, calling the
/// Kestrel function `body`.
pub fn define_export_wrapper(
    ctx: &mut CodegenCtx<'_>,
    func: &MonoFunction,
    body: FuncId,
    symbol: &str,
) -> Result<FuncId, CodegenError> {
    let module = ctx.module;
    let (csig, sig) = c_signature(ctx, func);
    let wrapper = ctx
        .cl_module
        .declare_function(symbol, Linkage::Export, &sig)
        .map_err(|e| CodegenError::FunctionDefinition {
            name: symbol.to_string(),
            source: Box::new(e),
        })?;
    define(ctx, wrapper, sig, symbol, |ctx, builder, cparams| {
        let ptr = ctx.ptr_ty;
        let ret_repr = ctx.tc.repr(func.ret, &module.ty_arena, module);
        let mut c = cparams.iter().copied();
        let csret = (csig.ret == CRet::Sret).then(|| c.next().unwrap());

        let mut kargs = Vec::new();
        let ksret = ret_repr.is_aggregate().then(|| {
            csret.unwrap_or_else(|| {
                let size = match &csig.ret {
                    CRet::Split { pieces } => CSig::split_slot_size(pieces).max(ret_repr.size()),
                    _ => ret_repr.size(),
                };
                mem::alloc_stack_slot(builder, size, 16, ptr)
            })
        });
        kargs.extend(ksret);
        for (carg, param) in csig.params.iter().zip(&func.params) {
            match carg {
                CArg::Ignore => {},
                CArg::Direct | CArg::Pointer | CArg::Stack { .. } | CArg::Indirect => {
                    kargs.push(c.next().unwrap());
                },
                CArg::Split { pieces, .. } => {
                    let size = ctx.tc.repr(param.ty, &module.ty_arena, module).size();
                    let vals: Vec<Value> = pieces.iter().map(|_| c.next().unwrap()).collect();
                    let tmp = mem::alloc_stack_slot(
                        builder,
                        CSig::split_slot_size(pieces).max(size),
                        16,
                        ptr,
                    );
                    store_pieces(builder, tmp, pieces, &vals);
                    kargs.push(tmp);
                },
            }
        }

        let fref = ctx.cl_module.declare_func_in_func(body, builder.func);
        let call = builder.ins().call(fref, &kargs);
        let results = builder.inst_results(call).to_vec();
        match &csig.ret {
            CRet::Void | CRet::Sret => {
                builder.ins().return_(&[]);
            },
            CRet::Direct => {
                builder.ins().return_(&results);
            },
            CRet::Split { pieces } => {
                let vals = load_pieces(builder, ksret.unwrap(), pieces);
                builder.ins().return_(&vals);
            },
        }
    })?;
    Ok(wrapper)
}
