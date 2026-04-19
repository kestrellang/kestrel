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
        cases: Vec<(SwitchCase, BlockId)>,
    },

    /// `panic "message"`
    Panic(String),

    /// `unreachable` — control flow should never reach here
    Unreachable,
}

/// A single arm of a `switch` terminator.
///
/// Replaces the earlier `String`-keyed scheme so codegen can test ranges
/// and literal values without parsing case names.
#[derive(Debug, Clone)]
pub enum SwitchCase {
    /// Default / wildcard arm — always matches. Emitted as an unconditional
    /// jump at codegen.
    Wildcard,
    /// Enum variant, resolved by name against the enum definition.
    Variant(String),
    /// Boolean constant (True/False).
    Bool(bool),
    /// Exact integer literal.
    IntLiteral(i64),
    /// Integer range with inclusive bounds; `None` means the bound is open.
    IntRange { start: Option<i64>, end: Option<i64> },
    /// Character literal as a Unicode codepoint.
    CharLiteral(u32),
    /// Character range with inclusive bounds.
    CharRange { start: Option<u32>, end: Option<u32> },
    /// String literal (byte-equality test).
    StringLiteral(String),
}

impl SwitchCase {
    /// True if this arm unconditionally matches (wildcard).
    pub fn is_wildcard(&self) -> bool {
        matches!(self, SwitchCase::Wildcard)
    }
}

impl std::fmt::Display for SwitchCase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwitchCase::Wildcard => write!(f, "_"),
            SwitchCase::Variant(name) => write!(f, "{}", name),
            SwitchCase::Bool(b) => write!(f, "{}", b),
            SwitchCase::IntLiteral(n) => write!(f, "{}", n),
            SwitchCase::IntRange { start, end } => {
                if let Some(s) = start { write!(f, "{}", s)?; }
                write!(f, "..=")?;
                if let Some(e) = end { write!(f, "{}", e)?; }
                Ok(())
            }
            SwitchCase::CharLiteral(c) => {
                match char::from_u32(*c) {
                    Some(ch) => write!(f, "'{}'", ch),
                    None => write!(f, "\\u{{{:x}}}", c),
                }
            }
            SwitchCase::CharRange { start, end } => {
                if let Some(s) = start {
                    if let Some(ch) = char::from_u32(*s) {
                        write!(f, "'{}'", ch)?;
                    } else {
                        write!(f, "\\u{{{:x}}}", s)?;
                    }
                }
                write!(f, "..=")?;
                if let Some(e) = end {
                    if let Some(ch) = char::from_u32(*e) {
                        write!(f, "'{}'", ch)?;
                    } else {
                        write!(f, "\\u{{{:x}}}", e)?;
                    }
                }
                Ok(())
            }
            SwitchCase::StringLiteral(s) => write!(f, "{:?}", s),
        }
    }
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

    pub fn switch(discriminant: Place, cases: Vec<(SwitchCase, BlockId)>) -> Self {
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
