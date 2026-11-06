use cranelift_codegen::ir;
use kestrel_codegen::ScalarModel;
use kestrel_mir::{FloatBits, IntBits};

/// Plugs Cranelift's `ir::Type` into the shared "scalar or memory"
/// classification (`kestrel_codegen::repr`), which the LLVM backend uses too.
#[derive(Debug, Clone, Copy)]
pub struct Cranelift;

impl ScalarModel for Cranelift {
    type Scalar = ir::Type;

    fn from_int_bits(bits: IntBits) -> ir::Type {
        int_bits_to_cl(bits)
    }

    fn from_float_bits(bits: FloatBits) -> ir::Type {
        float_bits_to_cl(bits)
    }

    fn size_bytes(scalar: ir::Type) -> u64 {
        u64::from(scalar.bytes())
    }
}

pub type TypeRepr = kestrel_codegen::TypeRepr<Cranelift>;
pub type TypeCache = kestrel_codegen::TypeCache<Cranelift>;

pub fn int_bits_to_cl(bits: IntBits) -> ir::Type {
    match bits {
        IntBits::I8 => ir::types::I8,
        IntBits::I16 => ir::types::I16,
        IntBits::I32 => ir::types::I32,
        IntBits::I64 => ir::types::I64,
    }
}

pub fn float_bits_to_cl(bits: FloatBits) -> ir::Type {
    match bits {
        FloatBits::F16 => ir::types::F16,
        FloatBits::F32 => ir::types::F32,
        FloatBits::F64 => ir::types::F64,
    }
}
