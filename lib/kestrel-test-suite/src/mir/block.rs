//! Block expectations for MIR testing.

use crate::mir::statement::StatementPattern;
use crate::mir::terminator::TerminatorPattern;
use kestrel_execution_graph::{BasicBlock, Block, Id, MirContext};

/// Expectations for a basic block in the MIR.
#[derive(Default)]
pub struct MirBlock {
    pub(crate) expectations: Vec<BlockExpectation>,
}

pub(crate) enum BlockExpectation {
    StatementCount(usize),
    AtLeastStatements(usize),
    HasStatement(StatementPattern),
    StatementAt(usize, StatementPattern),
    Terminates(TerminatorPattern),
    HasSuccessor(usize),
    SuccessorCount(usize),
}

impl MirBlock {
    pub fn new() -> Self {
        Self::default()
    }

    /// Expect exactly N statements in this block.
    pub fn has_statement_count(mut self, n: usize) -> Self {
        self.expectations.push(BlockExpectation::StatementCount(n));
        self
    }

    /// Expect at least N statements in this block.
    pub fn has_at_least_statements(mut self, n: usize) -> Self {
        self.expectations
            .push(BlockExpectation::AtLeastStatements(n));
        self
    }

    /// Expect a statement matching the pattern somewhere in this block.
    pub fn has_statement(mut self, pattern: StatementPattern) -> Self {
        self.expectations
            .push(BlockExpectation::HasStatement(pattern));
        self
    }

    /// Expect statement at index N to match the pattern (0-indexed).
    pub fn statement_at(mut self, index: usize, pattern: StatementPattern) -> Self {
        self.expectations
            .push(BlockExpectation::StatementAt(index, pattern));
        self
    }

    /// Expect the terminator to match the pattern.
    pub fn terminates_with(mut self, pattern: TerminatorPattern) -> Self {
        self.expectations
            .push(BlockExpectation::Terminates(pattern));
        self
    }

    /// Expect this block to have a successor at the given index (0-indexed).
    pub fn has_successor(mut self, block_index: usize) -> Self {
        self.expectations
            .push(BlockExpectation::HasSuccessor(block_index));
        self
    }

    /// Expect exactly N successor blocks.
    pub fn has_successor_count(mut self, n: usize) -> Self {
        self.expectations.push(BlockExpectation::SuccessorCount(n));
        self
    }

    /// Check all expectations against a block.
    pub(crate) fn check(
        &self,
        block_index: usize,
        block: &BasicBlock,
        all_blocks: &[Id<Block>],
        ctx: &MirContext,
    ) -> Result<(), String> {
        for expectation in &self.expectations {
            self.check_expectation(block_index, expectation, block, all_blocks, ctx)?;
        }
        Ok(())
    }

    fn check_expectation(
        &self,
        block_index: usize,
        expectation: &BlockExpectation,
        block: &BasicBlock,
        all_blocks: &[Id<Block>],
        ctx: &MirContext,
    ) -> Result<(), String> {
        match expectation {
            BlockExpectation::StatementCount(expected) => {
                let actual = block.statements.len();
                if actual != *expected {
                    return Err(format!(
                        "Block bb{} has {} statement(s), expected {}",
                        block_index, actual, expected
                    ));
                }
            },

            BlockExpectation::AtLeastStatements(expected) => {
                let actual = block.statements.len();
                if actual < *expected {
                    return Err(format!(
                        "Block bb{} has {} statement(s), expected at least {}",
                        block_index, actual, expected
                    ));
                }
            },

            BlockExpectation::HasStatement(pattern) => {
                let found = block.statements.iter().any(|stmt_id| {
                    let stmt = ctx.statement(*stmt_id);
                    pattern.matches(stmt, ctx)
                });
                if !found {
                    return Err(format!(
                        "Block bb{} does not contain statement matching '{}'",
                        block_index,
                        pattern.display()
                    ));
                }
            },

            BlockExpectation::StatementAt(idx, pattern) => {
                if *idx >= block.statements.len() {
                    return Err(format!(
                        "Block bb{} does not have statement at index {} (only {} statements)",
                        block_index,
                        idx,
                        block.statements.len()
                    ));
                }
                let stmt = ctx.statement(block.statements[*idx]);
                if !pattern.matches(stmt, ctx) {
                    return Err(format!(
                        "Block bb{} statement at index {} does not match '{}'. Actual: {:?}",
                        block_index,
                        idx,
                        pattern.display(),
                        stmt.kind
                    ));
                }
            },

            BlockExpectation::Terminates(pattern) => {
                if !pattern.matches(block, all_blocks, ctx) {
                    let actual = block
                        .terminator
                        .as_ref()
                        .map(|t| format!("{:?}", t.kind))
                        .unwrap_or_else(|| "no terminator".to_string());
                    return Err(format!(
                        "Block bb{} terminator does not match '{}'. Actual: {}",
                        block_index,
                        pattern.display(),
                        actual
                    ));
                }
            },

            BlockExpectation::HasSuccessor(expected_idx) => {
                let successors = block
                    .terminator
                    .as_ref()
                    .map(|t| t.successors())
                    .unwrap_or_default();
                let has_successor = successors.iter().any(|&succ| {
                    all_blocks
                        .iter()
                        .position(|&b| b == succ)
                        .map(|idx| idx == *expected_idx)
                        .unwrap_or(false)
                });
                if !has_successor {
                    let actual_indices: Vec<_> = successors
                        .iter()
                        .filter_map(|&s| all_blocks.iter().position(|&b| b == s))
                        .collect();
                    return Err(format!(
                        "Block bb{} does not have successor bb{}. Actual successors: {:?}",
                        block_index, expected_idx, actual_indices
                    ));
                }
            },

            BlockExpectation::SuccessorCount(expected) => {
                let actual = block
                    .terminator
                    .as_ref()
                    .map(|t| t.successors().len())
                    .unwrap_or(0);
                if actual != *expected {
                    return Err(format!(
                        "Block bb{} has {} successor(s), expected {}",
                        block_index, actual, expected
                    ));
                }
            },
        }
        Ok(())
    }
}
