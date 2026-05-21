use crate::immediate::Immediate;
use crate::place::Place;

#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    Place(Place),
    Const(Immediate),
}

impl Operand {
    pub fn place(p: Place) -> Self {
        Self::Place(p)
    }

    pub fn constant(imm: Immediate) -> Self {
        Self::Const(imm)
    }

    pub fn is_place(&self) -> bool {
        matches!(self, Self::Place(_))
    }

    pub fn is_const(&self) -> bool {
        matches!(self, Self::Const(_))
    }

    pub fn as_place(&self) -> Option<&Place> {
        match self {
            Self::Place(p) => Some(p),
            Self::Const(_) => None,
        }
    }
}

impl From<Place> for Operand {
    fn from(p: Place) -> Self {
        Self::Place(p)
    }
}

impl From<Immediate> for Operand {
    fn from(imm: Immediate) -> Self {
        Self::Const(imm)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UseMode {
    Copy,
    Move,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArgMode {
    Copy,
    Move,
    Ref,
    RefMut,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LocalId;

    #[test]
    fn operand_from_place() {
        let p = Place::local(LocalId::new(0));
        let op: Operand = p.clone().into();
        assert_eq!(op, Operand::Place(p));
        assert!(op.is_place());
        assert!(!op.is_const());
    }

    #[test]
    fn operand_from_immediate() {
        let imm = Immediate::i64(42);
        let op: Operand = imm.clone().into();
        assert_eq!(op, Operand::Const(imm));
        assert!(op.is_const());
        assert!(!op.is_place());
    }

    #[test]
    fn as_place() {
        let p = Place::local(LocalId::new(0));
        let op = Operand::place(p.clone());
        assert_eq!(op.as_place(), Some(&p));

        let op2 = Operand::constant(Immediate::unit());
        assert_eq!(op2.as_place(), None);
    }

    #[test]
    fn use_mode_equality() {
        assert_eq!(UseMode::Copy, UseMode::Copy);
        assert_ne!(UseMode::Copy, UseMode::Move);
    }

    #[test]
    fn arg_mode_equality() {
        assert_eq!(ArgMode::Ref, ArgMode::Ref);
        assert_ne!(ArgMode::Ref, ArgMode::RefMut);
        assert_ne!(ArgMode::Copy, ArgMode::Move);
    }

    #[test]
    fn use_mode_copy_semantics() {
        let a = UseMode::Move;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn arg_mode_copy_semantics() {
        let a = ArgMode::RefMut;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn operand_clone() {
        let op = Operand::place(Place::local(LocalId::new(5)));
        let op2 = op.clone();
        assert_eq!(op, op2);
    }
}
