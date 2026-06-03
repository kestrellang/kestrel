use crate::inst::Instruction;
use crate::terminator::{Terminator, TerminatorKind};
use crate::value::Ownership;
use crate::{TyId, ValueId};

#[derive(Debug, Clone, PartialEq)]
pub struct BlockParam {
    pub value: ValueId,
    pub ty: TyId,
    pub ownership: Ownership,
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub params: Vec<BlockParam>,
    pub insts: Vec<Instruction>,
    pub terminator: Terminator,
}

impl BasicBlock {
    pub fn new() -> Self {
        Self {
            params: Vec::new(),
            insts: Vec::new(),
            terminator: Terminator::new(TerminatorKind::Unreachable),
        }
    }
}

impl Default for BasicBlock {
    fn default() -> Self {
        Self::new()
    }
}
