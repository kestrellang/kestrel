use cranelift_codegen::ir::{self, AbiParam};
use cranelift_codegen::isa::CallConv;
use kestrel_mir::mono::MonoModule;
use kestrel_mir::{ParamConvention, TyArena, TyId};

use crate::ty::{Cranelift, TypeCache};

pub type PassMode = kestrel_codegen::PassMode<Cranelift>;
pub type ReturnMode = kestrel_codegen::ReturnMode<Cranelift>;
pub use kestrel_codegen::{param_pass_mode, return_mode};

pub fn build_signature(
    func: &kestrel_mir::mono::MonoFunction,
    tc: &mut TypeCache,
    arena: &TyArena,
    module: &MonoModule,
    call_conv: CallConv,
) -> ir::Signature {
    let kparams: Vec<_> = func.params.iter().map(|p| (p.ty, p.convention)).collect();
    build_signature_parts(&kparams, func.ret, tc, arena, module, call_conv)
}

/// The Kestrel-ABI signature of `(type, convention)` params and `ret`.
pub fn build_signature_parts(
    kparams: &[(TyId, ParamConvention)],
    ret: TyId,
    tc: &mut TypeCache,
    arena: &TyArena,
    module: &MonoModule,
    call_conv: CallConv,
) -> ir::Signature {
    let ptr_ty = tc.ptr_scalar;
    let mut sig = ir::Signature::new(call_conv);

    let ret_repr = tc.repr(ret, arena, module);
    let ret_mode = return_mode(ret_repr);

    if matches!(ret_mode, ReturnMode::Sret) {
        sig.params.push(AbiParam::new(ptr_ty));
    }

    for &(ty, convention) in kparams {
        let repr = tc.repr(ty, arena, module);
        match param_pass_mode(convention, repr) {
            PassMode::ByVal(t) => sig.params.push(AbiParam::new(t)),
            PassMode::ByRef => sig.params.push(AbiParam::new(ptr_ty)),
            PassMode::Zst => {},
        }
    }

    match ret_mode {
        ReturnMode::Direct(t) => sig.returns.push(AbiParam::new(t)),
        ReturnMode::Sret | ReturnMode::Void => {},
    }

    sig
}
