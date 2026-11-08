//! LLVM's machine scalar. The "scalar or memory" classification of MIR types
//! itself lives in `kestrel_codegen::repr`, shared with the Cranelift backend;
//! this module only supplies the scalar type it is generic over.
//!
//! Typed-`ptr` representation (formerly "Option A" used `i64` here): pointer-
//! width scalars — addresses, aggregate references, `Pointer`/`FuncThin`
//! scalars, function pointers — are the LLVM `ptr` scalar `ScalarTy::Ptr`, and
//! offset math is `getelementptr` (see `mem`). This restores pointer provenance
//! so LLVM's alias analysis can devirtualize/LICM/vectorize. The only genuine
//! `int<->ptr` conversions left are `Op::PtrToAddress`/`Op::PtrFromAddress`.

use inkwell::AddressSpace;
use inkwell::context::Context;
use inkwell::types::BasicTypeEnum;
use kestrel_codegen::ScalarModel;
use kestrel_mir::{FloatBits, IntBits};

/// A machine scalar type, independent of any LLVM context lifetime (Copy, like
/// Cranelift's `ir::Type`). `Ptr` is the pointer-width opaque LLVM `ptr` and is
/// neither int nor float (it carries addresses, `Pointer`/`FuncThin` scalars,
/// and function pointers). It currently assumes 8-byte pointers (64-bit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarTy {
    I8,
    I16,
    I32,
    I64,
    F16,
    F32,
    F64,
    Ptr,
}

impl ScalarTy {
    pub fn bytes(self) -> u64 {
        match self {
            Self::I8 => 1,
            Self::I16 | Self::F16 => 2,
            Self::I32 | Self::F32 => 4,
            // Ptr is pointer-width; the backend assumes 64-bit (see `CodegenCtx::new`).
            Self::I64 | Self::F64 | Self::Ptr => 8,
        }
    }

    pub fn is_int(self) -> bool {
        matches!(self, Self::I8 | Self::I16 | Self::I32 | Self::I64)
    }

    /// Positive match — `Ptr` is neither int nor float, so this is NOT `!is_int`.
    pub fn is_float(self) -> bool {
        matches!(self, Self::F16 | Self::F32 | Self::F64)
    }

    pub fn is_ptr(self) -> bool {
        matches!(self, Self::Ptr)
    }

    /// Materialise the inkwell type for this scalar.
    pub fn llvm<'ctx>(self, cx: &'ctx Context) -> BasicTypeEnum<'ctx> {
        match self {
            Self::I8 => cx.i8_type().into(),
            Self::I16 => cx.i16_type().into(),
            Self::I32 => cx.i32_type().into(),
            Self::I64 => cx.i64_type().into(),
            Self::F16 => cx.f16_type().into(),
            Self::F32 => cx.f32_type().into(),
            Self::F64 => cx.f64_type().into(),
            Self::Ptr => cx.ptr_type(AddressSpace::default()).into(),
        }
    }
}

impl ScalarTy {
    pub fn from_int_bits(bits: IntBits) -> Self {
        match bits {
            IntBits::I8 => Self::I8,
            IntBits::I16 => Self::I16,
            IntBits::I32 => Self::I32,
            IntBits::I64 => Self::I64,
        }
    }

    pub fn from_float_bits(bits: FloatBits) -> Self {
        match bits {
            FloatBits::F16 => Self::F16,
            FloatBits::F32 => Self::F32,
            FloatBits::F64 => Self::F64,
        }
    }
}

/// Plugs `ScalarTy` into the shared classification (`kestrel_codegen::repr`).
#[derive(Debug, Clone, Copy)]
pub struct Llvm;

impl ScalarModel for Llvm {
    type Scalar = ScalarTy;

    fn from_int_bits(bits: IntBits) -> ScalarTy {
        ScalarTy::from_int_bits(bits)
    }

    fn from_float_bits(bits: FloatBits) -> ScalarTy {
        ScalarTy::from_float_bits(bits)
    }

    fn size_bytes(scalar: ScalarTy) -> u64 {
        scalar.bytes()
    }
}

/// Classification is shared with the Cranelift backend (`kestrel_codegen::repr`).
pub type TypeRepr = kestrel_codegen::TypeRepr<Llvm>;
pub type TypeCache = kestrel_codegen::TypeCache<Llvm>;
