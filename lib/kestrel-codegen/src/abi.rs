//! How a value crosses a call boundary. A "manual ABI": scalars pass and
//! return by value; aggregates pass by pointer and return through a leading
//! sret pointer; ZSTs take no slot. Both backends build their signatures from
//! these modes, so their calling conventions cannot drift apart.

use kestrel_mir::ParamConvention;

use crate::repr::{ScalarModel, TypeRepr};

#[derive(Debug, Clone, Copy)]
pub enum PassMode<M: ScalarModel> {
    ByVal(M::Scalar),
    ByRef,
    Zst,
}

#[derive(Debug, Clone, Copy)]
pub enum ReturnMode<M: ScalarModel> {
    Direct(M::Scalar),
    Sret,
    Void,
}

pub fn param_pass_mode<M: ScalarModel>(
    convention: ParamConvention,
    repr: TypeRepr<M>,
) -> PassMode<M> {
    match convention {
        ParamConvention::Borrow | ParamConvention::MutBorrow => PassMode::ByRef,
        ParamConvention::Consuming => match repr {
            TypeRepr::Scalar(t) => PassMode::ByVal(t),
            TypeRepr::Aggregate { .. } => PassMode::ByRef,
            TypeRepr::Zst => PassMode::Zst,
        },
    }
}

pub fn return_mode<M: ScalarModel>(repr: TypeRepr<M>) -> ReturnMode<M> {
    // The `@main` entry point is an ordinary `i64`-returning function (the
    // MIR-synthesized wrapper), so no entry-point special-casing is needed here.
    match repr {
        TypeRepr::Scalar(t) => ReturnMode::Direct(t),
        TypeRepr::Aggregate { .. } => ReturnMode::Sret,
        TypeRepr::Zst => ReturnMode::Void,
    }
}
