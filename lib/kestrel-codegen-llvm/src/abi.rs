//! Calling convention. Faithful port of the Cranelift backend's `abi.rs`: a
//! "manual ABI" — aggregates pass/return by pointer (a leading `sret` integer
//! pointer param + manual `memcpy`), scalars pass by value. No LLVM `sret`/
//! `byval` attributes are used, so this matches the Cranelift lowering exactly.

use inkwell::context::Context;
use inkwell::types::{BasicMetadataTypeEnum, BasicType, FunctionType};
use kestrel_mir::mono::{MonoFunction, MonoModule};
use kestrel_mir::{ParamConvention, TyArena};

use crate::ty::{ScalarTy, TypeCache, TypeRepr};

#[derive(Debug, Clone, Copy)]
pub enum PassMode {
    ByVal(ScalarTy),
    ByRef,
    Zst,
}

#[derive(Debug, Clone, Copy)]
pub enum ReturnMode {
    Direct(ScalarTy),
    Sret,
    Void,
}

pub fn param_pass_mode(convention: ParamConvention, repr: TypeRepr) -> PassMode {
    match convention {
        ParamConvention::Borrow | ParamConvention::MutBorrow => PassMode::ByRef,
        ParamConvention::Consuming => match repr {
            TypeRepr::Scalar(t) => PassMode::ByVal(t),
            TypeRepr::Aggregate { .. } => PassMode::ByRef,
            TypeRepr::Zst => PassMode::Zst,
        },
    }
}

pub fn return_mode(repr: TypeRepr, is_main: bool) -> ReturnMode {
    if is_main {
        return ReturnMode::Direct(ScalarTy::I64);
    }
    match repr {
        TypeRepr::Scalar(t) => ReturnMode::Direct(t),
        TypeRepr::Aggregate { .. } => ReturnMode::Sret,
        TypeRepr::Zst => ReturnMode::Void,
    }
}

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
    is_main: bool,
    tc: &mut TypeCache,
    arena: &TyArena,
    module: &MonoModule,
    cx: &'ctx Context,
) -> FunctionType<'ctx> {
    let ptr_scalar = tc.ptr_scalar;
    let mut params: Vec<BasicMetadataTypeEnum<'ctx>> = Vec::new();

    let ret_repr = tc.repr(func.ret, arena, module);
    let ret_mode = return_mode(ret_repr, is_main);

    if matches!(ret_mode, ReturnMode::Sret) {
        params.push(ptr_scalar.llvm(cx).into());
    }

    for param in &func.params {
        let repr = tc.repr(param.ty, arena, module);
        match param_pass_mode(param.convention, repr) {
            PassMode::ByVal(t) => params.push(t.llvm(cx).into()),
            PassMode::ByRef => params.push(ptr_scalar.llvm(cx).into()),
            PassMode::Zst => {},
        }
    }

    finish_fn_type(cx, ret_mode, &params)
}

pub fn build_extern_signature<'ctx>(
    func: &MonoFunction,
    tc: &mut TypeCache,
    arena: &TyArena,
    module: &MonoModule,
    cx: &'ctx Context,
) -> FunctionType<'ctx> {
    let ptr_scalar = tc.ptr_scalar;
    let mut params: Vec<BasicMetadataTypeEnum<'ctx>> = Vec::new();

    let ret_repr = tc.repr(func.ret, arena, module);

    if ret_repr.is_aggregate() {
        params.push(ptr_scalar.llvm(cx).into());
    }

    for param in &func.params {
        let repr = tc.repr(param.ty, arena, module);
        match repr {
            TypeRepr::Scalar(t) => params.push(t.llvm(cx).into()),
            TypeRepr::Aggregate { .. } => params.push(ptr_scalar.llvm(cx).into()),
            TypeRepr::Zst => {},
        }
    }

    let ret_mode = match ret_repr {
        TypeRepr::Scalar(t) => ReturnMode::Direct(t),
        _ => ReturnMode::Void,
    };
    finish_fn_type(cx, ret_mode, &params)
}
