//! Basic blocks.

use crate::MirContext;
use crate::function::Terminator;
use crate::id::{Block, Id, Statement};
use crate::metadata::{Metadata, Prior};
use std::fmt;

/// A basic block is a sequence of statements ending in a terminator.
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub meta: Metadata,
    pub priors: Vec<Prior<BasicBlock>>,
    /// Statements in this block.
    pub statements: Vec<Id<Statement>>,
    /// The terminator (how this block exits).
    pub terminator: Option<Terminator>,
}

impl BasicBlock {
    pub fn new() -> Self {
        Self {
            meta: Metadata::new(),
            priors: Vec::new(),
            statements: Vec::new(),
            terminator: None,
        }
    }

    /// Check if this block has a terminator.
    pub fn is_terminated(&self) -> bool {
        self.terminator.is_some()
    }

    /// Get the successor blocks of this block.
    pub fn successors(&self) -> Vec<Id<Block>> {
        self.terminator
            .as_ref()
            .map(|t| t.successors())
            .unwrap_or_default()
    }

    /// Create a display wrapper for printing this block.
    pub fn display<'a>(
        &'a self,
        ctx: &'a MirContext,
        indent: &'a str,
        blocks: &'a [Id<Block>],
    ) -> impl fmt::Display + 'a {
        BasicBlockDisplay {
            block: self,
            ctx,
            indent,
            blocks,
        }
    }
}

impl Default for BasicBlock {
    fn default() -> Self {
        Self::new()
    }
}

struct BasicBlockDisplay<'a> {
    block: &'a BasicBlock,
    ctx: &'a MirContext,
    indent: &'a str,
    blocks: &'a [Id<Block>],
}

impl fmt::Display for BasicBlockDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for stmt_id in &self.block.statements {
            let stmt = &self.ctx.statements[*stmt_id];
            writeln!(f, "{}{}", self.indent, stmt.display(self.ctx))?;
        }

        if let Some(term) = &self.block.terminator {
            writeln!(f, "{}{}", self.indent, term.display(self.ctx, self.blocks))?;
        }

        Ok(())
    }
}
