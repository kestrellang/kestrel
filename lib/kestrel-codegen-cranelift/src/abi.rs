use cranelift_codegen::ir::{self, AbiParam};
use cranelift_codegen::isa::CallConv;
use kestrel_mir::TyArena;
use kestrel_mir::mono::MonoModule;

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
    let ptr_ty = tc.ptr_scalar;
    let mut sig = ir::Signature::new(call_conv);

    let ret_repr = tc.repr(func.ret, arena, module);
    let ret_mode = return_mode(ret_repr);

    if matches!(ret_mode, ReturnMode::Sret) {
        sig.params.push(AbiParam::new(ptr_ty));
    }

    for param in &func.params {
        let repr = tc.repr(param.ty, arena, module);
        match param_pass_mode(param.convention, repr) {
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
