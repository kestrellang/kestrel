//! Block terminators — how a basic block exits.

use crate::id::BlockId;
use crate::place::Place;
use crate::value::Value;
use kestrel_span2::Span;

/// A terminator ends a basic block. Every block must have exactly one.
#[derive(Debug, Clone)]
pub struct Terminator {
    pub kind: TerminatorKind,
    pub span: Option<Span>,
}

/// The different kinds of terminators.
#[derive(Debug, Clone)]
pub enum TerminatorKind {
    /// `return <value>`
    Return(Value),

    /// `jump bb`
    Jump(BlockId),

    /// `branch if <cond>, bb_true else bb_false`
    Branch {
        condition: Value,
        then_block: BlockId,
        else_block: BlockId,
    },

    /// `switch <discriminant> { Case => bb, ... }`
    Switch {
        discriminant: Place,
        cases: Vec<(String, BlockId)>,
    },

    /// `panic "message"`
    Panic(String),

    /// `unreachable` — control flow should never reach here
    Unreachable,
}

impl Terminator {
    pub fn ret(value: impl Into<Value>) -> Self {
        Self {
            kind: TerminatorKind::Return(value.into()),
            span: None,
        }
    }

    pub fn jump(target: BlockId) -> Self {
        Self {
            kind: TerminatorKind::Jump(target),
            span: None,
        }
    }

    pub fn branch(
        condition: impl Into<Value>,
        then_block: BlockId,
        else_block: BlockId,
    ) -> Self {
        Self {
            kind: TerminatorKind::Branch {
                condition: condition.into(),
                then_block,
                else_block,
            },
            span: None,
        }
    }

    pub fn switch(discriminant: Place, cases: Vec<(String, BlockId)>) -> Self {
        Self {
            kind: TerminatorKind::Switch {
                discriminant,
                cases,
            },
            span: None,
        }
    }

    pub fn panic(message: impl Into<String>) -> Self {
        Self {
            kind: TerminatorKind::Panic(message.into()),
            span: None,
        }
    }

    pub fn unreachable() -> Self {
        Self {
            kind: TerminatorKind::Unreachable,
            span: None,
        }
    }

    /// Get the successor blocks of this terminator.
    pub fn successors(&self) -> Vec<BlockId> {
        match &self.kind {
            TerminatorKind::Return(_) | TerminatorKind::Panic(_) | TerminatorKind::Unreachable => {
                vec![]
            },
            TerminatorKind::Jump(target) => vec![*target],
            TerminatorKind::Branch {
                then_block,
                else_block,
                ..
            } => vec![*then_block, *else_block],
            TerminatorKind::Switch { cases, .. } => cases.iter().map(|(_, b)| *b).collect(),
        }
    }
}
