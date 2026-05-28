use kestrel_hecs::Entity;
use kestrel_span::Span;
use smallvec::SmallVec;

use crate::callee::Callee;
use crate::immediate::Immediate;
use crate::op::Op;
use crate::ty::ParamConvention;
use crate::{FieldIdx, TyId, ValueId, VariantIdx};

#[derive(Debug, Clone, PartialEq)]
pub struct CallArg {
    pub value: ValueId,
    pub convention: ParamConvention,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Instruction {
    pub kind: InstKind,
    pub span: Option<Span>,
}

impl Instruction {
    pub fn new(kind: InstKind) -> Self {
        Self { kind, span: None }
    }

    pub fn with_span(kind: InstKind, span: Span) -> Self {
        Self {
            kind,
            span: Some(span),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstKind {
    // -- Value lifecycle --
    CopyValue {
        result: ValueId,
        operand: ValueId,
    },
    MoveValue {
        result: ValueId,
        operand: ValueId,
    },
    DestroyValue {
        operand: ValueId,
    },

    // -- Borrowing --
    BeginBorrow {
        result: ValueId,
        operand: ValueId,
    },
    EndBorrow {
        operand: ValueId,
    },
    BeginMutBorrow {
        result: ValueId,
        operand: ValueId,
    },
    EndMutBorrow {
        operand: ValueId,
    },

    // -- Memory access --
    Load {
        result: ValueId,
        address: ValueId,
    },
    CopyAddr {
        result: ValueId,
        address: ValueId,
        ty: TyId,
    },
    Take {
        result: ValueId,
        address: ValueId,
        ty: TyId,
    },
    BeginBorrowAddr {
        result: ValueId,
        address: ValueId,
        ty: TyId,
    },
    BeginMutBorrowAddr {
        result: ValueId,
        address: ValueId,
        ty: TyId,
    },
    StoreInit {
        address: ValueId,
        value: ValueId,
    },
    StoreAssign {
        address: ValueId,
        value: ValueId,
    },
    DestroyAddr {
        address: ValueId,
        ty: TyId,
    },

    // -- Enum discriminant --
    Discriminant {
        result: ValueId,
        operand: ValueId,
    },

    // -- Computation --
    Op1 {
        result: ValueId,
        op: Op,
        arg: ValueId,
    },
    Op2 {
        result: ValueId,
        op: Op,
        lhs: ValueId,
        rhs: ValueId,
    },
    Op3 {
        result: ValueId,
        op: Op,
        a: ValueId,
        b: ValueId,
        c: ValueId,
    },

    // -- Constants --
    Literal {
        result: ValueId,
        value: Immediate,
    },
    GlobalRef {
        result: ValueId,
        entity: Entity,
    },

    // -- Aggregates: construction --
    Struct {
        result: ValueId,
        ty: TyId,
        fields: Vec<(FieldIdx, ValueId)>,
    },
    Tuple {
        result: ValueId,
        elements: Vec<ValueId>,
    },
    Enum {
        result: ValueId,
        enum_ty: TyId,
        variant: VariantIdx,
        payload: Vec<ValueId>,
    },
    Array {
        result: ValueId,
        element_ty: TyId,
        elements: Vec<ValueId>,
    },

    // -- Aggregates: destructuring --
    StructExtract {
        result: ValueId,
        operand: ValueId,
        field: FieldIdx,
    },
    TupleExtract {
        result: ValueId,
        operand: ValueId,
        index: u32,
    },
    EnumPayload {
        result: ValueId,
        operand: ValueId,
        variant: VariantIdx,
        field: FieldIdx,
    },
    DestructureStruct {
        results: Vec<ValueId>,
        operand: ValueId,
    },
    DestructureTuple {
        results: Vec<ValueId>,
        operand: ValueId,
    },
    DestructureEnum {
        results: Vec<ValueId>,
        operand: ValueId,
        variant: VariantIdx,
    },

    // -- Calls --
    Call {
        result: Option<ValueId>,
        callee: Callee,
        args: Vec<CallArg>,
    },
    ApplyPartial {
        result: ValueId,
        func: Entity,
        captures: Vec<ValueId>,
    },

    // -- Address projection --
    FieldAddr {
        result: ValueId,
        base: ValueId,
        ty: TyId,
        field: FieldIdx,
    },

    // -- Special --
    Uninit {
        result: ValueId,
        ty: TyId,
    },
}

impl InstKind {
    /// Returns the single result ValueId, if this instruction produces exactly one.
    pub fn result(&self) -> Option<ValueId> {
        match self {
            InstKind::CopyValue { result, .. }
            | InstKind::MoveValue { result, .. }
            | InstKind::BeginBorrow { result, .. }
            | InstKind::BeginMutBorrow { result, .. }
            | InstKind::Load { result, .. }
            | InstKind::CopyAddr { result, .. }
            | InstKind::Take { result, .. }
            | InstKind::BeginBorrowAddr { result, .. }
            | InstKind::BeginMutBorrowAddr { result, .. }
            | InstKind::Discriminant { result, .. }
            | InstKind::Op1 { result, .. }
            | InstKind::Op2 { result, .. }
            | InstKind::Op3 { result, .. }
            | InstKind::Literal { result, .. }
            | InstKind::GlobalRef { result, .. }
            | InstKind::Struct { result, .. }
            | InstKind::Tuple { result, .. }
            | InstKind::Enum { result, .. }
            | InstKind::Array { result, .. }
            | InstKind::StructExtract { result, .. }
            | InstKind::TupleExtract { result, .. }
            | InstKind::EnumPayload { result, .. }
            | InstKind::ApplyPartial { result, .. }
            | InstKind::FieldAddr { result, .. }
            | InstKind::Uninit { result, .. } => Some(*result),
            InstKind::Call { result, .. } => *result,
            InstKind::DestroyValue { .. }
            | InstKind::EndBorrow { .. }
            | InstKind::EndMutBorrow { .. }
            | InstKind::StoreInit { .. }
            | InstKind::StoreAssign { .. }
            | InstKind::DestroyAddr { .. }
            | InstKind::DestructureStruct { .. }
            | InstKind::DestructureTuple { .. }
            | InstKind::DestructureEnum { .. } => None,
        }
    }

    /// Returns all result ValueIds (handles multi-result destructure variants).
    pub fn results(&self) -> SmallVec<[ValueId; 4]> {
        match self {
            InstKind::DestructureStruct { results, .. }
            | InstKind::DestructureTuple { results, .. }
            | InstKind::DestructureEnum { results, .. } => results.iter().copied().collect(),
            other => {
                if let Some(r) = other.result() {
                    SmallVec::from_elem(r, 1)
                } else {
                    SmallVec::new()
                }
            },
        }
    }

    /// Returns all operand ValueIds read by this instruction.
    pub fn operands(&self) -> SmallVec<[ValueId; 4]> {
        match self {
            InstKind::CopyValue { operand, .. }
            | InstKind::MoveValue { operand, .. }
            | InstKind::DestroyValue { operand }
            | InstKind::BeginBorrow { operand, .. }
            | InstKind::EndBorrow { operand }
            | InstKind::BeginMutBorrow { operand, .. }
            | InstKind::EndMutBorrow { operand }
            | InstKind::Discriminant { operand, .. }
            | InstKind::StructExtract { operand, .. }
            | InstKind::TupleExtract { operand, .. }
            | InstKind::EnumPayload { operand, .. }
            | InstKind::DestructureStruct { operand, .. }
            | InstKind::DestructureTuple { operand, .. }
            | InstKind::DestructureEnum { operand, .. } => SmallVec::from_elem(*operand, 1),
            InstKind::Load { address, .. } => SmallVec::from_elem(*address, 1),
            InstKind::CopyAddr { address, .. }
            | InstKind::Take { address, .. }
            | InstKind::BeginBorrowAddr { address, .. }
            | InstKind::BeginMutBorrowAddr { address, .. }
            | InstKind::DestroyAddr { address, .. } => SmallVec::from_elem(*address, 1),
            InstKind::StoreInit { address, value } | InstKind::StoreAssign { address, value } => {
                smallvec::smallvec![*address, *value]
            },
            InstKind::Op1 { arg, .. } => SmallVec::from_elem(*arg, 1),
            InstKind::Op2 { lhs, rhs, .. } => smallvec::smallvec![*lhs, *rhs],
            InstKind::Op3 { a, b, c, .. } => smallvec::smallvec![*a, *b, *c],
            InstKind::Literal { .. } | InstKind::GlobalRef { .. } => SmallVec::new(),
            InstKind::Struct { fields, .. } => fields.iter().map(|(_, v)| *v).collect(),
            InstKind::Tuple { elements, .. } | InstKind::Array { elements, .. } => {
                elements.iter().copied().collect()
            },
            InstKind::Enum { payload, .. } => payload.iter().copied().collect(),
            InstKind::Call { args, .. } => args.iter().map(|a| a.value).collect(),
            InstKind::ApplyPartial { captures, .. } => captures.iter().copied().collect(),
            InstKind::FieldAddr { base, .. } => SmallVec::from_elem(*base, 1),
            InstKind::Uninit { .. } => SmallVec::new(),
        }
    }
}
