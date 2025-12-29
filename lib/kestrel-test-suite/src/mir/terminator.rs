//! Terminator patterns for MIR testing.

use kestrel_execution_graph::{BasicBlock, Id, Block, MirContext, TerminatorKind};

/// Pattern for matching terminators in MIR.
#[derive(Debug, Clone)]
pub enum TerminatorPattern {
    /// return <value>
    Return,

    /// jump bbN
    Jump,

    /// jump to specific block (0-indexed)
    JumpTo(usize),

    /// branch if ...
    Branch,

    /// branch to specific blocks (0-indexed)
    BranchTo { then_block: usize, else_block: usize },

    /// switch on discriminant
    Switch,

    /// switch with specific case names
    SwitchCases(Vec<String>),

    /// panic
    Panic,

    /// unreachable
    Unreachable,
}

impl TerminatorPattern {
    /// Check if this pattern matches a terminator.
    pub(crate) fn matches(
        &self,
        block: &BasicBlock,
        all_blocks: &[Id<Block>],
        _ctx: &MirContext,
    ) -> bool {
        let Some(terminator) = &block.terminator else {
            return false;
        };

        match (&self, &terminator.kind) {
            (TerminatorPattern::Return, TerminatorKind::Return(_)) => true,
            (TerminatorPattern::Jump, TerminatorKind::Jump(_)) => true,
            (TerminatorPattern::JumpTo(expected_idx), TerminatorKind::Jump(target)) => {
                block_index(*target, all_blocks) == Some(*expected_idx)
            }
            (TerminatorPattern::Branch, TerminatorKind::Branch { .. }) => true,
            (
                TerminatorPattern::BranchTo {
                    then_block: expected_then,
                    else_block: expected_else,
                },
                TerminatorKind::Branch {
                    then_block: actual_then,
                    else_block: actual_else,
                    ..
                },
            ) => {
                block_index(*actual_then, all_blocks) == Some(*expected_then)
                    && block_index(*actual_else, all_blocks) == Some(*expected_else)
            }
            (TerminatorPattern::Switch, TerminatorKind::Switch { .. }) => true,
            (TerminatorPattern::SwitchCases(expected_cases), TerminatorKind::Switch { cases, .. }) => {
                if cases.len() != expected_cases.len() {
                    return false;
                }
                // Check that all expected case names are present
                let actual_case_names: Vec<_> = cases.iter().map(|(name, _)| name.clone()).collect();
                expected_cases.iter().all(|e| actual_case_names.contains(e))
            }
            (TerminatorPattern::Panic, TerminatorKind::Panic(_)) => true,
            (TerminatorPattern::Unreachable, TerminatorKind::Unreachable) => true,
            _ => false,
        }
    }

    /// Format this pattern for display in error messages.
    pub(crate) fn display(&self) -> String {
        match self {
            TerminatorPattern::Return => "return".to_string(),
            TerminatorPattern::Jump => "jump".to_string(),
            TerminatorPattern::JumpTo(idx) => format!("jump to bb{}", idx),
            TerminatorPattern::Branch => "branch".to_string(),
            TerminatorPattern::BranchTo {
                then_block,
                else_block,
            } => format!("branch to bb{}/bb{}", then_block, else_block),
            TerminatorPattern::Switch => "switch".to_string(),
            TerminatorPattern::SwitchCases(cases) => format!("switch with cases {:?}", cases),
            TerminatorPattern::Panic => "panic".to_string(),
            TerminatorPattern::Unreachable => "unreachable".to_string(),
        }
    }
}

/// Get the 0-based index of a block in the function's block list.
fn block_index(block_id: Id<Block>, all_blocks: &[Id<Block>]) -> Option<usize> {
    all_blocks.iter().position(|&b| b == block_id)
}
