//! Calling convention. A "manual ABI" — aggregates pass/return by pointer (a
//! leading `ptr` sret param + manual `memcpy`), scalars pass by value. No LLVM
//! `sret`/`byval`/`noalias` attributes are used (those are a deliberate
//! follow-up; this keeps exact ABI compatibility with extern "C" and across
//! calls). Sret/ByRef/aggregate params are the `ptr` scalar (`ScalarTy::Ptr`),
//! so no body change was needed for the typed-`ptr` migration.

use inkwell::context::Context;
use inkwell::types::{BasicMetadataTypeEnum, BasicType, FunctionType};
use kestrel_mir::mono::{MonoFunction, MonoModule};
use kestrel_mir::{ParamConvention, TyArena, TyId};

use crate::ty::{Llvm, TypeCache};

pub type PassMode = kestrel_codegen::PassMode<Llvm>;
pub type ReturnMode = kestrel_codegen::ReturnMode<Llvm>;
pub use kestrel_codegen::{param_pass_mode, return_mode};

fn finish_fn_type<'ctx>(
    cx: &'ctx Context,
    ret_mode: ReturnMode,
    params: &[BasicMetadataTypeEnum<'ctx>],
) -> FunctionType<'ctx> {
    match ret_mode {
        ReturnMode::Direct(t) => t.llvm(cx).fn_type(params, false),
        ReturnMode::Sret | ReturnMode::Void => cx.void_type().fn_type(params, false),
    }
}

pub fn build_signature<'ctx>(
    func: &MonoFunction,
    tc: &mut TypeCache,
    arena: &TyArena,
    module: &MonoModule,
    cx: &'ctx Context,
) -> FunctionType<'ctx> {
    let kparams: Vec<_> = func.params.iter().map(|p| (p.ty, p.convention)).collect();
    build_signature_parts(&kparams, func.ret, tc, arena, module, cx)
}

/// The Kestrel-ABI function type of `(type, convention)` params and `ret`.
pub fn build_signature_parts<'ctx>(
    kparams: &[(TyId, ParamConvention)],
    ret: TyId,
    tc: &mut TypeCache,
    arena: &TyArena,
    module: &MonoModule,
    cx: &'ctx Context,
) -> FunctionType<'ctx> {
    let ptr_scalar = tc.ptr_scalar;
    let mut params: Vec<BasicMetadataTypeEnum<'ctx>> = Vec::new();

    let ret_repr = tc.repr(ret, arena, module);
    let ret_mode = return_mode(ret_repr);

    if matches!(ret_mode, ReturnMode::Sret) {
        params.push(ptr_scalar.llvm(cx).into());
    }

    for &(ty, convention) in kparams {
        let repr = tc.repr(ty, arena, module);
        match param_pass_mode(convention, repr) {
            PassMode::ByVal(t) => params.push(t.llvm(cx).into()),
            PassMode::ByRef => params.push(ptr_scalar.llvm(cx).into()),
            PassMode::Zst => {},
        }
    }

    finish_fn_type(cx, ret_mode, &params)
}
