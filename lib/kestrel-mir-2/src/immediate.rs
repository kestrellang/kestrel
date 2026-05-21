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

    pub fn i8(value: i128) -> Self {
        Self::new(ImmediateKind::IntLiteral {
            bits: IntBits::I8,
            value,
        })
    }

    pub fn i16(value: i128) -> Self {
        Self::new(ImmediateKind::IntLiteral {
            bits: IntBits::I16,
            value,
        })
    }

    pub fn i32(value: i128) -> Self {
        Self::new(ImmediateKind::IntLiteral {
            bits: IntBits::I32,
            value,
        })
    }

    pub fn i64(value: i128) -> Self {
        Self::new(ImmediateKind::IntLiteral {
            bits: IntBits::I64,
            value,
        })
    }

    pub fn f32(value: f64) -> Self {
        Self::new(ImmediateKind::FloatLiteral {
            bits: FloatBits::F32,
            value,
        })
    }

    pub fn f64(value: f64) -> Self {
        Self::new(ImmediateKind::FloatLiteral {
            bits: FloatBits::F64,
            value,
        })
    }

    pub fn bool(value: bool) -> Self {
        Self::new(ImmediateKind::BoolLiteral(value))
    }

    pub fn string(s: impl Into<String>) -> Self {
        Self::new(ImmediateKind::StringLiteral(s.into()))
    }

    pub fn unit() -> Self {
        Self::new(ImmediateKind::Unit)
    }

    pub fn function_ref(
        func: Entity,
        type_args: Vec<TyId>,
        self_type: Option<TyId>,
    ) -> Self {
        Self::new(ImmediateKind::FunctionRef {
            func,
            type_args,
            self_type,
        })
    }

    pub fn null_ptr(ty: TyId) -> Self {
        Self::new(ImmediateKind::NullPtr(ty))
    }

    pub fn size_of(ty: TyId) -> Self {
        Self::new(ImmediateKind::SizeOf(ty))
    }

    pub fn align_of(ty: TyId) -> Self {
        Self::new(ImmediateKind::AlignOf(ty))
    }

    pub fn error() -> Self {
        Self::new(ImmediateKind::Error)
    }

    /// Return the type of this immediate, interning if needed.
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
            ImmediateKind::StringLiteral(_) | ImmediateKind::StringPointer(_) => arena.str_ty(),
            ImmediateKind::Unit => arena.unit(),
            ImmediateKind::NullPtr(ty) | ImmediateKind::SizeOf(ty) | ImmediateKind::AlignOf(ty) => {
                *ty
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_literal() {
        let imm = Immediate::i64(42);
        assert_eq!(
            imm.kind,
            ImmediateKind::IntLiteral {
                bits: IntBits::I64,
                value: 42,
            }
        );
    }

    #[test]
    fn float_literal() {
        let imm = Immediate::f64(2.5);
        match &imm.kind {
            ImmediateKind::FloatLiteral { bits, value } => {
                assert_eq!(*bits, FloatBits::F64);
                assert!((value - 2.5).abs() < f64::EPSILON);
            }
            other => panic!("expected FloatLiteral, got {other:?}"),
        }
    }

    #[test]
    fn bool_literal() {
        assert_eq!(Immediate::bool(true).kind, ImmediateKind::BoolLiteral(true));
        assert_eq!(Immediate::bool(false).kind, ImmediateKind::BoolLiteral(false));
    }

    #[test]
    fn string_literal() {
        let imm = Immediate::string("hello");
        assert_eq!(
            imm.kind,
            ImmediateKind::StringLiteral("hello".to_string())
        );
    }

    #[test]
    fn unit_immediate() {
        assert_eq!(Immediate::unit().kind, ImmediateKind::Unit);
    }

    #[test]
    fn null_ptr() {
        let ty = TyId::new(0);
        assert_eq!(Immediate::null_ptr(ty).kind, ImmediateKind::NullPtr(ty));
    }

    #[test]
    fn size_of_align_of() {
        let ty = TyId::new(1);
        assert_eq!(Immediate::size_of(ty).kind, ImmediateKind::SizeOf(ty));
        assert_eq!(Immediate::align_of(ty).kind, ImmediateKind::AlignOf(ty));
    }

    #[test]
    fn function_ref() {
        let func = Entity::from_raw(1);
        let ty_arg = TyId::new(0);
        let imm = Immediate::function_ref(func, vec![ty_arg], None);
        match &imm.kind {
            ImmediateKind::FunctionRef {
                func: f,
                type_args,
                self_type,
            } => {
                assert_eq!(*f, func);
                assert_eq!(type_args, &[ty_arg]);
                assert_eq!(*self_type, None);
            }
            other => panic!("expected FunctionRef, got {other:?}"),
        }
    }

    #[test]
    fn error_immediate() {
        assert_eq!(Immediate::error().kind, ImmediateKind::Error);
    }

    #[test]
    fn clone_equality() {
        let a = Immediate::i64(100);
        let b = a.clone();
        assert_eq!(a, b);
    }
}
