//! Block terminators.

use crate::function::{Place, Value};
use crate::id::{Block, Id};
use crate::metadata::Metadata;
use crate::MirContext;
use std::fmt;

/// A terminator ends a basic block.
#[derive(Debug, Clone)]
pub struct Terminator {
    pub meta: Metadata,
    pub kind: TerminatorKind,
}

/// The different kinds of terminators.
#[derive(Debug, Clone)]
pub enum TerminatorKind {
    /// `return <value>`
    Return(Value),

    /// `jump bb`
    Jump(Id<Block>),

    /// `branch if <cond>, bb_true else bb_false`
    Branch {
        condition: Value,
        then_block: Id<Block>,
        else_block: Id<Block>,
    },

    /// `switch <place> { Case => bb, ... }`
    Switch {
        discriminant: Place,
        cases: Vec<(String, Id<Block>)>,
    },

    /// `panic "message"`
    Panic(String),

    /// `unreachable`
    Unreachable,
}

impl Terminator {
    /// Create a return terminator.
    pub fn ret(value: impl Into<Value>) -> Self {
        Self {
            meta: Metadata::new(),
            kind: TerminatorKind::Return(value.into()),
        }
    }

    /// Create a jump terminator.
    pub fn jump(target: Id<Block>) -> Self {
        Self {
            meta: Metadata::new(),
            kind: TerminatorKind::Jump(target),
        }
    }

    /// Create a branch terminator.
    pub fn branch(
        condition: impl Into<Value>,
        then_block: Id<Block>,
        else_block: Id<Block>,
    ) -> Self {
        Self {
            meta: Metadata::new(),
            kind: TerminatorKind::Branch {
                condition: condition.into(),
                then_block,
                else_block,
            },
        }
    }

    /// Create a switch terminator.
    pub fn switch(discriminant: Place, cases: Vec<(String, Id<Block>)>) -> Self {
        Self {
            meta: Metadata::new(),
            kind: TerminatorKind::Switch {
                discriminant,
                cases,
            },
        }
    }

    /// Create a panic terminator.
    pub fn panic(message: impl Into<String>) -> Self {
        Self {
            meta: Metadata::new(),
            kind: TerminatorKind::Panic(message.into()),
        }
    }

    /// Create an unreachable terminator.
    pub fn unreachable() -> Self {
        Self {
            meta: Metadata::new(),
            kind: TerminatorKind::Unreachable,
        }
    }

    /// Get the successor blocks of this terminator.
    pub fn successors(&self) -> Vec<Id<Block>> {
        match &self.kind {
            TerminatorKind::Return(_) => vec![],
            TerminatorKind::Jump(target) => vec![*target],
            TerminatorKind::Branch {
                then_block,
                else_block,
                ..
            } => vec![*then_block, *else_block],
            TerminatorKind::Switch { cases, .. } => cases.iter().map(|(_, b)| *b).collect(),
            TerminatorKind::Panic(_) => vec![],
            TerminatorKind::Unreachable => vec![],
        }
    }

    /// Create a display wrapper for printing this terminator.
    pub fn display<'a>(
        &'a self,
        ctx: &'a MirContext,
        blocks: &'a [Id<Block>],
    ) -> impl fmt::Display + 'a {
        TerminatorDisplay {
            term: self,
            ctx,
            blocks,
        }
    }
}

struct TerminatorDisplay<'a> {
    term: &'a Terminator,
    ctx: &'a MirContext,
    blocks: &'a [Id<Block>],
}

impl fmt::Display for TerminatorDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let block_index =
            |id: Id<Block>| -> usize { self.blocks.iter().position(|&b| b == id).unwrap_or(0) };

        match &self.term.kind {
            TerminatorKind::Return(v) => {
                write!(f, "return {}", v.display(self.ctx))
            }
            TerminatorKind::Jump(target) => {
                write!(f, "jump bb{}", block_index(*target))
            }
            TerminatorKind::Branch {
                condition,
                then_block,
                else_block,
            } => {
                write!(
                    f,
                    "branch if {}, bb{} else bb{}",
                    condition.display(self.ctx),
                    block_index(*then_block),
                    block_index(*else_block)
                )
            }
            TerminatorKind::Switch {
                discriminant,
                cases,
            } => {
                writeln!(f, "switch {} {{", discriminant.display(self.ctx))?;
                for (case_name, target) in cases {
                    writeln!(f, "    {} => bb{}", case_name, block_index(*target))?;
                }
                write!(f, "}}")
            }
            TerminatorKind::Panic(msg) => {
                write!(f, "panic {:?}", msg)
            }
            TerminatorKind::Unreachable => {
                write!(f, "unreachable")
            }
        }
    }
}
