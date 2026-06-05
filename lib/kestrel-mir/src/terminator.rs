use kestrel_span::Span;
use smallvec::SmallVec;

use crate::{BlockId, ValueId, VariantIdx};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SwitchCase {
    Wildcard,
    Variant(VariantIdx),
    Bool(bool),
    IntLiteral(i64),
    IntRange { start: i64, end: i64 },
    CharLiteral(u32),
    CharRange { start: u32, end: u32 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchArm {
    pub pattern: SwitchCase,
    pub target: BlockId,
    pub args: Vec<ValueId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Terminator {
    pub kind: TerminatorKind,
    pub span: Option<Span>,
}

impl Terminator {
    pub fn new(kind: TerminatorKind) -> Self {
        Self { kind, span: None }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TerminatorKind {
    Return(ValueId),
    Jump {
        target: BlockId,
        args: Vec<ValueId>,
    },
    Branch {
        condition: ValueId,
        then_block: BlockId,
        then_args: Vec<ValueId>,
        else_block: BlockId,
        else_args: Vec<ValueId>,
    },
    Switch {
        discriminant: ValueId,
        cases: Vec<SwitchArm>,
    },
    Panic(String),
    Unreachable,
}

impl TerminatorKind {
    /// Returns all successor block IDs.
    pub fn successors(&self) -> SmallVec<[BlockId; 2]> {
        match self {
            TerminatorKind::Return(_) | TerminatorKind::Panic(_) | TerminatorKind::Unreachable => {
                SmallVec::new()
            },
            TerminatorKind::Jump { target, .. } => SmallVec::from_elem(*target, 1),
            TerminatorKind::Branch {
                then_block,
                else_block,
                ..
            } => {
                smallvec::smallvec![*then_block, *else_block]
            },
            TerminatorKind::Switch { cases, .. } => cases.iter().map(|arm| arm.target).collect(),
        }
    }

    /// Returns all ValueIds used by this terminator.
    pub fn operands(&self) -> SmallVec<[ValueId; 4]> {
        match self {
            TerminatorKind::Return(v) => SmallVec::from_elem(*v, 1),
            TerminatorKind::Jump { args, .. } => args.iter().copied().collect(),
            TerminatorKind::Branch {
                condition,
                then_args,
                else_args,
                ..
            } => {
                let mut ops = SmallVec::new();
                ops.push(*condition);
                ops.extend(then_args.iter().copied());
                ops.extend(else_args.iter().copied());
                ops
            },
            TerminatorKind::Switch {
                discriminant,
                cases,
            } => {
                let mut ops = SmallVec::new();
                ops.push(*discriminant);
                for arm in cases {
                    ops.extend(arm.args.iter().copied());
                }
                ops
            },
            TerminatorKind::Panic(_) | TerminatorKind::Unreachable => SmallVec::new(),
        }
    }

    /// Returns (successor_block, args) pairs for block argument checking.
    pub fn successor_args(&self) -> SmallVec<[(BlockId, &[ValueId]); 2]> {
        match self {
            TerminatorKind::Return(_) | TerminatorKind::Panic(_) | TerminatorKind::Unreachable => {
                SmallVec::new()
            },
            TerminatorKind::Jump { target, args } => {
                smallvec::smallvec![(*target, args.as_slice())]
            },
            TerminatorKind::Branch {
                then_block,
                then_args,
                else_block,
                else_args,
                ..
            } => {
                smallvec::smallvec![
                    (*then_block, then_args.as_slice()),
                    (*else_block, else_args.as_slice()),
                ]
            },
            TerminatorKind::Switch { cases, .. } => cases
                .iter()
                .map(|arm| (arm.target, arm.args.as_slice()))
                .collect(),
        }
    }
}
