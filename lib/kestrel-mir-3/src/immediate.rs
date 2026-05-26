use kestrel_hecs::Entity;

use crate::ty::TyArena;
use crate::{FloatBits, IntBits, MonoFuncId, TyId};

#[derive(Debug, Clone, PartialEq)]
pub enum ImmediateKind {
    IntLiteral { bits: IntBits, value: i128 },
    FloatLiteral { bits: FloatBits, value: f64 },
    BoolLiteral(bool),
    StringLiteral(String),
    StringPointer(String),
    Unit,
    FunctionRef {
        func: Entity,
        type_args: Vec<TyId>,
        self_type: Option<TyId>,
    },
    MonoFunctionRef(MonoFuncId),
    NullPtr(TyId),
    SizeOf(TyId),
    AlignOf(TyId),
    FloatInfinity(FloatBits),
    FloatNan(FloatBits),
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Immediate {
    pub kind: ImmediateKind,
}

impl Immediate {
    pub fn new(kind: ImmediateKind) -> Self {
        Self { kind }
    }

    pub fn i8(value: i128) -> Self { Self::new(ImmediateKind::IntLiteral { bits: IntBits::I8, value }) }
    pub fn i16(value: i128) -> Self { Self::new(ImmediateKind::IntLiteral { bits: IntBits::I16, value }) }
    pub fn i32(value: i128) -> Self { Self::new(ImmediateKind::IntLiteral { bits: IntBits::I32, value }) }
    pub fn i64(value: i128) -> Self { Self::new(ImmediateKind::IntLiteral { bits: IntBits::I64, value }) }
    pub fn f32(value: f64) -> Self { Self::new(ImmediateKind::FloatLiteral { bits: FloatBits::F32, value }) }
    pub fn f64(value: f64) -> Self { Self::new(ImmediateKind::FloatLiteral { bits: FloatBits::F64, value }) }
    pub fn bool(value: bool) -> Self { Self::new(ImmediateKind::BoolLiteral(value)) }
    pub fn string(s: impl Into<String>) -> Self { Self::new(ImmediateKind::StringLiteral(s.into())) }
    pub fn string_pointer(s: impl Into<String>) -> Self { Self::new(ImmediateKind::StringPointer(s.into())) }
    pub fn unit() -> Self { Self::new(ImmediateKind::Unit) }

    pub fn function_ref(func: Entity, type_args: Vec<TyId>, self_type: Option<TyId>) -> Self {
        Self::new(ImmediateKind::FunctionRef { func, type_args, self_type })
    }

    pub fn null_ptr(ty: TyId) -> Self { Self::new(ImmediateKind::NullPtr(ty)) }
    pub fn size_of(ty: TyId) -> Self { Self::new(ImmediateKind::SizeOf(ty)) }
    pub fn align_of(ty: TyId) -> Self { Self::new(ImmediateKind::AlignOf(ty)) }
    pub fn error() -> Self { Self::new(ImmediateKind::Error) }

    pub fn ty(&self, arena: &mut TyArena) -> TyId {
        match &self.kind {
            ImmediateKind::IntLiteral { bits, .. } => match bits {
                IntBits::I8 => arena.i8(),
                IntBits::I16 => arena.i16(),
                IntBits::I32 => arena.i32(),
                IntBits::I64 => arena.i64(),
            },
            ImmediateKind::FloatLiteral { bits, .. } => match bits {
                FloatBits::F16 => arena.f16(),
                FloatBits::F32 => arena.f32(),
                FloatBits::F64 => arena.f64(),
            },
            ImmediateKind::BoolLiteral(_) => arena.bool(),
            ImmediateKind::StringLiteral(_) => arena.str_ty(),
            ImmediateKind::StringPointer(_) => {
                let i8 = arena.i8();
                arena.pointer(i8)
            }
            ImmediateKind::Unit => arena.unit(),
            ImmediateKind::NullPtr(ty) | ImmediateKind::SizeOf(ty) | ImmediateKind::AlignOf(ty) => *ty,
            ImmediateKind::FloatInfinity(bits) | ImmediateKind::FloatNan(bits) => match bits {
                FloatBits::F16 => arena.f16(),
                FloatBits::F32 => arena.f32(),
                FloatBits::F64 => arena.f64(),
            },
            ImmediateKind::FunctionRef { .. }
            | ImmediateKind::MonoFunctionRef(_)
            | ImmediateKind::Error => arena.error(),
        }
    }
}
