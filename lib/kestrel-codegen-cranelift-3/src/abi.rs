use cranelift_codegen::ir::{self, AbiParam};
use cranelift_codegen::isa::CallConv;
use kestrel_mir_3::mono::MonoModule;
use kestrel_mir_3::{ParamConvention, TyArena};

use crate::ty::{TypeCache, TypeRepr};

#[derive(Debug, Clone, Copy)]
pub enum PassMode {
    ByVal(ir::Type),
    ByRef,
    Zst,
}

#[derive(Debug, Clone, Copy)]
pub enum ReturnMode {
    Direct(ir::Type),
    Sret,
    Void,
}

pub fn param_pass_mode(
    convention: ParamConvention,
    repr: TypeRepr,
    _ptr_ty: ir::Type,
) -> PassMode {
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
        return ReturnMode::Direct(ir::types::I64);
    }
    match repr {
        TypeRepr::Scalar(t) => ReturnMode::Direct(t),
        TypeRepr::Aggregate { .. } => ReturnMode::Sret,
        TypeRepr::Zst => ReturnMode::Void,
    }
}

pub fn build_signature(
    func: &kestrel_mir_3::mono::MonoFunction,
    is_main: bool,
    tc: &mut TypeCache,
    arena: &TyArena,
    module: &MonoModule,
    call_conv: CallConv,
) -> ir::Signature {
    let ptr_ty = tc.ptr_ty;
    let mut sig = ir::Signature::new(call_conv);

    let ret_repr = tc.repr(func.ret, arena, module);
    let ret_mode = return_mode(ret_repr, is_main);

    if matches!(ret_mode, ReturnMode::Sret) {
        sig.params.push(AbiParam::new(ptr_ty));
    }

    for param in &func.params {
        let repr = tc.repr(param.ty, arena, module);
        match param_pass_mode(param.convention, repr, ptr_ty) {
            PassMode::ByVal(t) => sig.params.push(AbiParam::new(t)),
            PassMode::ByRef => sig.params.push(AbiParam::new(ptr_ty)),
            PassMode::Zst => {}
        }
    }

    match ret_mode {
        ReturnMode::Direct(t) => sig.returns.push(AbiParam::new(t)),
        ReturnMode::Sret | ReturnMode::Void => {}
    }

    sig
}

pub fn build_extern_signature(
    func: &kestrel_mir_3::mono::MonoFunction,
    tc: &mut TypeCache,
    arena: &TyArena,
    module: &MonoModule,
    call_conv: CallConv,
) -> ir::Signature {
    let ptr_ty = tc.ptr_ty;
    let mut sig = ir::Signature::new(call_conv);

    let ret_repr = tc.repr(func.ret, arena, module);

    if ret_repr.is_aggregate() {
        sig.params.push(AbiParam::new(ptr_ty));
    }

    for param in &func.params {
        let repr = tc.repr(param.ty, arena, module);
        match repr {
            TypeRepr::Scalar(t) => sig.params.push(AbiParam::new(t)),
            TypeRepr::Aggregate { .. } => sig.params.push(AbiParam::new(ptr_ty)),
            TypeRepr::Zst => {}
        }
    }

    if let TypeRepr::Scalar(t) = ret_repr {
        sig.returns.push(AbiParam::new(t));
    }

    sig
}
