use kestrel_hecs::Entity;
use kestrel_span::Span;

use smallvec::SmallVec;

use crate::{ArgMode, FieldIdx, LocalId, Op, Operand, Place, TyId, UseMode, VariantIdx};

#[derive(Debug, Clone, PartialEq)]
pub enum Rvalue {
    // Value transfer
    Use(Operand, UseMode),

    // Reference creation
    Ref(Place),
    RefMut(Place),

    // Operations (always read, never consume)
    Op1 { op: Op, arg: Operand },
    Op2 { op: Op, lhs: Operand, rhs: Operand },
    Op3 { op: Op, a: Operand, b: Operand, c: Operand },

    // Composite construction
    Construct {
        ty: TyId,
        fields: Vec<(FieldIdx, Operand, UseMode)>,
    },
    Tuple(Vec<(Operand, UseMode)>),
    EnumVariant {
        enum_ty: TyId,
        variant: VariantIdx,
        payload: Vec<(Operand, UseMode)>,
    },
    ArrayLiteral {
        element_ty: TyId,
        values: Vec<(Operand, UseMode)>,
    },
    ApplyPartial {
        func: Entity,
        captures: Vec<(Operand, UseMode)>,
    },
}

impl Rvalue {
    pub fn operands(&self) -> impl Iterator<Item = &Operand> {
        let mut out: SmallVec<[&Operand; 4]> = SmallVec::new();
        match self {
            Rvalue::Use(op, _) => out.push(op),
            Rvalue::Ref(_) | Rvalue::RefMut(_) => {}
            Rvalue::Op1 { arg, .. } => out.push(arg),
            Rvalue::Op2 { lhs, rhs, .. } => {
                out.push(lhs);
                out.push(rhs);
            }
            Rvalue::Op3 { a, b, c, .. } => {
                out.push(a);
                out.push(b);
                out.push(c);
            }
            Rvalue::Construct { fields, .. } => {
                for (_, op, _) in fields {
                    out.push(op);
                }
            }
            Rvalue::Tuple(elems) => {
                for (op, _) in elems {
                    out.push(op);
                }
            }
            Rvalue::EnumVariant { payload, .. } => {
                for (op, _) in payload {
                    out.push(op);
                }
            }
            Rvalue::ArrayLiteral { values, .. } => {
                for (op, _) in values {
                    out.push(op);
                }
            }
            Rvalue::ApplyPartial { captures, .. } => {
                for (op, _) in captures {
                    out.push(op);
                }
            }
        }
        out.into_iter()
    }

