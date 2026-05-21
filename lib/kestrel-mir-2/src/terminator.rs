use kestrel_span::Span;
use smallvec::SmallVec;

use crate::{BlockId, Operand, Place, VariantIdx};

#[derive(Debug, Clone, PartialEq)]
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
pub enum TerminatorKind {
    Return(Operand),
    Jump(BlockId),
    Branch {
        condition: Operand,
        then_block: BlockId,
        else_block: BlockId,
    },
    Switch {
        discriminant: Place,
        cases: Vec<(SwitchCase, BlockId)>,
    },
    Panic(String),
    Unreachable,
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

    pub fn with_span(kind: TerminatorKind, span: Span) -> Self {
        Self {
            kind,
            span: Some(span),
        }
    }

    pub fn ret(operand: Operand) -> Self {
        Self::new(TerminatorKind::Return(operand))
    }

    pub fn jump(target: BlockId) -> Self {
        Self::new(TerminatorKind::Jump(target))
    }

    pub fn branch(condition: Operand, then_block: BlockId, else_block: BlockId) -> Self {
        Self::new(TerminatorKind::Branch {
            condition,
            then_block,
            else_block,
        })
    }

    pub fn switch(discriminant: Place, cases: Vec<(SwitchCase, BlockId)>) -> Self {
        Self::new(TerminatorKind::Switch {
            discriminant,
            cases,
        })
    }

    pub fn panic(message: impl Into<String>) -> Self {
        Self::new(TerminatorKind::Panic(message.into()))
    }

    pub fn unreachable() -> Self {
        Self::new(TerminatorKind::Unreachable)
    }

    pub fn successors(&self) -> SmallVec<[BlockId; 2]> {
        match &self.kind {
            TerminatorKind::Return(_) => SmallVec::new(),
            TerminatorKind::Jump(target) => SmallVec::from_elem(*target, 1),
            TerminatorKind::Branch {
                then_block,
                else_block,
                ..
            } => SmallVec::from_buf([*then_block, *else_block]),
            TerminatorKind::Switch { cases, .. } => {
                cases.iter().map(|(_, block)| *block).collect()
            }
            TerminatorKind::Panic(_) => SmallVec::new(),
            TerminatorKind::Unreachable => SmallVec::new(),
        }
    }

    pub fn successors_mut(&mut self) -> SmallVec<[&mut BlockId; 2]> {
        match &mut self.kind {
            TerminatorKind::Return(_) => SmallVec::new(),
            TerminatorKind::Jump(target) => {
                let mut v = SmallVec::new();
                v.push(target);
                v
            }
            TerminatorKind::Branch {
                then_block,
                else_block,
                ..
            } => SmallVec::from_buf([then_block, else_block]),
            TerminatorKind::Switch { cases, .. } => {
                cases.iter_mut().map(|(_, block)| block).collect()
            }
            TerminatorKind::Panic(_) => SmallVec::new(),
            TerminatorKind::Unreachable => SmallVec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::immediate::Immediate;
    use crate::place::Place;
    use crate::LocalId;

    fn place(n: usize) -> Place {
        Place::local(LocalId::new(n))
    }

    fn op(n: usize) -> Operand {
        Operand::Place(place(n))
    }

    #[test]
    fn return_no_successors() {
        let t = Terminator::ret(op(0));
        assert_eq!(t.successors().len(), 0);
    }

    #[test]
    fn jump_one_successor() {
        let t = Terminator::jump(BlockId::new(3));
        let s = t.successors();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0], BlockId::new(3));
    }

    #[test]
    fn branch_two_successors() {
        let t = Terminator::branch(op(0), BlockId::new(1), BlockId::new(2));
        let s = t.successors();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0], BlockId::new(1));
        assert_eq!(s[1], BlockId::new(2));
    }

    #[test]
    fn switch_n_successors() {
        let t = Terminator::switch(
            place(0),
            vec![
                (SwitchCase::Variant(VariantIdx::new(0)), BlockId::new(1)),
                (SwitchCase::Variant(VariantIdx::new(1)), BlockId::new(2)),
                (SwitchCase::Wildcard, BlockId::new(3)),
            ],
        );
        let s = t.successors();
        assert_eq!(s.len(), 3);
        assert_eq!(s[0], BlockId::new(1));
        assert_eq!(s[1], BlockId::new(2));
        assert_eq!(s[2], BlockId::new(3));
    }

    #[test]
    fn panic_no_successors() {
        let t = Terminator::panic("index out of bounds");
        assert_eq!(t.successors().len(), 0);
    }

    #[test]
    fn unreachable_no_successors() {
        let t = Terminator::unreachable();
        assert_eq!(t.successors().len(), 0);
    }

    #[test]
    fn switch_case_variant_uses_idx() {
        let case = SwitchCase::Variant(VariantIdx::new(2));
        assert_eq!(case, SwitchCase::Variant(VariantIdx::new(2)));
        assert_ne!(case, SwitchCase::Variant(VariantIdx::new(3)));
    }

    #[test]
    fn switch_case_int_range() {
        let case = SwitchCase::IntRange { start: 0, end: 10 };
        assert_eq!(case, SwitchCase::IntRange { start: 0, end: 10 });
    }

    #[test]
    fn switch_case_char_range() {
        let case = SwitchCase::CharRange {
            start: 'a' as u32,
            end: 'z' as u32,
        };
        assert_eq!(
            case,
            SwitchCase::CharRange {
                start: 97,
                end: 122
            }
        );
    }

    #[test]
    fn terminator_no_span() {
        let t = Terminator::jump(BlockId::new(0));
        assert!(t.span.is_none());
    }

    #[test]
    fn ret_unit() {
        let t = Terminator::ret(Operand::Const(Immediate::unit()));
        match &t.kind {
            TerminatorKind::Return(Operand::Const(imm)) => {
                assert_eq!(imm.kind, crate::ImmediateKind::Unit);
            }
            other => panic!("expected Return(unit), got {other:?}"),
        }
    }
}
