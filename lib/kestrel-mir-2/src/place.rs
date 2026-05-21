use kestrel_hecs::Entity;
use smallvec::SmallVec;

use crate::{FieldIdx, LocalId, VariantIdx};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PlaceBase {
    Local(LocalId),
    Global(Entity),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaceElem {
    Field(FieldIdx),
    TupleIndex(u32),
    Downcast(VariantIdx),
    Deref,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Place {
    pub base: PlaceBase,
    pub projections: SmallVec<[PlaceElem; 2]>,
}

impl Place {
    pub fn local(id: LocalId) -> Self {
        Self {
            base: PlaceBase::Local(id),
            projections: SmallVec::new(),
        }
    }

    pub fn global(entity: Entity) -> Self {
        Self {
            base: PlaceBase::Global(entity),
            projections: SmallVec::new(),
        }
    }

    pub fn field(mut self, idx: FieldIdx) -> Self {
        self.projections.push(PlaceElem::Field(idx));
        self
    }

    pub fn tuple_index(mut self, i: u32) -> Self {
        self.projections.push(PlaceElem::TupleIndex(i));
        self
    }

    pub fn downcast(mut self, variant: VariantIdx) -> Self {
        self.projections.push(PlaceElem::Downcast(variant));
        self
    }

    pub fn deref(mut self) -> Self {
        self.projections.push(PlaceElem::Deref);
        self
    }

    pub fn root_local(&self) -> Option<LocalId> {
        match self.base {
            PlaceBase::Local(id) => Some(id),
            PlaceBase::Global(_) => None,
        }
    }

    pub fn is_local(&self) -> bool {
        matches!(self.base, PlaceBase::Local(_))
    }

    pub fn as_local(&self) -> Option<LocalId> {
        if self.projections.is_empty() {
            self.root_local()
        } else {
            None
        }
    }

    /// Two places conflict if one is a prefix of the other (including equality).
    pub fn conflicts_with(&self, other: &Place) -> bool {
        self.base == other.base
            && (self.projections.starts_with(&other.projections)
                || other.projections.starts_with(&self.projections))
    }

    /// True if `self` is a prefix of `other` (self's projections are a prefix
    /// of other's projections, same base).
    pub fn is_prefix_of(&self, other: &Place) -> bool {
        self.base == other.base && other.projections.starts_with(&self.projections)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(n: usize) -> LocalId {
        LocalId::new(n)
    }

    fn field(n: usize) -> FieldIdx {
        FieldIdx::new(n)
    }

    fn variant(n: usize) -> VariantIdx {
        VariantIdx::new(n)
    }

    #[test]
    fn local_place_empty_projections() {
        let p = Place::local(local(0));
        assert!(p.projections.is_empty());
        assert_eq!(p.root_local(), Some(local(0)));
    }

    #[test]
    fn global_place() {
        let p = Place::global(Entity::from_raw(1));
        assert!(p.projections.is_empty());
        assert_eq!(p.root_local(), None);
        assert!(!p.is_local());
    }

    #[test]
    fn chaining_field() {
        let p = Place::local(local(0)).field(field(1));
        assert_eq!(p.projections.as_slice(), &[PlaceElem::Field(field(1))]);
        assert_eq!(p.root_local(), Some(local(0)));
    }

    #[test]
    fn chaining_multiple() {
        let p = Place::local(local(0))
            .field(field(0))
            .deref();
        assert_eq!(
            p.projections.as_slice(),
            &[PlaceElem::Field(field(0)), PlaceElem::Deref]
        );
    }

    #[test]
    fn chaining_downcast_then_field() {
        let p = Place::local(local(0))
            .downcast(variant(2))
            .field(field(0));
        assert_eq!(
            p.projections.as_slice(),
            &[
                PlaceElem::Downcast(variant(2)),
                PlaceElem::Field(field(0)),
            ]
        );
    }

    #[test]
    fn tuple_index_projection() {
        let p = Place::local(local(0)).tuple_index(1);
        assert_eq!(
            p.projections.as_slice(),
            &[PlaceElem::TupleIndex(1)]
        );
    }

    #[test]
    fn as_local_bare() {
        let p = Place::local(local(5));
        assert_eq!(p.as_local(), Some(local(5)));
    }

    #[test]
    fn as_local_with_projections_returns_none() {
        let p = Place::local(local(5)).field(field(0));
        assert_eq!(p.as_local(), None);
    }

    #[test]
    fn conflicts_with_self() {
        let p = Place::local(local(0));
        assert!(p.conflicts_with(&p));
    }

    #[test]
    fn conflicts_parent_child() {
        let s = Place::local(local(0));
        let sf = Place::local(local(0)).field(field(0));
        assert!(s.conflicts_with(&sf));
        assert!(sf.conflicts_with(&s));
    }

    #[test]
    fn conflicts_nested() {
        let sf = Place::local(local(0)).field(field(0));
        let sfg = Place::local(local(0)).field(field(0)).field(field(1));
        assert!(sf.conflicts_with(&sfg));
    }

    #[test]
    fn no_conflict_siblings() {
        let sf = Place::local(local(0)).field(field(0));
        let sg = Place::local(local(0)).field(field(1));
        assert!(!sf.conflicts_with(&sg));
    }

    #[test]
    fn no_conflict_different_base() {
        let a = Place::local(local(0)).field(field(0));
        let b = Place::local(local(1)).field(field(0));
        assert!(!a.conflicts_with(&b));
    }

    #[test]
    fn is_prefix_of_equal() {
        let p = Place::local(local(0)).field(field(0));
        assert!(p.is_prefix_of(&p));
    }

    #[test]
    fn is_prefix_parent_of_child() {
        let parent = Place::local(local(0));
        let child = Place::local(local(0)).field(field(0));
        assert!(parent.is_prefix_of(&child));
        assert!(!child.is_prefix_of(&parent));
    }

    #[test]
    fn smallvec_inline_for_two_or_fewer() {
        let p0 = Place::local(local(0));
        assert!(!p0.projections.spilled());

        let p1 = Place::local(local(0)).field(field(0));
        assert!(!p1.projections.spilled());

        let p2 = Place::local(local(0)).field(field(0)).deref();
        assert!(!p2.projections.spilled());

        let p3 = Place::local(local(0))
            .field(field(0))
            .deref()
            .field(field(1));
        assert!(p3.projections.spilled());
    }

    #[test]
    fn place_equality() {
        let a = Place::local(local(0)).field(field(1));
        let b = Place::local(local(0)).field(field(1));
        assert_eq!(a, b);
    }

    #[test]
    fn place_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Place::local(local(0)));
        set.insert(Place::local(local(0)).field(field(0)));
        set.insert(Place::local(local(0)));
        assert_eq!(set.len(), 2);
    }
}