    pub fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        let mut out: SmallVec<[&mut Operand; 4]> = SmallVec::new();
        match self {
            Rvalue::Use(op, _) => out.push(op),
            Rvalue::Ref(_) | Rvalue::RefMut(_) => {}
            Rvalue::Op1 { arg, .. } => out.push(arg),
            Rvalue::Op2 { lhs, rhs, .. } => {
                out.push(lhs);
                out.push(rhs);
            }
            Rvalue::Op3 { a, b, c, .. } => {
                out.push(a);
                out.push(b);
                out.push(c);
            }
            Rvalue::Construct { fields, .. } => {
                for (_, op, _) in fields {
                    out.push(op);
                }
            }
            Rvalue::Tuple(elems) => {
                for (op, _) in elems {
                    out.push(op);
                }
            }
            Rvalue::EnumVariant { payload, .. } => {
                for (op, _) in payload {
                    out.push(op);
                }
            }
            Rvalue::ArrayLiteral { values, .. } => {
                for (op, _) in values {
                    out.push(op);
                }
            }
            Rvalue::ApplyPartial { captures, .. } => {
                for (op, _) in captures {
                    out.push(op);
                }
            }
        }
        out.into_iter()
    }

    /// Yields each operand paired with its UseMode (None for Op operands
    /// which read without consuming).
    pub fn operands_with_mode(&self) -> impl Iterator<Item = (&Operand, Option<UseMode>)> {
        let mut out: SmallVec<[(&Operand, Option<UseMode>); 4]> = SmallVec::new();
        match self {
            Rvalue::Use(op, mode) => out.push((op, Some(*mode))),
            Rvalue::Ref(_) | Rvalue::RefMut(_) => {}
            Rvalue::Op1 { arg, .. } => out.push((arg, None)),
            Rvalue::Op2 { lhs, rhs, .. } => {
                out.push((lhs, None));
                out.push((rhs, None));
            }
            Rvalue::Op3 { a, b, c, .. } => {
                out.push((a, None));
                out.push((b, None));
                out.push((c, None));
            }
            Rvalue::Construct { fields, .. } => {
                for (_, op, mode) in fields {
                    out.push((op, Some(*mode)));
                }
            }
            Rvalue::Tuple(elems) => {
                for (op, mode) in elems {
                    out.push((op, Some(*mode)));
                }
            }
            Rvalue::EnumVariant { payload, .. } => {
                for (op, mode) in payload {
                    out.push((op, Some(*mode)));
                }
            }
            Rvalue::ArrayLiteral { values, .. } => {
                for (op, mode) in values {
                    out.push((op, Some(*mode)));
                }
            }
            Rvalue::ApplyPartial { captures, .. } => {
                for (op, mode) in captures {
                    out.push((op, Some(*mode)));
                }
            }
        }
        out.into_iter()
    }

    /// Places used in Ref/RefMut — these are live uses for liveness analysis
    /// but don't appear in operands() (which yields Operand, not Place).
    pub fn referenced_places(&self) -> SmallVec<[&crate::Place; 1]> {
        match self {
            Rvalue::Ref(place) | Rvalue::RefMut(place) => {
                let mut v = SmallVec::new();
                v.push(place);
                v
            }
            _ => SmallVec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WitnessMethodKey {
    pub name: String,
    pub labels: Vec<Option<String>>,
}

impl WitnessMethodKey {
    pub fn new(name: impl Into<String>, labels: Vec<Option<String>>) -> Self {
        Self {
            name: name.into(),
            labels,
        }
    }

    pub fn simple(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            labels: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Callee {
    Direct {
        func: Entity,
        type_args: Vec<TyId>,
        self_type: Option<TyId>,
    },
    Thin(Place),
    Thick(Place),
    Witness {
        protocol: Entity,
        method: WitnessMethodKey,
        self_type: TyId,
        method_type_args: Vec<TyId>,
    },
}

impl Callee {
    pub fn direct(func: Entity) -> Self {
        Self::Direct {
            func,
            type_args: vec![],
            self_type: None,
        }
    }

    pub fn direct_with_args(func: Entity, type_args: Vec<TyId>, self_type: Option<TyId>) -> Self {
        Self::Direct {
            func,
            type_args,
            self_type,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatementKind {
    Assign { dest: Place, rvalue: Rvalue },
    Call {
        dest: Option<Place>,
        callee: Callee,
        args: Vec<(Operand, ArgMode)>,
    },
    Drop { place: Place },
    DropIf { place: Place, flag: LocalId },
    SetDropFlag { flag: LocalId, value: bool },
    ScopeLive(LocalId),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Statement {
    pub kind: StatementKind,
    pub span: Option<Span>,
}

impl Statement {
    pub fn new(kind: StatementKind) -> Self {
        Self { kind, span: None }
    }

    pub fn with_span(kind: StatementKind, span: Span) -> Self {
        Self {
            kind,
            span: Some(span),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::immediate::Immediate;
    use crate::place::Place;
    use crate::{IntBits, Signedness};

    fn local(n: usize) -> LocalId {
        LocalId::new(n)
    }

    fn place(n: usize) -> Place {
        Place::local(local(n))
    }

    fn op(n: usize) -> Operand {
        Operand::Place(place(n))
    }

    // --- Rvalue::operands tests ---

    #[test]
    fn operands_use() {
        let rv = Rvalue::Use(op(0), UseMode::Copy);
        assert_eq!(rv.operands().count(), 1);
    }

    #[test]
    fn operands_ref_empty() {
        let rv = Rvalue::Ref(place(0));
        assert_eq!(rv.operands().count(), 0);
    }

    #[test]
    fn operands_ref_mut_empty() {
        let rv = Rvalue::RefMut(place(0));
        assert_eq!(rv.operands().count(), 0);
    }

    #[test]
    fn operands_op1() {
        let rv = Rvalue::Op1 {
            op: Op::Neg(IntBits::I64),
            arg: op(0),
        };
        assert_eq!(rv.operands().count(), 1);
    }

    #[test]
    fn operands_op2() {
        let rv = Rvalue::Op2 {
            op: Op::Add(IntBits::I64, Signedness::Signed),
            lhs: op(0),
            rhs: op(1),
        };
        assert_eq!(rv.operands().count(), 2);
    }

    #[test]
    fn operands_op3() {
        let rv = Rvalue::Op3 {
            op: Op::FloatFma(crate::FloatBits::F64),
            a: op(0),
            b: op(1),
            c: op(2),
        };
        assert_eq!(rv.operands().count(), 3);
    }

    #[test]
    fn operands_construct() {
        let rv = Rvalue::Construct {
            ty: TyId::new(0),
            fields: vec![
                (crate::FieldIdx::new(0), op(0), UseMode::Copy),
                (crate::FieldIdx::new(1), op(1), UseMode::Move),
            ],
        };
        assert_eq!(rv.operands().count(), 2);
    }

    #[test]
    fn operands_tuple() {
        let rv = Rvalue::Tuple(vec![
            (op(0), UseMode::Copy),
            (op(1), UseMode::Move),
            (op(2), UseMode::Copy),
        ]);
        assert_eq!(rv.operands().count(), 3);
    }

    #[test]
    fn operands_enum_variant() {
        let rv = Rvalue::EnumVariant {
            enum_ty: TyId::new(0),
            variant: crate::VariantIdx::new(1),
            payload: vec![(op(0), UseMode::Move)],
        };
        assert_eq!(rv.operands().count(), 1);
    }

    #[test]
    fn operands_array_literal() {
        let rv = Rvalue::ArrayLiteral {
            element_ty: TyId::new(0),
            values: vec![(op(0), UseMode::Copy), (op(1), UseMode::Copy)],
        };
        assert_eq!(rv.operands().count(), 2);
    }

    #[test]
    fn operands_apply_partial() {
        let rv = Rvalue::ApplyPartial {
            func: Entity::from_raw(1),
            captures: vec![(op(0), UseMode::Move)],
        };
        assert_eq!(rv.operands().count(), 1);
    }

    // --- operands_with_mode tests ---

    #[test]
    fn operands_with_mode_use() {
        let rv = Rvalue::Use(op(0), UseMode::Move);
        let modes: Vec<_> = rv.operands_with_mode().collect();
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0].1, Some(UseMode::Move));
    }

    #[test]
    fn operands_with_mode_op_is_none() {
        let rv = Rvalue::Op1 {
            op: Op::Neg(IntBits::I64),
            arg: op(0),
        };
        let modes: Vec<_> = rv.operands_with_mode().collect();
        assert_eq!(modes[0].1, None);
    }

    #[test]
    fn operands_with_mode_construct() {
        let rv = Rvalue::Construct {
            ty: TyId::new(0),
            fields: vec![
                (crate::FieldIdx::new(0), op(0), UseMode::Copy),
                (crate::FieldIdx::new(1), op(1), UseMode::Move),
            ],
        };
        let modes: Vec<_> = rv.operands_with_mode().collect();
        assert_eq!(modes[0].1, Some(UseMode::Copy));
        assert_eq!(modes[1].1, Some(UseMode::Move));
    }

    // --- operands_mut test ---

    #[test]
    fn operands_mut_can_modify() {
        let mut rv = Rvalue::Use(op(0), UseMode::Copy);
        for operand in rv.operands_mut() {
            *operand = Operand::Const(Immediate::i64(99));
        }
        match &rv {
            Rvalue::Use(Operand::Const(imm), _) => {
                assert_eq!(imm.kind, crate::ImmediateKind::IntLiteral { bits: IntBits::I64, value: 99 });
            }
            other => panic!("expected modified Use, got {other:?}"),
        }
    }

    // --- WitnessMethodKey tests ---

    #[test]
    fn witness_key_equality() {
        let a = WitnessMethodKey::simple("equals");
        let b = WitnessMethodKey::simple("equals");
        assert_eq!(a, b);
    }

    #[test]
    fn witness_key_with_labels() {
        let a = WitnessMethodKey::new("insert", vec![Some("at".into()), None]);
        let b = WitnessMethodKey::new("insert", vec![Some("at".into()), None]);
        let c = WitnessMethodKey::new("insert", vec![None, None]);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // --- Callee tests ---

    #[test]
    fn callee_direct() {
        let c = Callee::direct(Entity::from_raw(1));
        match &c {
            Callee::Direct {
                func,
                type_args,
                self_type,
            } => {
                assert_eq!(*func, Entity::from_raw(1));
                assert!(type_args.is_empty());
                assert!(self_type.is_none());
            }
            other => panic!("expected Direct, got {other:?}"),
        }
    }

    // --- Statement tests ---

    #[test]
    fn statement_no_span() {
        let stmt = Statement::new(StatementKind::ScopeLive(local(0)));
        assert!(stmt.span.is_none());
    }

    #[test]
    fn statement_assign() {
        let stmt = Statement::new(StatementKind::Assign {
            dest: place(0),
            rvalue: Rvalue::Use(op(1), UseMode::Move),
        });
        matches!(stmt.kind, StatementKind::Assign { .. });
    }

    #[test]
    fn statement_drop() {
        let stmt = Statement::new(StatementKind::Drop { place: place(0) });
        matches!(stmt.kind, StatementKind::Drop { .. });
    }

    #[test]
    fn statement_drop_if() {
        let stmt = Statement::new(StatementKind::DropIf {
            place: place(0),
            flag: local(1),
        });
        matches!(stmt.kind, StatementKind::DropIf { .. });
    }

    #[test]
    fn statement_set_drop_flag() {
        let stmt = Statement::new(StatementKind::SetDropFlag {
            flag: local(0),
            value: true,
        });
        matches!(stmt.kind, StatementKind::SetDropFlag { .. });
    }
}
